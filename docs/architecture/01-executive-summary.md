# 1. Executive Summary

This document defines the architecture for **Draughts** — a streamlined, extensible draughts (checkers) MVP running on a single high-memory machine.

The system supports exactly two execution modes:

1. **Human vs. CPU — Play Mode**
   - A human plays against a Rust-based MCTS engine.
   - The UI is minimal, server-driven, and implemented with HTMX and Alpine.js.
   - Optional LLM commentary/trash-talk is produced by an in-process Candle inference runtime behind a pluggable "Face" layer, guarded by a circuit breaker.

2. **CPU vs. CPU — Training Lab Mode**
   - Headless batch self-play mode.
   - The Rust engine plays many games against itself across a wide worker pool, sharing a single global transposition table.
   - Stores move histories, terminal results, and sampled MCTS node evaluations into SQLite through a dedicated writer actor.
   - Designed for future knowledge extraction, sequence discovery, and neural network training data generation.

The MVP emphasizes:

- A highly extensible Rust core engine.
- A deterministic rules engine isolated from UI and AI commentary.
- A Strategy/Trait-based MCTS evaluation layer so random rollouts can later be replaced by a neural network evaluator without rewriting MCTS.
- A shared, lock-free transposition table that turns spare RAM into search throughput.
- SQLite-only persistence with WAL mode, a 4 GB writer page cache, and a single batching writer actor fed by a large in-memory MPSC queue.
- A minimal frontend with no rich client state.
- An in-process LLM that runs on the GPU where one is present, is never allowed to calculate moves, and can be amputated at runtime by a circuit breaker without affecting gameplay.
- **Single-binary deployment.** One compiled executable, one SQLite file, one `.gguf` model file. No daemons, no sidecars, no service mesh, no container orchestration.

The guiding principle is unchanged from 1.0 — *the engine is authoritative and everything else is optional* — but the operating assumption inverted at 1.1 and was re-grounded at 1.4. v1.0 was written to survive scarcity. v1.1 was written to spend 128 GB deliberately. **v1.4 is written for the machine that actually exists**: 64 GB, 14 Broadwell cores on two memory channels, and an RTX 3050.

That host spends its resources on four things, and the ordering is deliberate:

1. **Search caching** — a 24 GB shared transposition table, still by far the largest single consumer.
2. **Database write batching** — a 2 GB in-RAM queue in front of one very efficient writer.
3. **Commentary on the GPU** — 6 GB of VRAM with roughly ten times the bandwidth two CPU cores can reach, which is what makes commentary meet its deadline at all ([§0.4.3](00-revision-history.md#043-consequence-two--the-cpu-inference-path-cannot-meet-its-own-deadline)).
4. **Cores returned to the engine** — because inference no longer competes for them.

The GPU is used where it helps and required nowhere. A build with no CUDA feature, on a machine with no driver, plays exactly the same draughts.

---

← [0. Revision History](00-revision-history.md) · **[Index](README.md)** · [2. Scope and Constraints](02-scope-and-constraints.md) →
