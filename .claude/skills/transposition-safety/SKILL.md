---
name: transposition-safety
description: The determinism contract for the draughts MCTS engine and its global transposition table — probe, store, eviction, EvaluatorIdentity, TtMode Deterministic vs Throughput, and the differential search test at 1/2/8/10 threads. Use when touching src/engine, tree search, evaluators, Zobrist keys, or anything that caches a search result.
---

# The table must not change what the search returns

**Rule 2 of the project: the transposition table may change how long a search
takes and must never change what it returns** (§20.5, §6.7).

Everything in `src/engine/transposition.rs` follows from that sentence — the
full-board verification on every hit, the evaluator scoping, and the refusal to
serve an impure evaluator's sample mean in deterministic mode. If a change makes
the table faster by making it authoritative, it is wrong however good the
numbers look.

## The shape — §6.7

One table. Global, shared across every worker thread and every concurrently
running lab game. Not per-search, not per-game: self-play revisits positions
constantly, and memory is not the scarce resource — recomputation is.

- Keyed by `TtKey(Zobrist)`, verified against the full board on every hit. A
  Zobrist collision that is not caught is a wrong move played from a cache.
- `TtEntry` is sized against the ~64-byte figure §16.3 budgets the table
  against. `MAX_INLINE_MOVES` exists for that reason; changing it changes the
  memory budget and needs §16.3 redone.
- `SmallMoveList::from_moves` returns `None` when the list does not fit. Cache
  the value **without** a move list. A truncated move list is an illegal-move
  generator.

## The three kinds — §6.7.3

| `TtKind` | What it is | Readable in |
|---|---|---|
| `Terminal` | A proven terminal score | Both modes. Never expires, never averaged |
| `Exact` | Position-pure evaluator output | Both modes |
| `Estimate` | Aggregated sample mean from an impure evaluator | `TtMode::Throughput` only |

`store` refuses to write an `Estimate` outside `Throughput` mode, and `probe`
refuses to serve one. That refusal is the whole of rule 2 in code — do not relax
it to raise a hit rate.

## The two modes — §6.7.5

- **`Deterministic`** — the default for Play Mode. Identical input produces
  identical output, at any thread count, with the table on or off.
- **`Throughput`** — the lab. Sample means are readable; the search is no longer
  bit-reproducible, and that is a stated trade, not an accident.

`MctsConfig` treats a search as deterministic when `max_time_ms == 0` and the
mode is `Deterministic`. A time-limited search is not reproducible by
construction — a search that consults the clock to decide *what* it returns
rather than *when it stops* is a defect.

## `EvaluatorIdentity` — §6.2, §6.8

Two evaluators sharing an identity share a cache. That is the point and it is
also the hazard: an evaluator that changes its output without changing its
identity poisons every entry it has written. Identity is derived from the
evaluator's name and its parameters — if you add a parameter that changes
output, it belongs in the identity.

## The test that has to pass

The differential search test (§20.5): the same search, with the table on and
off, at **1, 2, 8 and 10 threads**, returning identical output. Anything
touching `probe`, `store`, eviction, or `EvaluatorIdentity` needs it green at
all four.

```bash
just test                        # includes the differential test
just test-tt-off                 # §5.4 — the whole search suite, table disabled
just test-one transposition      # one module, with output
```

`just test-tt-off` runs nightly to prove the cache is not load-bearing for
correctness. A suite that passes with the table on and fails with it off has
found rule 2 being violated somewhere, not a flake.

## Zobrist

The key table is generated from a fixed seed and pinned by a fingerprint test.
Regenerating it invalidates every persisted `positions.board_hash` — silently,
because the rows still parse. That is a `format_version` bump; see the
`persisted-format` skill. Do not edit the expected constant.

## Eviction and capacity — §6.7.4

Capacity enforcement is part of `store`. The budget comes from §16.3 and the
host in §2.4. A change to eviction policy is a change to the hit-rate target in
[Appendix B](../../../docs/architecture/appendix-b-performance-targets.md)
(0.6–0.8 steady state) — measure it with `just bench` and record the number
there, not in a comment.
