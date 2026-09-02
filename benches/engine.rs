//! Performance baselines — §20.9.
//!
//! Tracked as committed numbers, not as pass/fail assertions. Expected values
//! are in Appendix B. A run slower than yesterday's is a question, not a
//! failure — except for the two rows Appendix B marks as gates, which are
//! asserted in the load suite rather than here.
//!
//! Only the benchmarks whose subjects exist are registered. The rest are named
//! in §20.9 and arrive with the code they measure.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use draughts::rules::{Board, Side, zobrist};

/// Zobrist hashing was a minor detail in v1.0 and is load-bearing since 1.1: it
/// is the key of a 256M-entry shared cache as well as a persisted column. Full
/// recomputation is the reference the incremental path is measured against, and
/// the gap between them is the reason §5.3 forbids recomputing per node.
fn zobrist_hashing(c: &mut Criterion) {
    let board = Board::initial();

    let mut group = c.benchmark_group("zobrist");

    group.bench_function("full_recomputation", |b| {
        b.iter(|| zobrist::of_position(black_box(&board), black_box(Side::Black)));
    });

    group.bench_function("incremental_toggle", |b| {
        let base = zobrist::of_position(&board, Side::Black);
        b.iter(|| {
            let h = zobrist::toggle_piece(black_box(base), 0, 8);
            let h = zobrist::toggle_piece(h, 0, 13);
            zobrist::toggle_side(h)
        });
    });

    group.finish();
}

/// The 16-byte board encoding is written once per sampled position and read
/// once per exported row, which at §14.2's default density is tens of millions
/// of times per batch.
fn board_encoding(c: &mut Criterion) {
    let board = Board::initial();
    let bytes = board.to_bytes();

    let mut group = c.benchmark_group("board_blob");

    group.bench_function("encode", |b| b.iter(|| black_box(&board).to_bytes()));
    group.bench_function("decode", |b| {
        b.iter(|| Board::from_bytes(black_box(&bytes)))
    });

    group.finish();
}

criterion_group!(benches, zobrist_hashing, board_encoding);
criterion_main!(benches);
