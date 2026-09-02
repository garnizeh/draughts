//! The global transposition table — §6.7.
//!
//! One table, shared across every worker thread and every concurrently running
//! lab game. Not per-search, not per-game.
//!
//! **The governing requirement: the table may change how long a search takes
//! and must never change what it returns.** Everything below follows from it —
//! the full-board verification on every hit, the evaluator scoping, and the
//! refusal to serve an impure evaluator's sample mean in deterministic mode.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;

use crate::config::TtMode;
use crate::engine::evaluator::EvaluatorIdentity;
use crate::rules::{Board, GameState, Move, Side, Zobrist};

/// Inline move list stored beside a cached position.
///
/// Sized so that a `TtEntry` stays near the 64-byte figure §16.3 sizes the
/// table against. A position with more legal moves than this caches its value
/// and not its move list.
pub const MAX_INLINE_MOVES: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TtKey(pub Zobrist);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TtKind {
    /// Proven terminal score. Never expires, never averaged.
    Terminal,
    /// Position-pure evaluator output. Safe in every mode.
    Exact,
    /// Aggregated sample mean from an impure evaluator. Readable only in
    /// [`TtMode::Throughput`].
    Estimate,
}

/// A fixed-capacity move list, stored inline in an entry.
#[derive(Clone, Copy, Debug, Default)]
pub struct SmallMoveList {
    moves: [u16; MAX_INLINE_MOVES],
    len: u8,
}

impl SmallMoveList {
    /// Returns `None` when the list does not fit; the caller then caches a
    /// value without a move list rather than a truncated one, because a
    /// truncated move list is an illegal-move generator.
    #[must_use]
    pub fn from_moves(moves: &[Move]) -> Option<Self> {
        if moves.len() > MAX_INLINE_MOVES {
            return None;
        }
        let mut packed = Self::default();
        for (slot, mv) in packed.moves.iter_mut().zip(moves) {
            *slot = mv.to_u16();
        }
        packed.len = moves.len() as u8;
        Some(packed)
    }

    #[must_use]
    pub fn to_moves(self) -> Vec<Move> {
        self.moves[..self.len as usize]
            .iter()
            .map(|packed| Move::from_u16(*packed))
            .collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TtEntry {
    /// The full board, stored so that a hit can be verified. A Zobrist
    /// collision must cost throughput, never correctness.
    pub board: Board,
    pub side_to_move: Side,
    /// In `[-1.0, 1.0]`, from `side_to_move`'s perspective.
    pub value: f32,
    /// Samples aggregated into `value`.
    pub samples: u32,
    pub moves: SmallMoveList,
    pub kind: TtKind,
    pub evaluator: EvaluatorIdentity,
    pub epoch: u16,
}

/// What a caller wants stored after a search visited a node.
#[derive(Clone, Copy, Debug)]
pub struct TtUpdate {
    pub value: f32,
    pub samples: u32,
    pub moves: SmallMoveList,
    pub kind: TtKind,
    pub evaluator: EvaluatorIdentity,
}

#[derive(Debug)]
pub enum Probe {
    Miss,
    /// Legal moves were cached; the value was not usable in this mode.
    Moves(SmallMoveList),
    /// Both the move list and a usable value were cached.
    Full {
        moves: SmallMoveList,
        value: f32,
        samples: u32,
        kind: TtKind,
    },
}

/// Hit/miss accounting. Relaxed atomics: these are diagnostics, not control flow.
#[derive(Debug, Default)]
pub struct TtStats {
    pub probes: u64,
    pub hits: u64,
    pub collisions: u64,
    pub stores: u64,
    pub evictions: u64,
}

impl TtStats {
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        if self.probes == 0 {
            return 0.0;
        }
        self.hits as f64 / self.probes as f64
    }
}

pub struct TranspositionTable {
    map: DashMap<TtKey, TtEntry>,
    capacity: usize,
    epoch: AtomicU64,
    probes: AtomicU64,
    hits: AtomicU64,
    collisions: AtomicU64,
    stores: AtomicU64,
    evictions: AtomicU64,
}

impl TranspositionTable {
    /// Smallest shard count `DashMap` accepts. Startup validation checks the
    /// configured value against this before the table is ever allocated (§23.1).
    pub const MIN_SHARDS: usize = 2;

    /// # Panics
    /// If `shards` is not a power of two greater than one — a `DashMap`
    /// requirement.
    #[must_use]
    pub fn with_capacity(entries: usize, shards: usize) -> Arc<Self> {
        assert!(
            shards.is_power_of_two() && shards >= Self::MIN_SHARDS,
            "shard count must be a power of two of at least {}, got {shards}",
            Self::MIN_SHARDS
        );

        Arc::new(Self {
            map: DashMap::with_capacity_and_shard_amount(entries, shards),
            capacity: entries,
            epoch: AtomicU64::new(0),
            probes: AtomicU64::new(0),
            hits: AtomicU64::new(0),
            collisions: AtomicU64::new(0),
            stores: AtomicU64::new(0),
            evictions: AtomicU64::new(0),
        })
    }

