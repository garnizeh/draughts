# 21. Observability

Minimal but useful telemetry. The v1.0 metrics remain; the 1.1 additions are the ones that answer "is the system keeping up" and "is the model participating at all".

| Metric | Source | Rev |
|---|---|---|
| Active matches | API | 1.0 |
| CPU move latency | Engine | 1.0 |
| MCTS iterations completed | Engine | 1.0 |
| Lab games per minute | Lab Runner | 1.0 |
| DB write batch latency | Persistence | 1.0 |
| SQLite WAL size | Persistence | 1.0 |
| LLM latency | Face Adapter | 1.0 |
| LLM fallback rate | Face Adapter | 1.0 |
| **Transposition hit rate, entry count, collisions, evictions** | Transposition Table | **1.1** |
| **Write channel depth and high-water mark** | Writer Actor | **1.1** |
| **Rows committed, commits, average commit latency** | Writer Actor | **1.1** |
| **Backpressure events** | Writer Actor | **1.1** |
| **Circuit state, trips, short-circuited requests** | Circuit Breaker | **1.1** |
| **Model resident size and load status** | Inference Runtime | **1.1** |
| **Resolved inference device, and whether it matched the request** | Inference Runtime | **1.4** |
| **VRAM used against the [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) budget** | Inference Runtime | **1.4** |
| **Completed MCTS iterations per move** | Engine | **1.4** |
| **Process peak RSS against the [§16.1](16-memory-strategy.md#161-memory-budget) budget** | Runtime | **1.1** |

The four that matter most, in order: **write channel high-water mark** (the single best indicator of whether the system is keeping up), **transposition hit rate** (directly predicts lab throughput), **circuit state** (tells you whether the model is participating), and **peak RSS** (the failure this architecture is most exposed to). Everything else is diagnostic detail once those four are healthy.

Two of the 1.4 additions are there to make a specific silent failure visible. **A resolved device that does not match the request** means commentary fell back to the CPU profile ([§7.4.1](07-face-llm-layer.md#741-device-selection)) — legitimate, and invisible from every other metric until latency drifts. **Completed iterations per move** separates "Play Mode is slow" from "Play Mode is time-bounded and always was", which on this host is the expected steady state ([§16.2](16-memory-strategy.md#162-engine-budgets)).

All of it is exposed on `/api/v1/health` ([§9.2](09-api-contract.md#92-health-endpoint)) and the batch status endpoint ([§9.10](09-api-contract.md#910-get-lab-batch-status)). Logs are secondary.

Logs should include:

- Match IDs.
- Batch IDs.
- Evaluator name.
- Engine budget.
- Error codes.
- LLM fallback usage.
- **Transposition mode and table identity.**
- **The resolved inference device, once, at startup — including the reason if it differs from the request.**
- **Circuit transitions, with the triggering error.**

Do not log full LLM prompts by default.

Logging discipline, which matters more at 1.1 throughput than it did at 1.0:

- Circuit transitions: one line per transition, never one per short-circuited request. An operator should see one line saying the model went away, not ten thousand.
- Backpressure: one line on entering a saturated state, one on leaving it, with the duration.
- Transposition retirement: one line per epoch, with entries retired and the resulting count.
- Per-game and per-position events: never logged. At 2 000 games per minute, logging per game is its own denial of service.

---

← [20. Testing Strategy](20-testing-strategy.md) · **[Index](README.md)** · [22. Deployment Model](22-deployment-model.md) →
