# 26. Summary

This architecture provides a small but robust MVP foundation for an AI-augmented draughts system.

The critical design property is separation:

- The **Rules Core** owns legality.
- The **MCTS Engine** owns move selection.
- The **EvaluationStrategy trait** owns value estimation and is swappable.
- The **Transposition Table** owns cached position facts, and owns nothing else — it may change how long a search takes and never what it returns.
- The **Training Lab** owns headless knowledge generation.
- The **Face/LLM adapter** owns personality and nothing else.
- The **Circuit Breaker** owns the decision to stop asking a failing model, and knows nothing about the game.
- The **DB Writer Actor** owns the single SQLite write lock, and is the only thing in the process that can begin a transaction.
- The **SQLite persistence layer** owns durable training and match records.
- The **HTMX frontend** owns only minimal interaction and rendering.

Version 1.1 changed the hardware assumption, not that structure. Every one of those shifts landed inside an existing seam: the transposition table sits between the engine and the evaluator without either knowing; the writer actor sits behind the persistence layer's API; Candle replaced Ollama behind an unchanged `FaceAdapter` trait; the circuit breaker wraps that trait without reaching through it; and `format_version` is a column, not a redesign.

**Version 1.4 tested the same claim a second time, harder.** The assumed host was replaced with a real one — half the RAM, fewer and slower cores, a third of the memory bandwidth, and an accelerator that had only ever been hypothetical. Every number in the document moved. Not one structural decision did. The GPU arrived through `candle_core::Device`, the one-line seam [§19.6.5](19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve) had been holding open for it, and no caller changed; the memory budget was re-derived row by row without a consumer being added or removed; the transposition table gave up 8 GB and kept its design. That an architecture absorbed a 128× change in available memory at 1.1, and then a halving and a new class of device at 1.4, without a structural rewrite either time, is the strongest available evidence that the v1.0 seams were drawn in the right places.

What 1.1 buys is throughput and simplicity at the same time — a rarer combination than it sounds, and it comes from spending memory rather than adding components. The process got faster by roughly an order of magnitude on lab throughput while shedding an entire external service.

What it costs is stated plainly rather than buried: bulk lab writes live in RAM for a while before they are durable ([§11.4](11-database-architecture.md#114-durability-classes--new-in-11)), a lab batch is reproducible only when it asks to be ([§6.7.5](06-mcts-extensibility.md#675-determinism-and-the-two-table-modes)), a fault in the inference path can take the whole process rather than just the commentary ([§17.5](17-reliability.md#175-process-level-risk-from-in-process-inference)) — and since 1.4 that blast radius includes a GPU driver, in exchange for commentary that meets its deadline at all. Each of those is a deliberate trade with a named mitigation, and each is listed in [Appendix D](appendix-d-risks-and-open-questions.md) so that the next revision inherits the reasoning and not just the result.

The 1.4 trade is the newest and the least settled: the GPU is worth it because the CPU path on this host cannot meet its own latency contract, and it is survivable because the CPU path remains fully built, fully tested, and one configuration key away ([§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests)). The moment that stops being true, the trade stops being defensible.

The MVP remains small while preserving a clear path toward neural evaluation, richer training pipelines, and more advanced AI features without destabilizing the core system. 1.1 shortened that path by linking in the inference runtime a value network would need and caching its outputs in the transposition table; 1.4 shortens it again, because that runtime now demonstrably runs on the GPU the training would use. What remains genuinely expensive is unchanged and is stated where it belongs: batched neural evaluation inside the search is a redesign of the search loop, not a new evaluator ([§19.6.4](19-extensibility-roadmap.md#1964-neural-evaluation-inside-mcts--the-expensive-one)).

---

← [25. MVP Acceptance Criteria](25-acceptance-criteria.md) · **[Index](README.md)** · [Appendix A — Memory Budget at a Glance](appendix-a-memory-budget.md) →
