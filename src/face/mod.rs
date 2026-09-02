//! Face / LLM layer — §7.
//!
//! The model runs in this process, behind a circuit breaker, and it never
//! plays. It cannot choose, validate, or influence a move: the constraint is
//! enforced by types, not by convention — [`CommentaryContext`] is the whole of
//! what the layer is ever given, and there is no move in it (§2.3, §5.7).
//!
//! The guiding principle for everything here: **the engine is authoritative and
//! this layer is optional.** A game played entirely on canned lines, with no
//! model file present, is a fully valid game.

pub mod breaker;
pub mod candle_adapter;
pub mod canned;
pub mod device;
pub mod prompt;
pub mod sanitize;

use std::sync::Arc;

use async_trait::async_trait;

pub use breaker::{Admission, CircuitBreaker, CircuitState, MonotonicClock, SystemClock};
pub use canned::CannedFaceAdapter;
pub use device::{DeviceKind, DeviceRequest, select_device};

use crate::rules::Side;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CommentaryEvent {
    GameStart,
    HumanMove,
    CpuMove,
    Capture,
    MultiCapture,
    Promotion,
    Win,
    Loss,
    Draw,
    IdleTaunt,
}

impl CommentaryEvent {
    /// Every variant, so that §20.7's canned-coverage test cannot go stale when
    /// one is added.
    pub const ALL: [Self; 10] = [
        Self::GameStart,
        Self::HumanMove,
        Self::CpuMove,
        Self::Capture,
        Self::MultiCapture,
        Self::Promotion,
        Self::Win,
        Self::Loss,
        Self::Draw,
        Self::IdleTaunt,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Tone {
    Neutral,
    Playful,
    Sarcastic,
    Competitive,
}

impl Tone {
    pub const ALL: [Self; 4] = [
        Self::Neutral,
        Self::Playful,
        Self::Sarcastic,
        Self::Competitive,
    ];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatusSummary {
    Ongoing,
    HumanWon,
    CpuWon,
    Drawn,
}

/// Everything the Face layer is permitted to know.
///
/// Note what is absent, and note that its absence is structural: there is no
/// move, no square index, no legal-move list, no search statistic, and no
/// handle to anything that could produce one (§5.7).
#[derive(Clone, Copy, Debug)]
pub struct CommentaryContext {
    pub event: CommentaryEvent,
    pub tone: Tone,
    pub ply: u32,
    pub side_to_move: Side,
    pub material_difference: i32,
    pub game_status: GameStatusSummary,
    pub max_tokens: u32,
}

/// A generated (or canned) line, plus how it was produced.
#[derive(Clone, Debug)]
pub struct Commentary {
    pub text: String,
    pub provider: &'static str,
    pub fallback_used: bool,
    /// `None` when the primary adapter produced the line.
    pub fallback_reason: Option<FallbackReason>,
    pub circuit_state: CircuitState,
    pub latency_ms: u64,
    pub token_count: u32,
}

/// The `face_events.fallback_reason` values (§12).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FallbackReason {
    CircuitOpen,
    Timeout,
    InferenceError,
    Saturated,
    ModelNotLoaded,
    EmptyOutput,
    Disabled,
    RateLimited,
}

impl FallbackReason {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CircuitOpen => "circuit_open",
            Self::Timeout => "timeout",
            Self::InferenceError => "inference_error",
            Self::Saturated => "saturated",
            Self::ModelNotLoaded => "model_not_loaded",
            Self::EmptyOutput => "empty_output",
            Self::Disabled => "disabled",
            Self::RateLimited => "rate_limited",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FaceError {
    #[error("commentary deadline exceeded")]
    Timeout,

    #[error("inference failed: {0}")]
    Inference(String),

    #[error("no model is loaded")]
    ModelNotLoaded,

    #[error("the commentary queue is full")]
    Saturated,

    #[error("the Face layer is disabled in configuration")]
    Disabled,

    #[error("the model produced nothing usable after sanitization")]
    EmptyOutput,

    #[error("the GGUF file declares no model architecture")]
    ModelArchitectureMissing,

    #[error("unsupported model architecture: {architecture}")]
    UnsupportedArchitecture { architecture: String },

    #[error("tokenizer error: {0}")]
    Tokenizer(String),
}

impl FaceError {
    /// §7.8.3. Getting this wrong in either direction is a real bug: a breaker
    /// that counts saturation trips constantly under lab load, and one that
    /// ignores timeouts never trips at all.
    #[must_use]
    pub fn counts_toward_trip(&self) -> bool {
        match self {
            Self::Timeout
            | Self::Inference(_)
            | Self::ModelNotLoaded
            | Self::EmptyOutput
            | Self::ModelArchitectureMissing
            | Self::UnsupportedArchitecture { .. }
            | Self::Tokenizer(_) => true,

            // Expected backpressure under lab load; the model is healthy.
            Self::Saturated => false,
            // Not a failure; the breaker is not consulted at all.
            Self::Disabled => false,
        }
    }

    #[must_use]
    pub fn fallback_reason(&self) -> FallbackReason {
        match self {
            Self::Timeout => FallbackReason::Timeout,
            Self::Saturated => FallbackReason::Saturated,
            Self::Disabled => FallbackReason::Disabled,
            Self::EmptyOutput => FallbackReason::EmptyOutput,
            Self::ModelNotLoaded
            | Self::ModelArchitectureMissing
            | Self::UnsupportedArchitecture { .. } => FallbackReason::ModelNotLoaded,
            Self::Inference(_) | Self::Tokenizer(_) => FallbackReason::InferenceError,
        }
    }
}

#[async_trait]
pub trait FaceAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    async fn generate_commentary(
        &self,
        context: &CommentaryContext,
    ) -> Result<Commentary, FaceError>;

    fn is_available(&self) -> bool;
}

/// The facade every caller sees. `CommentaryService` talks to this and to
/// nothing below it.
pub struct Face {
    primary: Arc<dyn FaceAdapter>,
    /// Infallible by construction, which is what makes commentary optional
    /// rather than merely unreliable.
    fallback: Arc<CannedFaceAdapter>,
    breaker: Arc<CircuitBreaker>,
    /// Injectable, so §20.7 can test the cooldown without sleeping for it.
    clock: Arc<dyn MonotonicClock>,
}

impl Face {
    #[must_use]
    pub fn new(
        primary: Arc<dyn FaceAdapter>,
        fallback: Arc<CannedFaceAdapter>,
        breaker: Arc<CircuitBreaker>,
        clock: Arc<dyn MonotonicClock>,
    ) -> Self {
        Self {
            primary,
            fallback,
            breaker,
            clock,
        }
    }

    #[must_use]
    pub fn breaker(&self) -> &Arc<CircuitBreaker> {
        &self.breaker
    }

    /// Never fails, and never blocks a move.
    ///
    /// A short-circuited request costs one atomic load and returns a canned
    /// line; that is the steady state the UI must render without treating it as
    /// an error (§5.1).
    pub async fn commentary(&self, context: &CommentaryContext) -> Commentary {
        let now_ms = self.clock.now_ms();

        if self.breaker.admit(now_ms) == Admission::ShortCircuit {
            return self.fallback.canned(
                context,
                FallbackReason::CircuitOpen,
                self.breaker.state(),
            );
        }

        match self.primary.generate_commentary(context).await {
            Ok(commentary) => {
                self.breaker.on_success();
                commentary
            }
            Err(error) => {
                let reason = error.fallback_reason();
                self.breaker.on_failure(self.clock.now_ms(), &error);
                tracing::debug!(%error, "commentary fell back");
                self.fallback.canned(context, reason, self.breaker.state())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §7.8.3, as a table. The two `false` rows are the ones that matter: a
    /// breaker that trips on backpressure protects nothing and breaks
    /// commentary under exactly the load it was built for.
    #[test]
    fn only_real_faults_count_toward_the_trip_threshold() {
        assert!(FaceError::Timeout.counts_toward_trip());
        assert!(FaceError::Inference("boom".into()).counts_toward_trip());
        assert!(FaceError::ModelNotLoaded.counts_toward_trip());
        assert!(FaceError::EmptyOutput.counts_toward_trip());

        assert!(!FaceError::Saturated.counts_toward_trip());
        assert!(!FaceError::Disabled.counts_toward_trip());
    }

    #[test]
    fn every_error_maps_to_a_persisted_fallback_reason() {
        let cases = [
            (FaceError::Timeout, "timeout"),
            (FaceError::Saturated, "saturated"),
            (FaceError::Disabled, "disabled"),
            (FaceError::EmptyOutput, "empty_output"),
            (FaceError::ModelNotLoaded, "model_not_loaded"),
            (FaceError::Inference("x".into()), "inference_error"),
        ];

        for (error, expected) in cases {
            assert_eq!(error.fallback_reason().as_str(), expected);
        }
    }
}
