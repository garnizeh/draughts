//! The record types that cross the writer channel — §12, §13.
//!
//! Every BLOB-bearing record carries its `format_version` explicitly. §20.8
//! runs a static check over the insert statements asserting that every write
//! path sets it from [`crate::CURRENT_FORMAT_VERSION`] rather than letting the
//! column default do it, because a default is how a version stops meaning
//! anything.

use crate::rules::{Board, GameResult, Move, Side};

/// `games.mode`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameMode {
    HumanCpu,
    CpuCpu,
}

impl GameMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HumanCpu => "human_cpu",
            Self::CpuCpu => "cpu_cpu",
        }
    }
}

/// `lab_batches.status`. `Cancelling` and `Interrupted` are 1.1 additions and
/// are the two that make the lifecycle honest: a cancel is two-phase (§17.6),
/// and a process that died mid-batch did not "fail" (§11.4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BatchStatus {
    Queued,
    Running,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}

impl BatchStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Cancelling => "cancelling",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }

    /// §9.11: only these two states accept a cancel request.
    #[must_use]
    pub fn is_cancellable(self) -> bool {
        matches!(self, Self::Queued | Self::Running)
    }

    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Cancelled | Self::Failed | Self::Interrupted
        )
    }
}

/// `positions.sample_kind`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SampleKind {
    Mcts,
    Terminal,
    /// Sampled from a human match, and therefore belonging to no batch. This is
    /// why `positions.batch_id` is nullable (§13.3).
    HumanGame,
}

impl SampleKind {
    #[must_use]
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Mcts => 0,
            Self::Terminal => 1,
            Self::HumanGame => 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GameRecord {
    /// Pre-assigned from an id allocator lease, so that child rows can be built
    /// before this row has been committed (§15.2).
    pub id: i64,
    pub format_version: u32,
    pub batch_id: Option<i64>,
    pub mode: GameMode,
    pub seed: i64,
    pub result: Option<GameResult>,
    pub winner_side: Option<Side>,
    pub ply_count: u32,
    pub moves: Vec<Move>,
    pub final_board: Option<Board>,
    pub elapsed_ms: Option<i64>,
}

impl GameRecord {
    /// The `games.moves` BLOB: packed little-endian `u16`, one per move.
    #[must_use]
    pub fn encode_moves(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.moves.len() * 2);
        for mv in &self.moves {
            out.extend_from_slice(&mv.to_u16().to_le_bytes());
        }
        out
    }

    /// Decode a `games.moves` BLOB.
    ///
    /// `format_version` is a parameter rather than a field read inside, so that
    /// a caller cannot decode without having looked at it (§13.7).
    pub fn decode_moves(blob: &[u8], format_version: u32) -> Result<Vec<Move>, super::DbError> {
        if format_version != crate::CURRENT_FORMAT_VERSION {
            return Err(super::DbError::UnsupportedFormatVersion {
                found: format_version,
            });
        }

        let (pairs, _remainder) = blob.as_chunks::<2>();
        Ok(pairs
            .iter()
            .map(|pair| Move::from_u16(u16::from_le_bytes(*pair)))
            .collect())
    }
}

#[derive(Clone, Debug)]
pub struct PositionRecord {
    pub id: i64,
    pub format_version: u32,
    pub batch_id: Option<i64>,
    pub game_id: i64,
    pub ply: u32,
    pub side_to_move: Side,
    pub board: Board,
    /// Zobrist, comparable across rows only when `format_version` matches.
    pub board_hash: i64,
    pub terminal: bool,
    /// From the perspective of `side_to_move`: 1 win, 0 draw, -1 loss.
    pub outcome: i8,
    pub root_visits: u32,
    pub root_q: f32,
    pub sample_kind: SampleKind,
}

#[derive(Clone, Copy, Debug)]
pub struct EdgeRecord {
    pub position_id: i64,
    pub mv: Move,
    pub visits: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    pub q_value: f32,
    /// `None` for uniform-prior MCTS; a policy head fills it in later.
    pub prior: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct FaceEventRecord {
    pub match_id: Option<String>,
    pub game_id: Option<i64>,
    pub event_type: String,
    /// `"candle"` or `"canned"`. `"ollama"` is no longer produced, and existing
    /// rows carrying it are left as historical fact (§12.1).
    pub provider: &'static str,
    pub fallback_used: bool,
    pub fallback_reason: Option<&'static str>,
    pub circuit_state: i64,
    pub latency_ms: Option<i64>,
    pub token_count: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CURRENT_FORMAT_VERSION;
    use crate::rules::MoveFlags;

    fn record(moves: Vec<Move>) -> GameRecord {
        GameRecord {
            id: 1,
            format_version: CURRENT_FORMAT_VERSION,
            batch_id: None,
            mode: GameMode::HumanCpu,
            seed: 7,
            result: Some(GameResult::BlackWin),
            winner_side: Some(Side::Black),
            ply_count: moves.len() as u32,
            moves,
            final_board: Some(Board::initial()),
            elapsed_ms: Some(1234),
        }
    }

    #[test]
    fn the_moves_blob_round_trips() {
        let moves: Vec<Move> = (0..40)
            .map(|i| Move {
                from: i % 32,
                to: (i + 5) % 32,
                flags: if i % 3 == 0 {
                    MoveFlags::CAPTURE
                } else {
                    MoveFlags::NONE
                },
            })
            .collect();

        let blob = record(moves.clone()).encode_moves();
        assert_eq!(blob.len(), moves.len() * 2);
        assert_eq!(
            GameRecord::decode_moves(&blob, CURRENT_FORMAT_VERSION).unwrap(),
            moves
        );
    }

    /// §20.8: a row with an unknown `format_version` produces an error — not a
    /// panic, not a default, not a silently skipped row.
    #[test]
    fn an_unknown_format_version_is_refused() {
        let blob = record(Vec::new()).encode_moves();

        let error = GameRecord::decode_moves(&blob, 255).expect_err("unknown version");
        assert!(matches!(
            error,
            super::super::DbError::UnsupportedFormatVersion { found: 255 }
        ));
    }

    #[test]
    fn only_queued_and_running_batches_can_be_cancelled() {
        assert!(BatchStatus::Queued.is_cancellable());
        assert!(BatchStatus::Running.is_cancellable());

        for status in [
            BatchStatus::Cancelling,
            BatchStatus::Completed,
            BatchStatus::Cancelled,
            BatchStatus::Failed,
            BatchStatus::Interrupted,
        ] {
            assert!(!status.is_cancellable(), "{status:?} is not cancellable");
        }
    }
}
