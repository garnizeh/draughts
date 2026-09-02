//! Monte Carlo tree search — §6.4.
//!
//! The engine holds an `Arc<TranspositionTable>` supplied by its owner; it does
//! not create one, and its lifecycle is the process's or the batch's, never the
//! search's.

use std::sync::Arc;

use crate::config::{SearchConfig, TtMode};
use crate::engine::evaluator::EvaluationStrategy;
use crate::engine::transposition::TranspositionTable;
use crate::rules::{GameState, Move};

/// How many threads a single search may use.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThreadBudget {
    /// One thread. Play Mode, where reproducibility is cheap and latency is not.
    Single,
    /// Root-parallel search across `n` threads.
    Threads(usize),
}

impl ThreadBudget {
    #[must_use]
    pub fn from_worker_threads(threads: usize) -> Self {
        if threads <= 1 {
            Self::Single
        } else {
            Self::Threads(threads)
        }
    }

    #[must_use]
    pub fn count(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Threads(n) => n,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MctsConfig {
    /// A ceiling. Where `max_time_ms` is non-zero it usually binds first (§16.2).
    pub max_iterations: u32,
    /// `0` = iteration-bounded, which is the only reproducible setting.
    pub max_time_ms: u64,
    pub exploration_constant: f32,
    pub seed: u64,
    pub thread_budget: ThreadBudget,
    pub tt_mode: TtMode,
}

impl MctsConfig {
    /// Build a search configuration from an `[engine.play]` or `[engine.lab]`
    /// section, so that the two cannot drift apart in interpretation.
    #[must_use]
    pub fn from_search_config(search: &SearchConfig, seed: u64) -> Self {
        Self {
            max_iterations: search.iterations,
            max_time_ms: search.time_budget_ms,
            exploration_constant: search.exploration_constant,
            seed,
            thread_budget: ThreadBudget::from_worker_threads(search.worker_threads),
            tt_mode: search.transposition_mode,
        }
    }

    /// True when two runs of this configuration must produce identical results.
    #[must_use]
    pub fn is_reproducible(&self) -> bool {
        self.max_time_ms == 0 && self.tt_mode == TtMode::Deterministic
    }
}

/// Root statistics for one move, exported for the UI and for training data.
#[derive(Clone, Copy, Debug)]
pub struct ChildStats {
    pub mv: Move,
    pub visits: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub q_value: f32,
    pub prior: f32,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub best_move: Move,
    pub root_visits: u32,
    pub root_q: f32,
    pub child_stats: Vec<ChildStats>,
    pub tt_hits: u32,
    pub tt_misses: u32,
}

pub struct MctsEngine<E: EvaluationStrategy> {
    evaluator: E,
    config: MctsConfig,
    tt: Arc<TranspositionTable>,
}

impl<E: EvaluationStrategy> MctsEngine<E> {
    #[must_use]
    pub fn new(evaluator: E, config: MctsConfig, tt: Arc<TranspositionTable>) -> Self {
        Self {
            evaluator,
            config,
            tt,
        }
    }

    #[must_use]
    pub fn config(&self) -> &MctsConfig {
        &self.config
    }

    #[must_use]
    pub fn evaluator(&self) -> &E {
        &self.evaluator
    }

    #[must_use]
    pub fn transposition_table(&self) -> &Arc<TranspositionTable> {
        &self.tt
    }

    /// Search from `root_state` and return the chosen move plus root statistics.
    ///
    /// Initialise the root, then until the iteration or time budget is
    /// exhausted: select a child using UCB/PUCT; expand if non-terminal,
    /// probing the table for cached legal moves and, when the mode permits, a
    /// cached leaf value; on a miss, evaluate the leaf through the
    /// [`EvaluationStrategy`]; backpropagate; and store subject to `tt_mode`
    /// and evaluator purity. Finally select a move by visits and return the
    /// root statistics that the UI and the training data both read.
    pub fn search(&mut self, root_state: &GameState) -> SearchResult {
        let _ = root_state;
        todo!("tree search — §6.4; the differential test in §20.5 is its safety net")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn play_mode_is_time_bounded_and_lab_mode_is_not() {
        let config = Config::default();

        let play = MctsConfig::from_search_config(&config.engine.play, 0);
        let lab = MctsConfig::from_search_config(&config.engine.lab, 0);

        assert!(play.max_time_ms > 0, "§16.2: a human notices latency");
        assert_eq!(
            lab.max_time_ms, 0,
            "§16.2: a lab batch must be reproducible"
        );
    }

    /// §20.5: throughput mode is *expected* to diverge. A batch that turns out
    /// reproducible there is evidence the table is not actually being shared.
    #[test]
    fn reproducibility_requires_deterministic_mode_and_no_clock() {
        let config = Config::default();

        let lab = MctsConfig::from_search_config(&config.engine.lab, 0);
        assert!(!lab.is_reproducible(), "lab defaults to throughput mode");

        let mut deterministic = lab;
        deterministic.tt_mode = TtMode::Deterministic;
        assert!(deterministic.is_reproducible());

        deterministic.max_time_ms = 1;
        assert!(
            !deterministic.is_reproducible(),
            "a clock is not reproducible"
        );
    }

    #[test]
    fn a_single_worker_thread_is_a_single_thread_budget() {
        assert_eq!(ThreadBudget::from_worker_threads(0), ThreadBudget::Single);
        assert_eq!(ThreadBudget::from_worker_threads(1), ThreadBudget::Single);
        assert_eq!(
            ThreadBudget::from_worker_threads(10),
            ThreadBudget::Threads(10)
        );
    }
}