    /// A table that never hits and never stores.
    ///
    /// Used to prove that the cache is a performance optimisation and not
    /// load-bearing for correctness: every test in the rules and search suites
    /// must pass with it, and a nightly job runs the golden-search suite here
    /// (§5.4, §20.5).
    #[must_use]
    pub fn disabled() -> Arc<Self> {
        Self::with_capacity(0, Self::MIN_SHARDS)
    }

    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.capacity == 0
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    #[must_use]
    pub fn stats(&self) -> TtStats {
        TtStats {
            probes: self.probes.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            collisions: self.collisions.load(Ordering::Relaxed),
            stores: self.stores.load(Ordering::Relaxed),
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Retire every entry from previous epochs. O(1) per shard, deferred.
    pub fn advance_epoch(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn epoch(&self) -> u16 {
        self.epoch.load(Ordering::Relaxed) as u16
    }

    pub fn probe(&self, state: &GameState, id: EvaluatorIdentity, mode: TtMode) -> Probe {
        if self.capacity == 0 {
            return Probe::Miss;
        }

        self.probes.fetch_add(1, Ordering::Relaxed);

        let Some(entry) = self.map.get(&TtKey(state.hash)) else {
            return Probe::Miss;
        };

        // Verify the hit against the full board before anything is trusted.
        if entry.board != state.board || entry.side_to_move != state.side_to_move {
            self.collisions.fetch_add(1, Ordering::Relaxed);
            return Probe::Miss;
        }

        // Never serve one evaluator's numbers to another.
        if entry.evaluator != id {
            return Probe::Miss;
        }

        self.hits.fetch_add(1, Ordering::Relaxed);

        let value_usable = match entry.kind {
            TtKind::Terminal | TtKind::Exact => true,
            TtKind::Estimate => mode == TtMode::Throughput,
        };

        if value_usable {
            Probe::Full {
                moves: entry.moves,
                value: entry.value,
                samples: entry.samples,
                kind: entry.kind,
            }
        } else {
            Probe::Moves(entry.moves)
        }
    }

    pub fn store(&self, state: &GameState, update: TtUpdate, mode: TtMode) {
        if self.capacity == 0 {
            return;
        }

        if update.kind == TtKind::Estimate && mode != TtMode::Throughput {
            // Deterministic mode caches move generation and proven scores only.
            // The sample mean of an impure evaluator is deliberately dropped.
            return;
        }

        let _ = (state, update);
        todo!("store, merge and capacity enforcement — §6.7.5")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Move, MoveFlags};

    fn identity() -> EvaluatorIdentity {
        EvaluatorIdentity::new("test", &[])
    }

    /// §5.4: the engine must remain correct with the table disabled, which
    /// starts with a disabled table never hitting and never storing.
    #[test]
    fn a_disabled_table_never_hits() {
        let table = TranspositionTable::disabled();
        let state = GameState::new();

        assert!(table.is_disabled());
        assert!(matches!(
            table.probe(&state, identity(), TtMode::Throughput),
            Probe::Miss
        ));
        assert_eq!(
            table.stats().probes,
            0,
            "a disabled table is not a hot path"
        );
    }

    /// §6.7.4: an `Estimate` is never stored in deterministic mode. The store
    /// path below `todo!()`s, so reaching it would panic — that this does not
    /// panic is the assertion.
    #[test]
    fn deterministic_mode_drops_estimates_before_storing() {
        let table = TranspositionTable::with_capacity(16, 2);
        let state = GameState::new();

        table.store(
            &state,
            TtUpdate {
                value: 0.5,
                samples: 1,
                moves: SmallMoveList::default(),
                kind: TtKind::Estimate,
                evaluator: identity(),
            },
            TtMode::Deterministic,
        );

        assert!(table.is_empty());
    }

    #[test]
    fn a_move_list_that_does_not_fit_is_refused_rather_than_truncated() {
        let mv = Move {
            from: 1,
            to: 2,
            flags: MoveFlags::NONE,
        };

        assert!(SmallMoveList::from_moves(&[mv; MAX_INLINE_MOVES]).is_some());
        assert!(SmallMoveList::from_moves(&[mv; MAX_INLINE_MOVES + 1]).is_none());
    }

    #[test]
    fn an_inline_move_list_round_trips() {
        let moves: Vec<Move> = (0..6)
            .map(|i| Move {
                from: i,
                to: i + 4,
                flags: MoveFlags::CAPTURE,
            })
            .collect();

        let packed = SmallMoveList::from_moves(&moves).expect("fits");
        assert_eq!(packed.len(), moves.len());
        assert_eq!(packed.to_moves(), moves);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn a_shard_count_that_is_not_a_power_of_two_is_rejected() {
        let _ = TranspositionTable::with_capacity(16, 500);
    }

    #[test]
    #[should_panic(expected = "power of two")]
    fn a_single_shard_is_rejected() {
        let _ = TranspositionTable::with_capacity(16, 1);
    }
}
