//! MCTS Engine — §6.
//!
//! The engine is generic over evaluation strategy and must not hard-code
//! rollout logic: random rollout is one implementation of a trait, and a neural
//! value head is another. It must also remain correct with the transposition
//! table disabled — [`TranspositionTable::disabled`] exists so that the claim
//! can be tested rather than asserted.

pub mod evaluator;
pub mod mcts;
pub mod transposition;

pub use evaluator::{EvaluationStrategy, EvaluatorIdentity, RandomRolloutEvaluator};
pub use mcts::{ChildStats, MctsConfig, MctsEngine, SearchResult, ThreadBudget};
pub use transposition::{Probe, TranspositionTable, TtEntry, TtKey, TtKind, TtUpdate};

pub use crate::config::TtMode;
