# 5. Runtime Components

## 5.1 Frontend

Unchanged from v1.0. Minimalist web UI using:

- HTMX for server-driven HTML updates.
- Alpine.js only where tiny local interactions are needed, such as selecting a square.
- No SPA framework.
- No complex animation.
- No client-side game state beyond transient selection.

The frontend renders:

- Board grid.
- Legal move highlights.
- Current turn.
- Game status.
- Commentary pane.
- Lab batch status page.

HTMX interactions:

- Click a piece.
- Click destination.
- POST move to server.
- Server returns updated board partial.
- Commentary optional via asynchronous partial or polling.

One addition: the commentary pane must render correctly when commentary is `null` or came from the canned fallback. The circuit breaker ([§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11)) makes fallback a routine steady state, not an exceptional one, and the UI must not treat it as an error.

## 5.2 Rust HTTP API

Recommended implementation:

- `axum` on the Tokio async runtime.
- CPU-bound engine work executed via `spawn_blocking` or a dedicated `rayon` pool.
- Inference work dispatched to the inference runtime's own thread, never executed on a Tokio worker.

Responsibilities:

- Start human matches.
- Accept human moves.
- Return updated game state.
- Trigger CPU responses.
- Start and monitor lab batches.
- Serve health, config, and runtime-statistics endpoints.
- Serve HTML partials for HTMX.

The API layer holds `Arc` handles to the engine pool, the transposition table, the writer channel, and the Face facade. It owns none of them.

## 5.3 Game Rules Core

Pure Rust domain library.

Responsibilities:

- Board representation.
- Side-to-move tracking.
- Legal move generation.
- Mandatory captures.
- Multi-jump sequences.
- Promotion.
- Terminal detection.
- Draw rules for MVP.
- Deterministic state hashing.

**Zobrist hashing is promoted from "deterministic state hashing" to a first-class, specified requirement in 1.1**, because it is now the key of a shared cache rather than merely a column in `positions`. Requirements:

- A fixed, compile-time constant Zobrist key table, generated from a hard-coded seed. The keys must never change between builds without a `format_version` bump, since `positions.board_hash` is persisted.
- Incremental update on `apply_move` — full recomputation per node is not acceptable at lab throughput.
- Side-to-move folded into the hash.
- A documented, tested collision policy: the transposition table stores enough of the position to verify a hit ([§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11)), so a Zobrist collision degrades performance, never correctness.

This component must remain:

- Dependency-light.
- Fully unit-testable.
- Free from I/O.
- Free from LLM concerns.
- Free from persistence concerns.

## 5.4 MCTS Engine

The MCTS engine is generic over evaluation strategy.

It implements:

- Selection.
- Expansion.
- Simulation / leaf evaluation.
- Backpropagation.
- Budget management.
- Move selection for gameplay.
- Root statistics export for training data.
- **Transposition probe on expansion and store on backpropagation.**

The engine must not hard-code random rollout logic. Random rollout is one implementation of the evaluation strategy.

New in 1.1, the engine holds an `Arc<TranspositionTable>` supplied by its owner. The table is *shared across every worker thread and every concurrently running lab game*. It is not per-search and not per-game. Its lifecycle is the lifecycle of the process, or of a lab batch, whichever the operator configures. Full design in [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11).

The engine must remain correct with the table disabled. `TranspositionTable::disabled()` returns a null implementation whose probe always misses and whose store is a no-op; every test in the rules and search suites must pass in both configurations, and a nightly CI job runs the golden-search suite with the table off to prove the cache is not load-bearing for correctness.

## 5.5 Training Lab Runner

A headless service that runs CPU vs. CPU games.

Responsibilities:

