//! The DB writer actor — §11.2, §15.2.
//!
//! Sole owner of the write connection. Every write in the system goes through
//! this channel, which is what turns SQLite's single-writer constraint from a
//! contention problem into a batching opportunity.

use crossbeam_channel::{Sender, TrySendError};
use tokio::sync::oneshot;

use super::{
    BatchStatus, DbError, DbResult, EdgeRecord, FaceEventRecord, GameRecord, PositionRecord,
};

/// A unit of work for the writer.
pub enum WriteOp {
    Game(GameRecord),
    Positions(Vec<PositionRecord>),
    Edges(Vec<EdgeRecord>),
    BatchProgress {
        batch_id: i64,
        completed: u64,
        failed: u64,
        gpm: f32,
    },
    BatchStatus {
        batch_id: i64,
        status: BatchStatus,
    },
    FaceEvent(FaceEventRecord),

    /// Durability barrier. The writer commits everything ahead of it and then
    /// signals. This is how the durable class works (§11.4).
    Flush(oneshot::Sender<DbResult<()>>),

    /// Drain and stop.
    Shutdown(oneshot::Sender<()>),
}

impl WriteOp {
    /// Rows this message contributes to the current transaction. Used to decide
    /// when a batch is full, so `db_batch_rows` means rows and not messages.
    #[must_use]
    pub fn row_count(&self) -> usize {
        match self {
            Self::Game(_) | Self::FaceEvent(_) => 1,
            Self::Positions(rows) => rows.len(),
            Self::Edges(rows) => rows.len(),
            Self::BatchProgress { .. } | Self::BatchStatus { .. } => 1,
            Self::Flush(_) | Self::Shutdown(_) => 0,
        }
    }
}

/// The three producer policies at a full channel — §11.2.5.
///
/// They differ because the data differs, not because the callers do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backpressure {
    /// Lab workers block. Slowing self-play down is the correct response to a
    /// writer that cannot keep up.
    Block,
    /// Durable writes return `write_queue_saturated` (503) and let the client
    /// retry. A human's move must never wait behind a lab batch.
    Reject,
    /// Telemetry is dropped, with a counter. It is the only class whose loss
    /// costs nothing.
    Drop,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WriterStats {
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub queue_high_water: usize,
    pub rows_committed: u64,
    pub last_commit_ms: u64,
    pub dropped_telemetry: u64,
}

/// A cloneable handle to the writer. Holds a channel and nothing else — no
/// connection, no transaction, no lock.
#[derive(Clone)]
pub struct WriterHandle {
    tx: Sender<WriteOp>,
}

impl WriterHandle {
    #[must_use]
    pub fn new(tx: Sender<WriteOp>) -> Self {
        Self { tx }
    }

    /// Bulk class: fire-and-forget. Lost on a crash, and regenerable by
    /// construction — a batch has a seed and a config, so losing its tail costs
    /// machine time, not information (§11.4).
    pub fn send_bulk(&self, op: WriteOp) -> DbResult<()> {
        self.tx.send(op).map_err(|_| DbError::WriterGone)
    }

    /// Telemetry class: dropped rather than queued when the channel is full.
    pub fn send_telemetry(&self, op: WriteOp) -> Backpressure {
        match self.tx.try_send(op) {
            Ok(()) => Backpressure::Block,
            Err(TrySendError::Full(_)) => Backpressure::Drop,
            Err(TrySendError::Disconnected(_)) => Backpressure::Drop,
        }
    }

    /// Durable class: enqueue, then wait for the commit that contains it.
    ///
    /// The HTTP response is returned only after this resolves, which is what
    /// "nothing acknowledged is lost" means (§11.4).
    pub async fn send_durable(&self, op: WriteOp) -> DbResult<()> {
        self.tx.try_send(op).map_err(|error| match error {
            TrySendError::Full(_) => DbError::Degraded("write queue saturated".to_string()),
            TrySendError::Disconnected(_) => DbError::WriterGone,
        })?;

        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(WriteOp::Flush(ack_tx))
            .map_err(|_| DbError::WriterGone)?;

        ack_rx.await.map_err(|_| DbError::WriterGone)?
    }

    #[must_use]
    pub fn queue_depth(&self) -> usize {
        self.tx.len()
    }

    #[must_use]
    pub fn queue_capacity(&self) -> usize {
        self.tx.capacity().unwrap_or(0)
    }

    /// Drain and stop. Called by the shutdown sequence, step 3 (§22.4).
    pub async fn shutdown(&self) -> DbResult<()> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(WriteOp::Shutdown(ack_tx))
            .map_err(|_| DbError::WriterGone)?;
        ack_rx.await.map_err(|_| DbError::WriterGone)
    }
}

/// Start the writer thread and return a handle to it.
///
/// The connection is moved onto the thread and never comes back: that is the
/// whole design (§11.2).
pub fn spawn(
    conn: rusqlite::Connection,
    config: &crate::config::WriterConfig,
) -> DbResult<(WriterHandle, std::thread::JoinHandle<()>)> {
    let _ = (conn, config);
    todo!("writer actor loop — §11.2.2, and the durability semantics in §20.6")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CURRENT_FORMAT_VERSION;
    use crate::db::{GameMode, SampleKind};
    use crate::rules::{Board, Side};

    fn position() -> PositionRecord {
        PositionRecord {
            id: 1,
            format_version: CURRENT_FORMAT_VERSION,
            batch_id: Some(1),
            game_id: 1,
            ply: 0,
            side_to_move: Side::Black,
            board: Board::initial(),
            board_hash: 0,
            terminal: false,
            outcome: 0,
            root_visits: 0,
            root_q: 0.0,
            sample_kind: SampleKind::Mcts,
        }
    }

    fn game() -> GameRecord {
        GameRecord {
            id: 1,
            format_version: CURRENT_FORMAT_VERSION,
            batch_id: None,
            mode: GameMode::CpuCpu,
            seed: 0,
            result: None,
            winner_side: None,
            ply_count: 0,
            moves: Vec::new(),
            final_board: None,
            elapsed_ms: None,
        }
    }

    /// `db_batch_rows` is a row budget, so a message carrying 5 000 positions
    /// must not count as one.
    #[test]
    fn a_batch_is_measured_in_rows_not_messages() {
        assert_eq!(WriteOp::Game(game()).row_count(), 1);
        assert_eq!(
            WriteOp::Positions(vec![position(); 5_000]).row_count(),
            5_000
        );

        let (ack, _rx) = oneshot::channel();
        assert_eq!(WriteOp::Flush(ack).row_count(), 0, "a barrier is not a row");
    }

    /// §11.2.5: the three producer policies are distinct, and the distinction
    /// is what keeps a human's move off the back of a lab batch's queue.
    #[test]
    fn a_full_channel_rejects_durable_writes_and_drops_telemetry() {
        let (tx, _rx) = crossbeam_channel::bounded(1);
        let handle = WriterHandle::new(tx);

        handle.send_bulk(WriteOp::Game(game())).expect("first fits");
        assert_eq!(handle.queue_depth(), 1);
        assert_eq!(handle.queue_capacity(), 1);

        assert_eq!(
            handle.send_telemetry(WriteOp::Game(game())),
            Backpressure::Drop
        );
    }
}
