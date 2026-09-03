# 8. Game Modes and Execution Flows

## 8.1 Human vs. CPU — Play Mode

```mermaid
flowchart TB
    A(["Human clicks move"]) --> B["Frontend POSTs the move to the API"]
    B --> C["API validates match state"]
    C --> D["Rules Core validates the move"]
    D --> E["Move applied to GameState<br/>Zobrist updated incrementally"]
    E --> F{"Game over?<br/>Rules Core terminality,<br/>or a §5.3.1 draw"}

    F -->|Yes| G1["Enqueue durable write:<br/>game result, ack awaited"]
    G1 --> G2["Return the finished board"]
    G2 --> G3["Optional commentary"]
    G3 --> Z

    F -->|No| H["<b>MCTS Engine</b> searches for the CPU response<br/>probes and stores the shared transposition table<br/>(Deterministic mode)"]
    H --> I["CPU move selected"]
    I --> J["Move applied to GameState"]
    J --> J2{"Game over?<br/>the same check, after<br/>the engine's move too"}
    J2 -->|Yes| G1
    J2 -->|No| K["Enqueue durable write: move history, ack awaited<br/><i>durable class, §11.4</i>"]
    K --> L["Face::commentary(ctx)"]
    L --> M{"Circuit state"}
    M -->|CLOSED| N["Candle inference, deadline 2500 ms"]
    M -->|OPEN| O["Canned line, ~0 ms"]
    N --> Z
    O --> Z
    Z(["Return the updated board partial to the UI"])
```

Important constraints:

- Human move validation is always done by Rules Core.
- **The two draw rules are adjudicated here, not by `apply_move`** ([§5.3.1](05-runtime-components.md#531-draw-rules-for-mvp--new-in-15)). The match service holds the Zobrist keys seen since the last capture or promotion and checks them, and reads `GameState.non_progress_plies` against the configured threshold, after every applied move — its own and the engine's. The Rules Core reports only that the side to move has no move.
- CPU move is chosen only by MCTS.
- LLM commentary is generated after state transition, not before.
- If the model fails, the game continues unaffected and the breaker absorbs the fault.
- Play Mode writes use the **durable** write class ([§11.4](11-database-architecture.md#114-durability-classes--new-in-11)): the response is not returned until the writer actor has committed. A human's game is not regenerable data, and it is a handful of rows per move — there is no throughput argument for buffering it.

## 8.2 CPU vs. CPU — Training Lab Mode

```mermaid
flowchart TB
    A(["Operator POSTs a lab batch config"]) --> B["LabService creates the lab_batches row<br/><i>durable write, ack awaited</i>"]
    B --> C["Lab Runner claims the batch, sizes the worker pool,<br/>allocates or attaches the shared TranspositionTable,<br/>and opens the MPSC write channel"]
    C --> D

    subgraph W["For each worker thread, for each game in its slice"]
        direction TB
        D["Derive seed = hash(batch_seed, game_index)"]
        D --> E["Lease id ranges for games and positions<br/>from the id allocator"]
        E --> F["Initialize the seeded GameState"]
        F --> G{"Game over?<br/>terminality, or a §5.3.1 draw"}
        G -->|No| H["MCTS search with the configured evaluator"]
        H --> I{"probe shared TT"}
        I -->|Hit| J["Reuse the cached moves and value"]
        I -->|Miss| K["Evaluate, then store"]
        J --> L["Select move"]
        K --> L
        L --> M["Optionally sample root and child stats"]
        M --> N["Apply move"]
        N --> O["Append the move to the in-worker history buffer<br/>and the new position's key to the repetition window,<br/>which was seeded with the key it opened on (§5.3.1)"]
        O --> G
    end

    G -->|Yes| P["send WriteOp::GameFinished — non-blocking unless the channel is full<br/>send WriteOp::Positions(batch)<br/>send WriteOp::Edges(batch)<br/>send WriteOp::BatchProgress — coalesced, every N games"]
    P --> Q["MPSC channel → <b>DB Writer Actor</b><br/>50k-row transactions → SQLite"]
    Q --> R["Batch completed, cancelled, or failed"]
    R --> S(["Final Flush barrier awaited<br/>WAL checkpoint TRUNCATE<br/>PRAGMA optimize"])
```

Training Lab does not invoke the LLM. `face.lab_mode_enabled` defaults to `false`, and the breaker is not consulted at all.

Two things a lab worker cannot do, by construction: open a database connection, and begin a transaction. It has neither a connection handle nor the API to obtain one. Everything it wants persisted goes into the channel.

---

← [7. Pluggable "Face" / LLM Layer](07-face-llm-layer.md) · **[Index](README.md)** · [9. API Contract](09-api-contract.md) →
