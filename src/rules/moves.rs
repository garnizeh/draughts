//! Moves, move generation, and application.
//!
//! Move generation is where the mandatory-capture rule lives, and it is the one
//! place in the system permitted to decide what is legal. Everything else —
//! search, the API, the lab runner — asks.

use super::board::{GameState, SQUARES, Side};

/// A move, encoded as a `u16` when persisted.
///
/// The `u16` layout is governed by `format_version` (§13.7) and is shared by
/// `games.moves` and `position_edges.move`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Move {
    /// Origin square, 0..32.
    pub from: u8,
    /// Destination square, 0..32. For a multi-jump this is the final landing
    /// square; the intermediate hops are recoverable from the board.
    pub to: u8,
    pub flags: MoveFlags,
}

/// Move properties needed to replay a game from `games.moves` alone.
///
/// Hand-written rather than pulled from `bitflags`: the rules core is required
/// to stay dependency-light (§5.3), and three bits of a persisted encoding
/// should not be at the mercy of a dependency bump (§13.7).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct MoveFlags(u8);

impl MoveFlags {
    pub const NONE: Self = Self(0);
    pub const CAPTURE: Self = Self(0b0000_0001);
    pub const MULTI_JUMP: Self = Self(0b0000_0010);
    pub const PROMOTION: Self = Self(0b0000_0100);

    const ALL: u8 = 0b0000_0111;

    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Bits outside the known set are dropped rather than trusted. A row whose
    /// flags this build does not recognise is a `format_version` question, and
    /// §13.7 requires that question to have been asked before decoding.
    #[must_use]
    pub const fn from_bits_truncate(bits: u8) -> Self {
        Self(bits & Self::ALL)
    }

    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl Move {
    /// Pack into the persisted `u16`: `from` in bits 0–4, `to` in bits 5–9,
    /// flags in bits 10–12. Version 1 of the encoding.
    ///
    /// # Panics
    /// If `from` or `to` is outside `0..32`. Masking an out-of-range square
    /// instead would silently persist a different, wrong square (`32` becomes
    /// `0`), corrupting move replay and transposition move lists — a defect
    /// at the call site that built this `Move`, and one this must not hide.
    #[must_use]
    pub fn to_u16(self) -> u16 {
        assert!(
            u32::from(self.from) < SQUARES && u32::from(self.to) < SQUARES,
            "square out of range: from={}, to={} (valid range is 0..{SQUARES})",
            self.from,
            self.to
        );
        u16::from(self.from)
            | (u16::from(self.to) << 5)
            | (u16::from(self.flags.bits() & 0x07) << 10)
    }

    /// Unpack a persisted move.
    ///
    /// Callers must have dispatched on the owning row's `format_version` before
    /// reaching here; this decodes version 1 and knows nothing about any other.
    #[must_use]
    pub fn from_u16(packed: u16) -> Self {
        Self {
            from: (packed & 0x1F) as u8,
            to: ((packed >> 5) & 0x1F) as u8,
            flags: MoveFlags::from_bits_truncate(((packed >> 10) & 0x07) as u8),
        }
    }

    #[must_use]
    pub fn is_capture(self) -> bool {
        self.flags.contains(MoveFlags::CAPTURE)
    }
}

/// Every legal move in `state`, with mandatory captures already enforced: when
/// a capture exists, the returned list contains captures and nothing else.
///
/// An empty result means the side to move has no move, which is a loss for that
/// side under English draughts rules.
#[must_use]
pub fn generate_legal_moves(state: &GameState) -> Vec<Move> {
    let _ = state;
    todo!("move generation — §5.3; tested by the perft baselines in §20.1")
}

/// Apply a legal move, updating the board, the side to move, the ply, the
/// status, and the Zobrist hash **incrementally**.
///
/// Full recomputation per node is not acceptable at lab throughput (§5.3); the
/// incremental path is checked against [`super::zobrist::of_position`] over a
/// random-play corpus in §20.1.
pub fn apply_move(state: &mut GameState, mv: Move) {
    let _ = (state, mv);
    todo!("move application — §5.3")
}

/// Which side, if either, has no legal continuation.
#[must_use]
pub fn side_to_move_is_stuck(state: &GameState) -> bool {
    generate_legal_moves(state).is_empty()
}

/// The side that wins when `loser` cannot move.
#[must_use]
pub fn winner_when_stuck(loser: Side) -> Side {
    loser.opponent()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packed_move_round_trips_over_every_square_pair() {
        for from in 0..32u8 {
            for to in 0..32u8 {
                let mv = Move {
                    from,
                    to,
                    flags: MoveFlags::CAPTURE.union(MoveFlags::PROMOTION),
                };
                assert_eq!(Move::from_u16(mv.to_u16()), mv);
            }
        }
    }

    /// A caller must not be able to silently persist a truncated, wrong
    /// square: `to_u16` on an out-of-range square must fail loudly, not mask
    /// it into a different legal-looking one.
    #[test]
    #[should_panic(expected = "square out of range")]
    fn encoding_an_out_of_range_square_panics_rather_than_truncating() {
        let mv = Move {
            from: 32,
            to: 0,
            flags: MoveFlags::NONE,
        };
        let _ = mv.to_u16();
    }

    #[test]
    fn flags_survive_the_round_trip_independently() {
        for flags in [
            MoveFlags::NONE,
            MoveFlags::CAPTURE,
            MoveFlags::MULTI_JUMP,
            MoveFlags::PROMOTION,
            MoveFlags::CAPTURE.union(MoveFlags::MULTI_JUMP),
        ] {
            let mv = Move {
                from: 5,
                to: 14,
                flags,
            };
            assert_eq!(Move::from_u16(mv.to_u16()).flags, flags);
        }
    }
}
