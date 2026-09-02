//! Batch execution and cancellation — §5.5, §17.6.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::config::Config;
use crate::db::{BatchStatus, WriterHandle};
use crate::engine::TranspositionTable;

/// What `POST /api/v1/lab/batches` asks for (§9.8).
#[derive(Clone, Debug)]
pub struct BatchRequest {
    pub name: String,
    pub target_games: u64,
    pub seed: Option<u64>,
    /// When true, the batch must produce byte-identical `games.moves` BLOBs on
    /// a re-run at any thread count — which requires deterministic mode and an
    /// iteration-bounded search (§20.5).
    pub reproducible: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BatchProgress {
    pub completed_games: u64,
    pub failed_games: u64,
    pub games_per_minute: f32,
}

/// A running batch, and the only way to stop one.
///
/// Cancellation is two-phase (§17.6): the flag moves the batch to `cancelling`,
/// workers finish the game in hand, and only once the drain completes does the
/// batch become `cancelled`. Every game counted in `completed_games` is present
/// in `games` by then — §20.3 asserts exactly that.
pub struct BatchHandle {
    pub batch_id: i64,
    cancel: Arc<AtomicBool>,
    completed: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
}

impl BatchHandle {
    #[must_use]
    pub fn new(batch_id: i64) -> Self {
        Self {
            batch_id,
            cancel: Arc::new(AtomicBool::new(false)),
            completed: Arc::new(AtomicU64::new(0)),
            failed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Request cancellation. Returns immediately; the batch is not cancelled
    /// until its workers reach a game boundary.
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelling(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }

    #[must_use]
    pub fn progress(&self) -> BatchProgress {
        BatchProgress {
            completed_games: self.completed.load(Ordering::Relaxed),
            failed_games: self.failed.load(Ordering::Relaxed),
            games_per_minute: 0.0,
        }
    }

    /// The status a batch in this state should report (§9.10).
    #[must_use]
    pub fn status(&self, finished: bool) -> BatchStatus {
        match (self.is_cancelling(), finished) {
            (true, false) => BatchStatus::Cancelling,
            (true, true) => BatchStatus::Cancelled,
            (false, true) => BatchStatus::Completed,
            (false, false) => BatchStatus::Running,
        }
    }
}

/// Owns the worker pool and the batch lifecycle.
///
/// It holds `Arc` handles to the transposition table and the writer; it owns
/// neither.
pub struct LabRunner {
    config: Arc<Config>,
    tt: Arc<TranspositionTable>,
    writer: WriterHandle,
}

impl LabRunner {
    #[must_use]
    pub fn new(config: Arc<Config>, tt: Arc<TranspositionTable>, writer: WriterHandle) -> Self {
        Self { config, tt, writer }
    }

    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.config.engine.lab.worker_threads
    }

    #[must_use]
    pub fn transposition_table(&self) -> &Arc<TranspositionTable> {
        &self.tt
    }

    #[must_use]
    pub fn writer(&self) -> &WriterHandle {
        &self.writer
    }

    /// Start a batch across the worker pool.
    ///
    /// Each worker owns one `GameState`, one MCTS arena reused across games,
    /// one deterministic RNG seeded from `(batch_seed, game_index)`, and one id
    /// allocator lease (§5.5).
    pub fn start(&self, request: BatchRequest) -> BatchHandle {
        let _ = request;
        todo!("worker pool and batch lifecycle — §5.5, §15.3")
    }

    /// Mark batches left `running` by a crashed process as `interrupted`, and
    /// recompute their `completed_games` from the row count.
    ///
    /// The recorded count is advisory; the row count is authoritative (§11.4).
    /// Runs at startup, step 4 (§22.3).
    pub fn recover_interrupted_batches(&self) -> crate::db::DbResult<usize> {
        todo!("interrupted-batch recovery — §11.4")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §17.6: `running → cancelling → cancelled`. The middle state exists so
    /// that a cancelled batch's row count and its `completed_games` agree.
    #[test]
    fn cancellation_is_two_phase() {
        let handle = BatchHandle::new(1);

        assert_eq!(handle.status(false), BatchStatus::Running);

        handle.request_cancel();
        assert_eq!(handle.status(false), BatchStatus::Cancelling);
        assert_eq!(handle.status(true), BatchStatus::Cancelled);
    }

    #[test]
    fn a_batch_that_finishes_uncancelled_is_completed() {
        let handle = BatchHandle::new(1);
        assert_eq!(handle.status(true), BatchStatus::Completed);
    }

    #[test]
    fn cancelling_twice_is_harmless() {
        let handle = BatchHandle::new(1);
        handle.request_cancel();
        handle.request_cancel();
        assert!(handle.is_cancelling());
    }
}
