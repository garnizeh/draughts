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
- Draw rules for MVP ([§5.3.1](#531-draw-rules-for-mvp--new-in-15)).
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

### 5.3.1 Draw Rules for MVP — New in 1.5

Through 1.4 the responsibility list above said only "draw rules for MVP" and no section said which rules or what thresholds, while [§20.1](20-testing-strategy.md#201-rules-tests) required them tested, [§19.4](19-extensibility-roadmap.md#194-rule-variants) described a future layer that would make their thresholds swappable, and [§25](25-acceptance-criteria.md) criterion 5 required terminal detection to be provable. Every layer above the Rules Core was written against a draw the Rules Core had no rule for. This subsection is that rule.

**The non-progress rule.** A game is drawn after **80 plies** — 40 moves per side — in which no capture has been made and no man has moved. The counter is reset by a capture **or** by any man move, and by nothing else: a king move is not progress. This is the American Checker Federation's 40-move rule, and the reset condition is the load-bearing half of it. Men only ever move forward, so the number of man moves available in a game is bounded; captures are bounded by the twenty-four pieces on the board; therefore the number of resets is bounded, and a game must reach the threshold in a finite number of plies. That termination proof is the reason the rule is specified here rather than left to the first implementation, and it is the thing that is lost if the counter is allowed to reset on a king move.

The counter is `GameState.non_progress_plies`, a `u32` maintained incrementally by `apply_move` beside `ply` and `hash`. Four bytes and no allocation, which matters because `GameState` is cloned on every path the search walks ([§16.2](16-memory-strategy.md#162-engine-budgets)).

**The repetition rule.** A game is drawn when the same position occurs for the **third** time, counted **since the last irreversible move** — the last capture or promotion. A position is identified by its Zobrist key, which already folds in the side to move, so "the same position" means the same key and therefore the same side on move. Counting since the last irreversible move rather than over the whole game is what bounds the history: a capture and a promotion are both undoable only by starting a different game, so no position from before one can recur after it, and the keys held before it can be dropped.

**Which layer adjudicates: the game loop, not `apply_move`.** [§20.5](20-testing-strategy.md#205-transposition-table-tests) requires the transposition table to change how long a search takes and never what it returns, and [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11) caches terminal detection under a `TtKey` derived from the Zobrist key alone. Terminality must therefore be a pure function of `(board, side_to_move)` — and neither draw rule is one. Both depend on how the position was reached. So `apply_move` sets `GameStatus::Finished` for the single position-pure terminal condition, which is that the side to move has no legal move and has therefore lost, and for that condition only. The two draw rules are applied above it, by whatever owns the game: the lab runner's per-worker game loop ([§5.5](#55-training-lab-runner)) and the Play Mode service. That layer owns the key history. `GameState` does not, which is the second reason it stays cheap to clone.

Three consequences follow, and they are why the decision belongs in the document rather than in a pull request:

1. **`TtKey` and [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11) are untouched.** No counter joins the key, no history joins the entry, and `TtKind::Terminal` keeps meaning exactly what it has meant since 1.1. [§6.7.3](06-mcts-extensibility.md#673-probe-and-store) states the same thing from the table's side.
2. **The random rollout applies the non-progress rule and not the repetition rule.** The counter travels inside the cloned `GameState` for nothing; a key history does not, and a random playout repeats a position too rarely for one to pay for itself. This gives the rollout the termination proof it never had: `max_playout_ply` ([§6.3](06-mcts-extensibility.md#63-random-rollout-evaluator)) stops being what guarantees a playout ends and becomes the backstop it was always meant to be. It also makes the non-progress policy part of the distribution the rollout samples, so the policy joins `EvaluatorIdentity` exactly as `max_playout_ply` already has — two configurations with different thresholds are different evaluators and their estimates must not pool.
3. **The search does not see repetition.** The engine can therefore walk into a line the game loop then adjudicates as drawn, while the search believed it was winning. This is an accepted MVP limitation, stated rather than discovered: correcting it means giving the search a path history, which is a change to search design ([§19.4](19-extensibility-roadmap.md#194-rule-variants)), not to the rules.

**All four values are configuration, and the numbers above are the defaults.** [§19.4](19-extensibility-roadmap.md#194-rule-variants) lists draw thresholds among the policies a post-MVP variant layer would make swappable. 1.5 makes that half true early, because the cost is one `[rules.draw]` table and four validated keys ([§23](23-configuration-example.md), [§23.1](23-configuration-example.md#231-startup-validation)) while the benefit is that these numbers live in one file instead of as constants distributed through the rules core. Nothing in the MVP is expected to change them. A configuration that does is no longer playing `english_draughts` even though `games.rules` will still record that it was, so [§23.1](23-configuration-example.md#231-startup-validation) warns — once, naming the keys that moved.

### 5.3.2 Two Fine Rules, Stated — New in 1.5

[§2.1](02-scope-and-constraints.md#21-in-scope) fixes the variant as `english_draughts`, which settles most of the fine rules by reference. Two of them decide code in the move generator, are one sentence each, and are expensive to get wrong silently, so they are written down:

- **A man crowned by a jump does not continue jumping.** Promotion ends the move, even where the newly crowned king would have a further jump available from the square it landed on. This is the English-draughts rule and it is the opposite of the international one, which is the reason it is worth stating.
- **A capture sequence must be completed, but need not be maximal.** When captures are available the side to move must capture, and may choose any one of the available sequences; having chosen, it must play that sequence out until the moving piece has no further jump. There is no requirement to choose the sequence that captures the most pieces.

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
- One repetition window — the Zobrist keys seen since the last capture or promotion — cleared at each irreversible move and at each new game. The worker's game loop adjudicates both draw rules from it and from `GameState.non_progress_plies`, because the Rules Core deliberately does not ([§5.3.1](#531-draw-rules-for-mvp--new-in-15)).
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
