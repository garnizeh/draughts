# 24. Key Architectural Decisions

| Decision | Rationale | Rev |
|---|---|---|
| Rust core engine | Performance, safety, extensibility | 1.0 |
| Trait-based MCTS evaluator | Enables future neural evaluator without core rewrite | 1.0 |
| SQLite WAL only | MVP simplicity, local storage, sufficient for single-node volume | 1.0 |
| HTMX frontend | Minimal client complexity, fast MVP delivery | 1.0 |
| Alpine.js only for micro-interactions | Avoids SPA state burden | 1.0 |
| LLM never calculates moves | Preserves engine correctness and determinism | 1.0 |
| Move histories stored as compact BLOBs | Efficient for millions of games | 1.0 |
| Positions/edges sampled | Prevents storage explosion; density unchanged by the 1.4 re-baseline, because halving RAM changes what is cached, not what is written | 1.0 |
| Single writer DB pattern | Avoids SQLite write contention | 1.0 |
| Headless lab runner | Enables training without UI overhead | 1.0 |
| **Global lock-free transposition table (`DashMap`)** | **Self-play revisits positions constantly. Memory is no longer scarce; recomputation is. Shared across all workers, not per-search** | **1.1** |
| **Two transposition modes, not one** | **A shared cache written by concurrent games breaks bit-reproducibility unless entries are position-pure. Rather than choose between speed and replayability, both are offered and the choice is recorded per batch** | **1.1** |
| **MPSC actor with 50k-row transactions** | **SQLite permits one writer. Rather than fight it, make that writer maximally efficient and absorb bursts in RAM** | **1.1** |
| **Bounded, not unbounded, write channel** | **An unbounded channel trades a visible stall for an invisible OOM. Backpressure is a designed behavior with three per-producer policies** | **1.1** |
| **Two named durability classes** | **Buffering 500k messages in RAM is correct for regenerable lab data and unacceptable for human match history. Naming the classes prevents one policy being applied to both** | **1.1** |
| **Candle in-process, Ollama removed** | **Eliminates a daemon, a REST hop, a serialization boundary, and an independent failure domain. Deployment becomes one executable** | **1.1** |
| **Commentary decoupled from the move response** | **A *successful* 2.5 s inference on the critical path is still 2.5 s. The board must return at engine speed, not model speed** | **1.1** |
| **Circuit breaker on the Face layer** | **An in-process model fails slowly, by consuming CPU the engine needs. Retrying it per move is a self-inflicted denial of service** | **1.1** |
| **`Face::commentary` returns `Commentary`, not `Result`** | **From the game's point of view commentary cannot fail, only be less interesting. Making that a type-level fact removes the error path a caller could mishandle** | **1.1** |
| **`format_version` on `games` and `positions`** | **A 16-byte undelimited BLOB cannot be distinguished from a differently-encoded one after the fact. The version tag is what makes a future encoding change a migration rather than an archaeology project** | **1.1** |
| **Explicit per-consumer memory budget** | **An unbudgeted process does not use memory well; it uses it accidentally, then dies at the worst moment. Validated at startup — and at 64 GB with 24 committed to one hash table, the margin for getting it wrong is half what 1.1 assumed** | **1.1** |
| **CPU-only, with `Device` as a single-point seam** | **A GPU was available but required by nothing. Constructing `Device` in exactly one place made CUDA a cargo feature rather than a refactor. Superseded by the row below; the seam is what made that supersession a one-line change** | **1.3** |
| **CUDA is the default Face device, CPU is a first-class fallback** | **The measured bandwidth of this host — two DDR4-2400 channels, ~15 GB/s to two cores — makes the CPU path miss its own `deadline_ms` at any model worth listening to, which is a permanently open circuit reporting itself healthy ([§0.4.3](00-revision-history.md#043-consequence-two--the-cpu-inference-path-cannot-meet-its-own-deadline)). The card has ~10× the bandwidth and returns two cores to the engine. `Device` is still constructed in exactly one place ([§7.4.1](07-face-llm-layer.md#741-device-selection))** | **1.4** |
| **Two model profiles, not one `model_path`** | **The resolved device can change between boots with no config edit — a driver update, a busy card, a binary built without the feature. A single model path turns that into a 4.3 s model against a 2.5 s deadline: a correct degradation that produces a silent outage ([§7.5.4](07-face-llm-layer.md#754-two-profiles-not-one-model-path))** | **1.4** |
| **A separate VRAM budget, never merged into the host budget** | **Two ceilings, two allocators, two ways to die. Merging them would let 3 GB of "free" host RAM appear to excuse a model that does not fit on a 6 GB card ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14))** | **1.4** |
| **The engine never touches the GPU** | **Rollouts, move generation, and transposition probes are branchy, latency-bound and — for the 21 GB table — far too large for 6 GB of VRAM. This card is worth using for dense batchable tensor math and nothing else in this codebase ([§19.6.6](19-extensibility-roadmap.md#1966-what-cuda-is-not-for--new-in-14))** | **1.4** |
| **GGUF loader dispatched on `general.architecture`** | **Each quantized family in `candle-transformers` has its own `ModelWeights` type and they share no trait. Pinning one at compile time would limit config-only model swaps to a single architecture, which is weaker than what §7.5 promises ([§7.4](07-face-llm-layer.md#74-candle-inference-runtime--replaces-the-ollama-rest-adapter))** | **1.3** |
| **Per-role SQLite pragmas** | **`cache_size` is per-connection. One pragma set across the read pool multiplies the writer's cache by the pool size — 28 GB instead of 5.5 GB on the 1.4 budget** | **1.1** |

---

← [23. Configuration Example](23-configuration-example.md) · **[Index](README.md)** · [25. MVP Acceptance Criteria](25-acceptance-criteria.md) →
