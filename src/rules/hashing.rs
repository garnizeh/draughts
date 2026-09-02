//! Shared FNV-1a 64-bit folding.
//!
//! Used by the Zobrist key-table fingerprint ([`crate::rules::zobrist`]) and
//! by `EvaluatorIdentity` ([`crate::engine::evaluator`]) — two hashes with
//! different jobs but the same algorithm, kept in one place so a change to
//! one cannot silently drift from the other.

/// The standard FNV-1a 64-bit offset basis.
pub const FNV1A_OFFSET_BASIS: u64 = 0xCBF2_9CE4_8422_2325;

/// The standard FNV-1a 64-bit prime.
const FNV1A_PRIME: u64 = 0x0000_0100_0000_01B3;

/// Fold `bytes` into `hash` with FNV-1a.
#[must_use]
pub fn fnv1a_fold(hash: u64, bytes: &[u8]) -> u64 {
    let mut hash = hash;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV1A_PRIME);
    }
    hash
}
