# 25. MVP Acceptance Criteria

The MVP is complete when:

1. A human can start a match against the CPU.
2. The human can submit legal moves through the web UI.
3. The CPU responds with legal MCTS-selected moves.
4. Illegal moves are rejected cleanly.
5. Game terminal states are detected correctly.
6. Match results are persisted to SQLite.
7. CPU vs. CPU lab batches can be started via API.
8. Lab batches persist games, move histories, and sampled MCTS evaluations.
9. Lab batches can be monitored and cancelled.
10. The MCTS evaluator can be swapped via configuration or compile-time factory without changing tree-search code.
11. The LLM can be disabled without affecting gameplay.
12. LLM failure does not block moves.
13. SQLite WAL mode is enabled.
14. The frontend remains minimal and server-driven.
15. The system operates within its configured memory budget, validated at startup and verified under a full-density batch. *(v1.0: "within configurable constrained-hardware budgets".)*

Version 1.1 adds:

16. A search with the transposition table enabled in `Deterministic` mode returns results identical to a search with the table disabled, at every thread count ([§20.5](20-testing-strategy.md#205-transposition-table-tests)).
17. A batch marked `reproducible: true` produces byte-identical games across two runs at different thread counts.
18. The database writer sustains its committed throughput target with 50 000-row transactions, and the channel's high-water mark is observable.
19. A hard kill mid-batch loses only bulk-class data; every acknowledged durable write survives, and the database opens cleanly.
20. The binary runs with no external service of any kind, and plays a complete game with the model file absent.
21. Three consecutive inference failures route all commentary to the canned fallback within one request, and move latency with the circuit open is indistinguishable from running with the Face disabled.
22. Every persisted BLOB-bearing row carries a `format_version`, and a row with an unknown version is refused rather than misread.

Version 1.4 adds five, all of which exist to keep the GPU optional:

23. **`cargo build --release`, with no features, produces a binary with no CUDA dependency** that links, starts, plays a complete game, and produces commentary on a machine with no driver and no toolkit installed ([§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests)).
24. **A GPU that is absent, busy, or out of VRAM degrades to the CPU profile**, logs exactly one warning, and is visible on `/health` as `device_requested` differing from `device`. It is never a startup failure and never an error response.
25. **Startup refuses a configuration whose active model cannot meet its own `deadline_ms`**, naming the offending key, and warns when the *inactive* profile cannot ([§23.1](23-configuration-example.md#231-startup-validation)).
26. **The process operates within both budgets**: host RSS within `limits.max_total_memory_gb` and VRAM within `limits.max_vram_mb`, each verified under a full-density batch with commentary enabled ([§16.1](16-memory-strategy.md#161-memory-budget), [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)).
27. **`candle_core::Device` is constructed in exactly one function**, verified by a static check in CI ([§19.6.5](19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve)).

Criterion 15 is re-read against the 1.4 budget rather than the 1.1 one: "its configured memory budget" now means 56 GB of host RAM and 4.5 GB of VRAM, not 96 GB and nothing.

---

← [24. Key Architectural Decisions](24-key-decisions.md) · **[Index](README.md)** · [26. Summary](26-summary.md) →
