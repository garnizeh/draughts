# Appendix B — Performance Targets

Targets, not guarantees. They are recorded so that a build which misses them by an order of magnitude is recognizably wrong rather than merely disappointing, and they are replaced by measurements from [§20.9](20-testing-strategy.md#209-performance-regression-baselines) once the implementation exists.

**Every figure is re-derived for the target host in [§2.4](02-scope-and-constraints.md#24-hardware-baseline)** — 14 Broadwell cores at 2.6 GHz, two populated DDR4-2400 channels, an RTX 3050 — rather than carried forward from the 16-core, 128 GB, four-channel host assumed through 1.3. Recording a target this machine cannot reach would defeat the purpose of having targets: a build that hits 900 games/minute should read as *on plan*, not as a 40 % miss against somebody else's hardware.

| Metric | v1.0 baseline | 1.3 target | **1.4 target** | Why it moved |
|---|---|---|---|---|
| Lab games/minute | ~40 (2 workers, 200 iters) | 1 500–2 500 (16 workers) | **600–1 200** (10 workers, 800 iters) | 10 workers not 16, and a 2.6 GHz Broadwell core is not a modern one |
| Transposition hit rate, steady state | — | 0.6–0.8 | **0.6–0.8** | A property of the game tree, not the host. Unchanged |
| Search speedup, table on vs. off (throughput mode) | — | 4–8× | **4–8×** | Unchanged; if anything larger here, since the rollout it replaces is slower too |
| Search speedup, table on vs. off (deterministic mode) | — | 1.5–2.5× | **1.5–2.5×** | Move-gen and terminal reuse only |
| Transposition probe rate, 10 workers | — | — | **Measured, with and without huge pages** | New in 1.4. The probe path is TLB- and DRAM-bound on two channels ([§16.3.1](16-memory-strategy.md#1631-bandwidth-not-just-capacity--new-in-14)) |
| Sustained write throughput | ~5k rows/s | 150k–400k rows/s | **100k–250k rows/s** | One writer thread on a slower core; the 50k-row transaction design is unchanged |
| Commit latency, 50k rows, p99 | — | < 1 500 ms | **< 2 000 ms** | Same transaction against a 4 GB cache instead of 8 |
| Play Mode move latency, p99 | — | < 1 200 ms | **< 1 600 ms** | `time_budget_ms = 1500` is the binding constraint on this host, not the iteration count ([§16.2](16-memory-strategy.md#162-engine-budgets)) |
| Play Mode completed iterations/move | — | — | **Measured, not targeted** | New in 1.4. Distinguishes "slow" from "time-bounded", which is the expected steady state here |
| Commentary latency, CUDA, 64 tokens, p50 | — | — | **0.8–1.5 s** | Qwen2.5-1.5B Q4_K_M on the RTX 3050 ([§7.5.2](07-face-llm-layer.md#752-the-cuda-ladder--the-default-profile)) |
| Commentary latency, CPU, 64 tokens, p50 | — | 1.5–2.0 s (1.5B) | **~1.7 s (0.5B)** | Same latency, smaller model. The 1.3 figure assumed 35 GB/s to two cores; this host has ~15 ([§7.5.1](07-face-llm-layer.md#751-what-the-two-devices-actually-deliver)) |
| Move latency with circuit open, p99 | — | Unchanged from Face-disabled | **Unchanged from Face-disabled** | The point of [§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11) |
| Peak RSS, full-density batch | — | < 96 GB | **< 56 GB** | [§16.1](16-memory-strategy.md#161-memory-budget) — a gate, not a metric |
| Peak VRAM, commentary under lab load | — | — | **< 4.5 GB** | [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) — also a gate |

Two notes on reading this table.

**The lab throughput drop is not a regression.** It is the same design measured against a smaller, older machine. The per-worker figure is roughly unchanged; there are fewer workers and each core is slower.

**Populating the other two memory channels would move several of these rows at once** ([§2.4.1](02-scope-and-constraints.md#241-two-notes-for-whoever-operates-this-host)) — lab throughput, probe rate, and the CPU commentary ladder all share the same bottleneck. It is worth re-measuring rather than re-estimating if that ever happens.

---

← [Appendix A — Memory Budget at a Glance](appendix-a-memory-budget.md) · **[Index](README.md)** · [Appendix C — Migration from v1.0](appendix-c-migration-from-v1-0.md) →
