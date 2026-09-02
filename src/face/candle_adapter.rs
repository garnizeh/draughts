//! The in-process Candle inference runtime — §7.4.
//!
//! v1.0 called an external Ollama daemon over HTTP. This loads the model into
//! the application's own memory space instead: one binary, one model file, and
//! a `Result` where there used to be a network.
//!
//! It buys one new risk, recorded rather than hidden: **a bug in the inference
//! path can abort the whole process**, and on CUDA the driver is inside that
//! blast radius (§17.5).
//!
//! Note what this module does *not* do. It does not construct a `Device` — that
//! happens once, in [`super::device::select_device`], and the `Device` arrives
//! here as a parameter (§19.6.5).

use async_trait::async_trait;
use candle_core::Device;

use super::{Commentary, CommentaryContext, FaceAdapter, FaceError};
use crate::config::{FaceConfig, ModelProfile};
use crate::face::device::DeviceKind;

/// Sampling parameters, resolved once from `[face.sampling]`.
#[derive(Clone, Copy, Debug)]
pub struct SamplingParams {
    pub temperature: f64,
    pub top_p: f64,
    pub repeat_penalty: f32,
    pub repeat_last_n: usize,
    /// `0` means "draw a fresh seed per request".
    pub seed: u64,
}

/// Hard budgets, enforced by the runtime rather than trusted to the model.
#[derive(Clone, Copy, Debug)]
pub struct InferenceBudget {
    pub max_tokens: u32,
    pub deadline_ms: u64,
    pub max_queue_depth: usize,
}

/// Which quantized architectures this build can load.
///
/// A GGUF that declares anything else is a startup error naming the
/// architecture, not a panic inside a tensor op.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Architecture {
    Qwen2,
    Llama,
}

impl Architecture {
    pub fn from_gguf_name(name: &str) -> Result<Self, FaceError> {
        match name {
            "qwen2" => Ok(Self::Qwen2),
            "llama" => Ok(Self::Llama),
            other => Err(FaceError::UnsupportedArchitecture {
                architecture: other.to_string(),
            }),
        }
    }
}

/// Owns the loaded weights, the tokenizer, and the KV cache, and serializes
/// every inference request through them. Never constructs a `Device`.
pub struct InferenceRuntime {
    device: Device,
    device_kind: DeviceKind,
    profile: ModelProfile,
    sampling: SamplingParams,
    budget: InferenceBudget,
}

impl InferenceRuntime {
    /// Load the profile's `.gguf` onto an already-resolved device.
    ///
    /// A CUDA out-of-memory here is a [`FaceError::ModelNotLoaded`], which
    /// leaves the breaker permanently open and the service on canned lines. It
    /// is **not** a silent retry on the CPU: a model chosen for VRAM is the
    /// wrong model for two Broadwell cores, and quietly running it there
    /// reproduces exactly the defect §7.4.1 rule 4 exists to prevent.
    pub fn load(
        device: Device,
        device_kind: DeviceKind,
        config: &FaceConfig,
    ) -> Result<Self, FaceError> {
        let _ = (&device, device_kind, config);
        todo!("GGUF load and tokenizer initialisation — §7.4")
    }

    /// The device this runtime was loaded onto. Borrowed, never rebuilt: a
    /// second construction anywhere is the defect §19.6.5 forbids.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    #[must_use]
    pub fn device_kind(&self) -> DeviceKind {
        self.device_kind
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.profile.model_id
    }

    #[must_use]
    pub fn budget(&self) -> InferenceBudget {
        self.budget
    }

    #[must_use]
    pub fn sampling(&self) -> SamplingParams {
        self.sampling
    }

    /// Host RSS attributable to the Face layer, for `/health` (§9.2). Small on
    /// CUDA, because the weights are on the card.
    #[must_use]
    pub fn resident_mb(&self) -> u64 {
        todo!("host-side residency accounting — §16.4")
    }

    /// Device memory in use, or `0` on the CPU path (§9.2).
    #[must_use]
    pub fn vram_used_mb(&self) -> u64 {
        match self.device_kind {
            DeviceKind::Cpu => 0,
            DeviceKind::Cuda { .. } => todo!("CUDA memory accounting — §16.6"),
        }
    }
}

pub struct CandleFaceAdapter {
    runtime: Option<InferenceRuntime>,
}

impl CandleFaceAdapter {
    #[must_use]
    pub fn new(runtime: Option<InferenceRuntime>) -> Self {
        Self { runtime }
    }

    /// A model that never loaded. The circuit opens permanently, `/health`
    /// reports `model_loaded: false`, and a complete game is still playable on
    /// canned lines (§20.3).
    #[must_use]
    pub fn unloaded() -> Self {
        Self { runtime: None }
    }

    #[must_use]
    pub fn runtime(&self) -> Option<&InferenceRuntime> {
        self.runtime.as_ref()
    }

    #[must_use]
    pub fn model_loaded(&self) -> bool {
        self.runtime.is_some()
    }
}

#[async_trait]
impl FaceAdapter for CandleFaceAdapter {
    fn name(&self) -> &'static str {
        "candle"
    }

    async fn generate_commentary(
        &self,
        context: &CommentaryContext,
    ) -> Result<Commentary, FaceError> {
        if self.runtime.is_none() {
            return Err(FaceError::ModelNotLoaded);
        }

        let _ = context;
        todo!(
            "dispatch to the inference thread under a hard token and wall-clock \
             budget, sanitize, and return — §7.4. Never retry internally: the \
             breaker owns that decision (§7.8)."
        )
    }

    fn is_available(&self) -> bool {
        self.runtime.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unloaded_adapter_reports_itself_unavailable() {
        let adapter = CandleFaceAdapter::unloaded();
        assert!(!adapter.is_available());
        assert!(!adapter.model_loaded());
    }

    /// §20.3: with no model present, commentary fails cleanly with an error the
    /// breaker counts — it does not hang, and it does not panic.
    #[tokio::test]
    async fn an_unloaded_adapter_fails_rather_than_hanging() {
        use crate::face::{CommentaryEvent, GameStatusSummary, Tone};
        use crate::rules::Side;

        let adapter = CandleFaceAdapter::unloaded();
        let context = CommentaryContext {
            event: CommentaryEvent::GameStart,
            tone: Tone::Neutral,
            ply: 0,
            side_to_move: Side::Black,
            material_difference: 0,
            game_status: GameStatusSummary::Ongoing,
            max_tokens: 64,
        };

        let error = adapter
            .generate_commentary(&context)
            .await
            .expect_err("no model is loaded");

        assert!(matches!(error, FaceError::ModelNotLoaded));
        assert!(error.counts_toward_trip());
    }

    #[test]
    fn an_unsupported_architecture_names_itself() {
        assert_eq!(
            Architecture::from_gguf_name("qwen2").unwrap(),
            Architecture::Qwen2
        );
        assert_eq!(
            Architecture::from_gguf_name("llama").unwrap(),
            Architecture::Llama
        );

        let error = Architecture::from_gguf_name("mamba").expect_err("unsupported");
        assert!(error.to_string().contains("mamba"));
    }
}
