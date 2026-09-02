//! Zobrist hashing — promoted to a first-class requirement in 1.1 (§5.3).
//!
//! This is the key of a 256M-entry shared cache *and* a persisted column
//! (`positions.board_hash`). Both facts have the same consequence: **the key
//! table must never change between builds without a `format_version` bump.**
//! A build that silently regenerates its keys invalidates every stored hash and
//! every cached entry, and does so without failing anything.
//!
//! The table is therefore derived from a hard-coded seed by a documented
//! generator, and [`TABLE_FINGERPRINT`] pins the result.

use super::board::{Board, SQUARES, Side};

pub type Zobrist = u64;

/// The seed. Changing this constant is a `format_version` bump (§13.7).
const ZOBRIST_SEED: u64 = 0x5852_F42D_4C95_7F2D;

/// Piece planes, in the order they are hashed: black men, white men,
/// black kings, white kings — the same order as the persisted board blob.
const PLANES: usize = 4;

/// `PLANES × SQUARES` piece keys, plus one key folded in when White is to move.
pub struct KeyTable {
    pieces: [[u64; SQUARES as usize]; PLANES],
    side_to_move: u64,
}

impl KeyTable {
    const fn generate() -> Self {
        // SplitMix64: a fixed, tiny, fully specified generator. Deliberately not
        // `rand`, whose output is not a stability guarantee across versions and
        // which would put a persisted encoding at the mercy of a dependency bump.
        const fn next(state: &mut u64) -> u64 {
            *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = *state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        }

        let mut state = ZOBRIST_SEED;
        let mut pieces = [[0u64; SQUARES as usize]; PLANES];

        let mut plane = 0;
        while plane < PLANES {
            let mut square = 0;
            while square < SQUARES as usize {
                pieces[plane][square] = next(&mut state);
                square += 1;
            }
            plane += 1;
        }

        Self {
            pieces,
            side_to_move: next(&mut state),
        }
    }

    #[must_use]
    pub fn piece(&self, plane: usize, square: u32) -> u64 {
        self.pieces[plane][square as usize]
    }

    #[must_use]
    pub fn side_to_move(&self) -> u64 {
        self.side_to_move
    }

    /// An order-independent fingerprint of the whole table.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        // FNV-1a over every key, so that a reordering is caught as well as a
        // regeneration.
        let mut hash: u64 = 0xCBF2_9CE4_8422_2325;
        let mut fold = |value: u64| {
            for byte in value.to_le_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
            }
        };
        for plane in &self.pieces {
            for key in plane {
                fold(*key);
            }
        }
        fold(self.side_to_move);
        hash
    }
}

/// The committed key table. Generated at compile time from the fixed seed.
pub static KEYS: KeyTable = KeyTable::generate();

/// Fingerprint of [`KEYS`], asserted by a test.
///
/// This constant exists so that a change to the generator, the seed, the plane
/// order, or the square count fails a test rather than silently invalidating
/// every persisted `board_hash` in every existing database.
pub const TABLE_FINGERPRINT: u64 = 5_314_808_694_657_829_724;

/// Full recomputation. Correct, and too slow for the search hot path — it is
/// the reference that [`toggle_piece`] is tested against (§20.1).
#[must_use]
pub fn of_position(board: &Board, side_to_move: Side) -> Zobrist {
    let planes = [
        board.black_men,
        board.white_men,
        board.black_kings,
        board.white_kings,
    ];

    let mut hash = 0u64;
    for (plane, mut bits) in planes.into_iter().enumerate() {
        while bits != 0 {
            let square = bits.trailing_zeros();
            hash ^= KEYS.piece(plane, square);
            bits &= bits - 1;
        }
    }

    if side_to_move == Side::White {
        hash ^= KEYS.side_to_move();
    }

    hash
}

/// Add or remove one piece. XOR is its own inverse, so one function does both.
#[must_use]
pub fn toggle_piece(hash: Zobrist, plane: usize, square: u32) -> Zobrist {
    hash ^ KEYS.piece(plane, square)
}

/// Flip the side to move. Folded into the hash, never carried beside it.
#[must_use]
pub fn toggle_side(hash: Zobrist) -> Zobrist {
    hash ^ KEYS.side_to_move()
}

/// Plane index for a piece, matching the persisted board blob's field order.
#[must_use]
pub fn plane_of(side: Side, king: bool) -> usize {
    match (side, king) {
        (Side::Black, false) => 0,
        (Side::White, false) => 1,
        (Side::Black, true) => 2,
        (Side::White, true) => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §20.1: a build that silently regenerates its keys invalidates every
    /// persisted `board_hash`. If this test fails, the fix is a
    /// `format_version` bump — not a new expected value.
    #[test]
    fn the_key_table_is_stable_across_builds() {
        assert_eq!(
            KEYS.fingerprint(),
            TABLE_FINGERPRINT,
            "the Zobrist key table changed; see §13.7 before updating this constant"
        );
    }

    #[test]
    fn every_key_is_distinct() {
        let mut seen = std::collections::HashSet::new();
        for plane in 0..PLANES {
            for square in 0..SQUARES {
                assert!(seen.insert(KEYS.piece(plane, square)), "duplicate key");
            }
        }
        assert!(seen.insert(KEYS.side_to_move()));
    }

    #[test]
    fn side_to_move_is_folded_into_the_hash() {
        let board = Board::initial();
        assert_ne!(
            of_position(&board, Side::Black),
            of_position(&board, Side::White),
            "two positions differing only in side to move must not collide"
        );
    }

    #[test]
    fn toggling_a_piece_twice_is_the_identity() {
        let hash = of_position(&Board::initial(), Side::Black);
        assert_eq!(toggle_piece(toggle_piece(hash, 0, 17), 0, 17), hash);
        assert_eq!(toggle_side(toggle_side(hash)), hash);
    }

    /// The property the incremental path depends on: an incremental update is
    /// indistinguishable from a full recomputation.
    #[test]
    fn incremental_update_agrees_with_full_recomputation() {
        let mut board = Board::initial();
        let mut hash = of_position(&board, Side::Black);

        // Move a black man off square 8 and onto the empty square 13.
        board.black_men &= !(1 << 8);
        hash = toggle_piece(hash, plane_of(Side::Black, false), 8);
        board.black_men |= 1 << 13;
        hash = toggle_piece(hash, plane_of(Side::Black, false), 13);
        hash = toggle_side(hash);

        assert_eq!(hash, of_position(&board, Side::White));
    }
}
