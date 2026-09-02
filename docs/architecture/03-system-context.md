# 3. High-Level System Context

Everything inside the `SINGLE RUST BINARY` subgraph is one operating-system process produced by one `cargo build --release`.

```mermaid
flowchart TB
    UI["<b>Human Player / UI</b><br/>HTMX + Alpine.js"]

    subgraph PROC["SINGLE RUST BINARY — one OS process, zero sidecars"]
        direction TB

        API["<b>HTTP API Layer</b><br/>axum + tokio"]
        SVC["<b>Application Services</b><br/>MatchService<br/>LabService<br/>CommentaryService"]
        RULES["<b>Game Rules Core</b><br/>Board / Legal Moves<br/>Captures / Promotion<br/>Zobrist Hashing"]

        MCTS["<b>MCTS Engine</b><br/>Selection / Expand<br/>EvaluationStrategy<br/>Backpropagation"]
        LAB["<b>Training Lab Runner</b><br/>Headless CPU vs CPU<br/>N worker threads<br/>Sampling / cancellation"]
        FACE["<b>Face Layer</b><br/>CircuitBreaker §7.8<br/>CandleFaceAdapter<br/>CannedFaceAdapter"]

        TT["<b>GLOBAL TRANSPOSITION TABLE</b><br/>Arc&lt;DashMap&lt;TtKey, TtEntry&gt;&gt;<br/>lock-free, sharded, ~24 GB"]
        CANDLE["<b>Candle Inference Runtime</b><br/>candle-core + candle-transformers<br/>quantized GGUF<br/>one resolved Device §7.4.1"]

        CHAN["<b>MPSC WRITE CHANNEL</b> (bounded)<br/>512k messages buffered in RAM"]
        WRITER["<b>DB WRITER ACTOR</b> (1 thread)<br/>50k+ rows per transaction<br/>BEGIN IMMEDIATE / prepared stmts"]
        READPOOL["<b>SQLite Read Pool</b><br/>N read-only WAL connections<br/>status pages, exports"]
    end

    DB[("<b>SQLite database file</b> — WAL mode<br/>4 GB page cache (writer)<br/>8 GB mmap window")]
    GPU["<b>NVIDIA RTX 3050 — 6 GB VRAM</b><br/>optional device, Face layer only<br/>CPU path is always compiled in"]

    UI -->|"HTTP / HTMX partials"| API
    API --> SVC
    SVC --> RULES
    SVC --> MCTS
    SVC --> LAB
    SVC --> FACE

    MCTS --> TT
    LAB -->|"shared by every worker"| TT
    FACE --> CANDLE
    CANDLE -.->|"device = cuda,<br/>falls back to CPU"| GPU

    MCTS --> CHAN
    LAB --> CHAN
    FACE --> CHAN
    CHAN --> WRITER

    WRITER -->|"single writer"| DB
    READPOOL -->|"many readers"| DB
```

```text
  On-disk artifacts loaded at startup, not services:
     ./draughts                 the binary
     ./data/draughts.db         SQLite + -wal + -shm
     ./models/model.gguf        quantized weights, mmap'd into the process
                                (copied to VRAM when device = cuda)
```

The GPU box is drawn outside the process for a reason: it is a **device the process may use**, not a component the process contains. Nothing in the diagram other than the Candle runtime has an edge to it, and the dashed edge is the only one in the drawing that may be absent at runtime without anything else changing.

Compare with v1.0: the `Local Ollama Server` box is gone. There is no longer any process boundary, port, retry policy, health probe, or version-skew risk between the application and the language model. The failure modes that boundary used to produce are replaced by a single in-process failure mode, handled by the circuit breaker in [§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11).

---

← [2. Scope and Constraints](02-scope-and-constraints.md) · **[Index](README.md)** · [4. Separation of Concerns](04-separation-of-concerns.md) →
