//! Board representation — 8×8 English draughts on 32 playable squares.

use super::moves::Move;
use super::zobrist::{self, Zobrist};

/// Playable dark squares: 32 of them, numbered 0 to 31 (`0..SQUARES` as a
/// half-open range) from Black's back rank.
pub const SQUARES: u32 = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Side {
    Black,
    White,
}

impl Side {
    #[must_use]
    pub fn opponent(self) -> Self {
        match self {
            Self::Black => Self::White,
            Self::White => Self::Black,
        }
    }

    /// The `side_to_move` encoding persisted in `positions` (§12).
    #[must_use]
    pub fn as_i64(self) -> i64 {
        match self {
            Self::Black => 0,
            Self::White => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameResult {
    BlackWin,
    WhiteWin,
    Draw,
}

impl GameResult {
    /// The `games.result` encoding persisted in §12: 1 black, 2 white, 3 draw.
    #[must_use]
    pub fn as_i64(self) -> i64 {
        match self {
            Self::BlackWin => 1,
            Self::WhiteWin => 2,
            Self::Draw => 3,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    Ongoing,
    Finished(GameResult),
}

/// Four bitboards over the 32 playable squares.
///
/// Persisted as exactly 16 bytes, little-endian, in this field order. The
/// encoding is governed by `format_version` (§13.7): changing it — including
/// changing the square numbering — is a version bump, not a refactor.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct Board {
    pub black_men: u32,
    pub white_men: u32,
    pub black_kings: u32,
    pub white_kings: u32,
}

impl Board {
    /// The standard opening position: twelve men a side on the first three rows.
    #[must_use]
    pub fn initial() -> Self {
        Self {
            black_men: 0x0000_0FFF,
            white_men: 0xFFF0_0000,
            black_kings: 0,
            white_kings: 0,
        }
    }

    #[must_use]
    pub fn occupied(&self) -> u32 {
        self.black_men | self.white_men | self.black_kings | self.white_kings
    }

    #[must_use]
    pub fn pieces_of(&self, side: Side) -> u32 {
        match side {
            Side::Black => self.black_men | self.black_kings,
            Side::White => self.white_men | self.white_kings,
        }
    }

    /// Men count 1, kings count 2 — the figure the Face layer is allowed to see
    /// (§5.7), and nothing else about the position.
    #[must_use]
    pub fn material_difference(&self, perspective: Side) -> i32 {
        let score = |men: u32, kings: u32| men.count_ones() as i32 + 2 * kings.count_ones() as i32;
        let black = score(self.black_men, self.black_kings);
        let white = score(self.white_men, self.white_kings);
        match perspective {
            Side::Black => black - white,
            Side::White => white - black,
        }
    }

    /// The 16-byte `positions.board` / `games.final_board` encoding (§12).
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 16] {
        let mut out = [0u8; 16];
        out[0..4].copy_from_slice(&self.black_men.to_le_bytes());
        out[4..8].copy_from_slice(&self.white_men.to_le_bytes());
        out[8..12].copy_from_slice(&self.black_kings.to_le_bytes());
        out[12..16].copy_from_slice(&self.white_kings.to_le_bytes());
        out
    }

    /// Decode a persisted board.
    ///
    /// `format_version` is a parameter rather than a field read inside, so
    /// that a caller cannot decode without having looked at it (§13.7) — the
    /// same discipline `GameRecord::decode_moves` (`src/db/records.rs`)
    /// applies to the sibling `games.moves` column. `None` for any version
    /// this decoder does not know; it knows only version 1.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 16], format_version: u32) -> Option<Self> {
        if format_version != crate::CURRENT_FORMAT_VERSION {
            return None;
        }
        let word = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        Some(Self {
            black_men: word(0),
            white_men: word(4),
            black_kings: word(8),
            white_kings: word(12),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GameState {
    pub board: Board,
    pub side_to_move: Side,
    pub status: GameStatus,
    pub ply: u32,
    /// Incrementally maintained across `apply_move`; never recomputed per node.
    pub hash: Zobrist,
    pub history: Vec<Move>,
}

impl GameState {
    #[must_use]
    pub fn new() -> Self {
        let board = Board::initial();
        let side_to_move = Side::Black;
        Self {
            hash: zobrist::of_position(&board, side_to_move),
            board,
            side_to_move,
            status: GameStatus::Ongoing,
            ply: 0,
            history: Vec::new(),
        }
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, GameStatus::Finished(_))
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_opening_position_has_twelve_men_a_side() {
        let board = Board::initial();
        assert_eq!(board.black_men.count_ones(), 12);
        assert_eq!(board.white_men.count_ones(), 12);
        assert_eq!(board.black_kings, 0);
        assert_eq!(board.white_kings, 0);
        assert_eq!(board.material_difference(Side::Black), 0);
    }

    #[test]
    fn the_board_blob_round_trips() {
        let board = Board {
            black_men: 0x0123_4567,
            white_men: 0x89AB_CDEF,
            black_kings: 0x0000_00FF,
            white_kings: 0xFF00_0000,
        };
        assert_eq!(
            Board::from_bytes(&board.to_bytes(), crate::CURRENT_FORMAT_VERSION),
            Some(board)
        );
    }

    #[test]
    fn an_unrecognized_format_version_does_not_decode() {
        let board = Board::initial();
        assert_eq!(Board::from_bytes(&board.to_bytes(), 999), None);
    }

    #[test]
    fn the_board_blob_is_exactly_sixteen_bytes() {
        assert_eq!(Board::initial().to_bytes().len(), 16);
    }
}
