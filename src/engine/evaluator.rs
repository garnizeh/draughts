//! The evaluation strategy trait — §6.2.
//!
//! This is the seam that lets a neural value head replace random rollouts
//! without touching tree search.

use std::fmt;

use crate::rules::{GameResult, GameState, GameStatus, Move, Side};

/// Identity of an evaluator *including its configuration*.
///
/// Transposition entries are only shared between evaluators with an identical
/// identity. Changing a rollout depth, a network checkpoint, or a value
/// normalisation must change this value — otherwise one evaluator is served
/// another's numbers, which is a cache turning into a source of wrong answers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EvaluatorIdentity(u64);

impl EvaluatorIdentity {
    /// Build an identity from a name and its numeric parameters.
    #[must_use]
    pub fn new(name: &str, params: &[(&str, u64)]) -> Self {
        let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
        let mut fold_bytes = |bytes: &[u8]| {
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };

        fold_bytes(name.as_bytes());
        for (key, value) in params {
            fold_bytes(key.as_bytes());
            fold_bytes(&value.to_le_bytes());
        }

        Self(hash)
    }

    /// Build an identity from a name and an opaque discriminator, such as a
    /// model checkpoint id.
    #[must_use]
    pub fn from_str(name: &str, discriminator: &str) -> Self {
        let mut identity = Self::new(name, &[]);
        for byte in discriminator.as_bytes() {
            identity.0 ^= u64::from(*byte);
            identity.0 = identity.0.wrapping_mul(0x0000_0100_0000_01B3);
        }
        identity
    }

    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for EvaluatorIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

pub trait EvaluationStrategy: Send {
    /// Name of the evaluator, e.g. `"random_rollout"` or `"nn_value_v1"`.
    fn name(&self) -> &'static str;

    /// See [`EvaluatorIdentity`]. Two evaluators sharing an identity share a
    /// cache; two that should not must not.
    fn identity(&self) -> EvaluatorIdentity;

    /// True when [`Self::estimate_leaf_value`] is a pure function of `state`.
    ///
    /// Random rollout returns `false`: its result depends on RNG draws. A
    /// neural value head returns `true`. The engine uses this to decide whether
    /// an estimate may be cached in deterministic mode (§6.7.4).
    ///
    /// **An evaluator that claims purity and is not is the single most
    /// dangerous bug the transposition table can host.** §20.2 tests it.
    fn is_position_pure(&self) -> bool;

    /// Exact terminal score from `perspective`, normalised to `[-1.0, 1.0]`:
    /// `1.0` win, `0.0` draw, `-1.0` loss. `None` for a non-terminal state.
    fn terminal_score(&self, state: &GameState, perspective: Side) -> Option<f32>;

    /// Evaluate a non-terminal leaf from `perspective`.
    fn estimate_leaf_value(&mut self, state: &GameState, perspective: Side) -> f32;

    /// Optional prior probabilities over `legal_moves`. Uniform by default.
    fn move_priors(&mut self, state: &GameState, legal_moves: &[Move]) -> Vec<f32> {
        let _ = state;
        if legal_moves.is_empty() {
            return Vec::new();
        }
        vec![1.0 / legal_moves.len() as f32; legal_moves.len()]
    }
}

/// The normalisation every evaluator must agree on.
///
/// §20.2 asserts `score(state, Black) == -score(state, White)` for every
/// non-draw terminal fixture; this function is why that holds.
#[must_use]
pub fn score_result(result: GameResult, perspective: Side) -> f32 {
    match (result, perspective) {
        (GameResult::Draw, _) => 0.0,
        (GameResult::BlackWin, Side::Black) | (GameResult::WhiteWin, Side::White) => 1.0,
        (GameResult::BlackWin, Side::White) | (GameResult::WhiteWin, Side::Black) => -1.0,
    }
}

/// The MVP evaluator: play the position out at random and score the result.
pub struct RandomRolloutEvaluator {
    rng_seed: u64,
    max_playout_ply: u32,
}

impl RandomRolloutEvaluator {
    #[must_use]
    pub fn new(rng_seed: u64, max_playout_ply: u32) -> Self {
        Self {
            rng_seed,
            max_playout_ply,
        }
    }

    #[must_use]
    pub fn seed(&self) -> u64 {
        self.rng_seed
    }
}

impl EvaluationStrategy for RandomRolloutEvaluator {
    fn name(&self) -> &'static str {
        "random_rollout"
    }

    fn identity(&self) -> EvaluatorIdentity {
        // The seed is deliberately *not* part of the identity: two workers with
        // different seeds are the same evaluator sampling the same distribution,
        // and their estimates are poolable. The playout cap is part of it,
        // because it changes the distribution.
        EvaluatorIdentity::new(
            "random_rollout",
            &[("max_playout_ply", u64::from(self.max_playout_ply))],
        )
    }

    fn is_position_pure(&self) -> bool {
        // A rollout samples a trajectory; two calls on the same state
        // legitimately disagree. Never cacheable in deterministic mode.
        false
    }

    fn terminal_score(&self, state: &GameState, perspective: Side) -> Option<f32> {
        match state.status {
            GameStatus::Finished(result) => Some(score_result(result, perspective)),
            GameStatus::Ongoing => None,
        }
    }

    fn estimate_leaf_value(&mut self, state: &GameState, perspective: Side) -> f32 {
        let _ = (state, perspective);
        todo!("random rollout playout — §6.3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_scores_are_antisymmetric() {
        for result in [GameResult::BlackWin, GameResult::WhiteWin] {
            assert_eq!(
                score_result(result, Side::Black),
                -score_result(result, Side::White)
            );
        }
        assert_eq!(score_result(GameResult::Draw, Side::Black), 0.0);
        assert_eq!(score_result(GameResult::Draw, Side::White), 0.0);
    }

    #[test]
    fn identity_changes_with_configuration_but_not_with_seed() {
        let a = RandomRolloutEvaluator::new(1, 200);
        let b = RandomRolloutEvaluator::new(999, 200);
        let c = RandomRolloutEvaluator::new(1, 400);

        assert_eq!(
            a.identity(),
            b.identity(),
            "the seed is not part of identity"
        );
        assert_ne!(a.identity(), c.identity(), "the playout cap is");
    }

    #[test]
    fn different_evaluators_never_share_an_identity() {
        assert_ne!(
            EvaluatorIdentity::new("random_rollout", &[]),
            EvaluatorIdentity::new("nn_value_v1", &[])
        );
        assert_ne!(
            EvaluatorIdentity::from_str("nn_value_v1", "ckpt-a"),
            EvaluatorIdentity::from_str("nn_value_v1", "ckpt-b")
        );
    }

    #[test]
    fn random_rollout_never_claims_purity() {
        assert!(!RandomRolloutEvaluator::new(0, 200).is_position_pure());
    }
}