- Execute configured self-play batches across a worker pool sized to physical cores ([§15.4](15-concurrency-model.md#154-cpu-partitioning): 10 workers on the 14-core target host).
- Use the same MCTS engine as Play Mode.
- Share one transposition table across all workers in the batch.
- Sample positions and MCTS statistics.
- **Emit write messages into the MPSC channel rather than touching SQLite.** A lab worker holds no database connection and cannot begin a transaction.
- Support cancellation and progress reporting.
- Operate without UI.

Each worker owns:

- One `GameState` and one MCTS arena, reused across games to avoid reallocation churn.
- One deterministic RNG seeded from `(batch_seed, game_index)`.
- One id allocator lease for pre-assigning `games.id` and `positions.id` ([§15.2](15-concurrency-model.md#152-the-write-path--mpsc-actor-detail)), which is what makes it possible to build `position_edges` rows before their parent rows have been committed.

## 5.6 Persistence Layer

SQLite-only repository, restructured in 1.1 around a strict single-writer actor.

Responsibilities:

- Schema migrations and `format_version` enforcement.
- Ownership of exactly one write connection, held by the writer actor thread and by nothing else.
- A pool of read-only connections for status pages and export.
- Prepared-statement caching.
- Monotonic id allocation for `games` and `positions`.
- Batch composition, transaction management, and WAL checkpoint scheduling.
- Read APIs for status pages and future export.

The write connection is not behind a mutex, because it is not shared. It lives on the writer thread's stack. This is the difference between "we serialize access with a lock and hope contention is low" and "concurrent access is unrepresentable". Full design in [§11.2](11-database-architecture.md#112-write-strategy--mpsc-actor-pattern) and [§15.2](15-concurrency-model.md#152-the-write-path--mpsc-actor-detail).

## 5.7 Face / LLM Adapter — In-Process Candle Runtime

**This component is fully rewritten in 1.1.** v1.0 called an external Ollama daemon over HTTP. v1.1 loads the model into the application's own memory space.

Responsibilities:

- Resolve the inference `Device` exactly once at startup — CUDA where configured and available, CPU otherwise ([§7.4.1](07-face-llm-layer.md#741-device-selection)).
- Load a quantized `.gguf` model file at startup, or lazily on first commentary request, onto that device.
- Own the tokenizer, the model weights, the KV cache, and the sampling parameters.
- Receive a safe narrative context ([§7.3](07-face-llm-layer.md#73-commentary-context)).
- Run inference on a dedicated thread with a hard token budget and a hard wall-clock budget.
- Return short commentary.
- Report failures to the circuit breaker ([§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11)) and never retry internally.
- Sanitize and truncate output.

Consequences of moving in-process:

| Concern | v1.0 (Ollama REST) | v1.1 (Candle in-process) |
|---|---|---|
| Deployment | Two processes, one of them externally managed | One binary + one model file |
| Failure domain | Network, daemon lifecycle, HTTP status codes, JSON parsing | A `Result<_, candle_core::Error>` |
| Latency floor | HTTP + JSON + daemon scheduling on every taunt | Direct function call |
| Model size ceiling | Whatever the daemon was configured for | On CUDA: bounded by **5.0 GB of usable VRAM** ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)). On CPU: bounded by the 2-core cap in [§15.4](15-concurrency-model.md#154-cpu-partitioning) and this host's memory bandwidth, long before any memory budget binds — 0.5B, not 1.5B ([§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget)) |
| Resource contention | Invisible; a separate process competing for CPU | Explicit; the inference thread is inside our own thread budget and can be capped ([§15.4](15-concurrency-model.md#154-cpu-partitioning)). On CUDA it leaves the CPU budget almost entirely |
| New risk | — | **A bug in the inference path can abort the whole process**, and on CUDA the driver is inside that blast radius. Mitigated in [§17.5](17-reliability.md#175-process-level-risk-from-in-process-inference) |

The Face layer must never receive:

- Full legal move lists.
- Engine search internals.
- MCTS visit counts intended for decision-making.
- Transposition table handles.
- Raw board state if not needed.

It may receive:

- Event type: capture, promotion, win, loss, draw.
- Side names.
- Material difference.
- Ply number.
- Game result.
- Requested tone.

---

← [4. Separation of Concerns](04-separation-of-concerns.md) · **[Index](README.md)** · [6. Rust MCTS Extensibility Design](06-mcts-extensibility.md) →
