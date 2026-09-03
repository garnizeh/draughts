# Draughts — System Architecture

**AI-Augmented Draughts Engine and Training Lab**

| | |
|---|---|
| **Version** | 1.5 |
| **Status** | Approved architecture revision — supersedes v1.4 |
| **Primary stack** | Rust, SQLite/WAL, HTMX, Alpine.js, in-process Hugging Face Candle LLM on CUDA |
| **Target host** | Single machine — 64 GB RAM, Xeon E5-2690 v4 (14C/28T), NVIDIA RTX 3050 6 GB. **No component requires the GPU** |
| **Deployment unit** | One executable, one SQLite file, two `.gguf` model files (one per device profile) |

Draughts is a single-binary engine and training lab for the 8x8 draughts variant known as English draughts, American checkers, or straight checkers. It runs exactly two modes: a human match against a Monte Carlo tree search engine, and headless CPU-vs-CPU self-play that generates training data. An in-process language model supplies commentary — on the GPU where one is present, on the CPU otherwise — and is never permitted to touch the game.

The guiding principle is that **the engine is authoritative and everything else is optional**.

---

## Reading Paths

**If you are new to the system** — read [§1](01-executive-summary.md), [§3](03-system-context.md), [§4](04-separation-of-concerns.md), then [§5](05-runtime-components.md). That is roughly forty minutes and covers what exists and who owns what.

**If you are implementing the engine** — [§5.3](05-runtime-components.md#53-game-rules-core), [§5.4](05-runtime-components.md#54-mcts-engine), [§6](06-mcts-extensibility.md) in full, [§15.1](15-concurrency-model.md#151-http-and-engine-separation), [§16.2](16-memory-strategy.md#162-engine-budgets), [§20.1](20-testing-strategy.md#201-rules-tests), [§20.2](20-testing-strategy.md#202-engine-tests), [§20.5](20-testing-strategy.md#205-transposition-table-tests).

**If you are implementing persistence** — [§5.6](05-runtime-components.md#56-persistence-layer), [§11](11-database-architecture.md) in full, [§12](12-database-schema.md), [§13](13-data-dictionary.md), [§15.2](15-concurrency-model.md#152-the-write-path--mpsc-actor-detail), [§15.3](15-concurrency-model.md#153-sqlite-concurrency), [§17.3](17-reliability.md#173-database-failures), [§20.6](20-testing-strategy.md#206-writer-actor-and-durability-tests).

**If you are implementing the Face layer** — [§2.3](02-scope-and-constraints.md#23-explicit-constraint-llm-does-not-play-draughts), [§5.7](05-runtime-components.md#57-face--llm-adapter--in-process-candle-runtime), [§7](07-face-llm-layer.md) in full, [§10.3](10-frontend-architecture.md#103-htmx-interaction-pattern), [§15.4](15-concurrency-model.md#154-cpu-partitioning), [§17.2](17-reliability.md#172-face-failures), [§17.5](17-reliability.md#175-process-level-risk-from-in-process-inference), [§18.2](18-security-and-safety.md#182-llm-prompt-safety), [§20.7](20-testing-strategy.md#207-face-and-circuit-breaker-tests).

**If you are operating a deployment** — [§9.2](09-api-contract.md#92-health-endpoint), [§16](16-memory-strategy.md), [§21](21-observability.md), [§22.5](22-deployment-model.md#225-operational-playbook), [§23](23-configuration-example.md), and [Appendix D](appendix-d-risks-and-open-questions.md).

**If you are working on the GPU path** — [§0.4](00-revision-history.md#04-what-changed-in-14) for why it exists, then [§7.4.1](07-face-llm-layer.md#741-device-selection), [§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget), [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14), [§19.6](19-extensibility-roadmap.md#196-gpu-acceleration), and [§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests). Read [§19.6.6](19-extensibility-roadmap.md#1966-what-cuda-is-not-for--new-in-14) before proposing any other use of the device.

**If you are upgrading from v1.0 or v1.1** — [§0](00-revision-history.md) first, then [Appendix C](appendix-c-migration-from-v1-0.md) and [Appendix E](appendix-e-change-checklist.md). **From v1.3** — [§0.4](00-revision-history.md#04-what-changed-in-14) and [Appendix F](appendix-f-v13-to-v14-checklist.md). **From v1.4** — [§0.5](00-revision-history.md#05-what-changed-in-15) alone; it is one subsection and its configuration surface.

---

## Contents

### Orientation

| § | Document | What it settles |
|---|---|---|
| 0 | [Revision History](00-revision-history.md) | The six shifts that produced v1.1, the naming and corrections in v1.2, the model re-baseline in v1.3, the host and CUDA re-baseline in v1.4, and the draw rules stated in v1.5 |
| 1 | [Executive Summary](01-executive-summary.md) | The two execution modes and what the MVP emphasizes |
| 2 | [Scope and Constraints](02-scope-and-constraints.md) | What is in, what is out, the real hardware baseline, and the constraint that the LLM does not play draughts |
| 3 | [High-Level System Context](03-system-context.md) | Every component in one process, the files on disk, and the one optional device |
| 4 | [Separation of Concerns](04-separation-of-concerns.md) | Per-layer responsibilities and, more usefully, forbidden responsibilities |
| 5 | [Runtime Components](05-runtime-components.md) | The seven runtime components, one subsection each, and the draw rules the Rules Core owes the game |

### The Engine

| § | Document | What it settles |
|---|---|---|
| 6 | [Rust MCTS Extensibility Design](06-mcts-extensibility.md) | Domain types, the `EvaluationStrategy` trait, the global transposition table, and the two table modes |
| 8 | [Game Modes and Execution Flows](08-game-modes-and-flows.md) | Play Mode and Training Lab Mode, end to end |
| 14 | [Training Lab Sampling Strategy](14-sampling-strategy.md) | What is recorded, at what density, and what it costs on disk |

### The Face Layer

| § | Document | What it settles |
|---|---|---|
| 7 | [Pluggable "Face" / LLM Layer](07-face-llm-layer.md) | The adapter trait, the Candle runtime, **device selection**, the two model profiles, guardrails, and the circuit breaker |

### Interfaces

| § | Document | What it settles |
|---|---|---|
| 9 | [API Contract](09-api-contract.md) | Every endpoint, request, response, and error code |
| 10 | [Frontend Architecture](10-frontend-architecture.md) | Pages, board rendering, and the decoupled commentary pane |

### Persistence

| § | Document | What it settles |
|---|---|---|
| 11 | [Database Architecture](11-database-architecture.md) | Per-role pragmas, the MPSC writer actor, backpressure, and the two durability classes |
| 12 | [Database Schema](12-database-schema.md) | The DDL, and the migration from a v1.0 database |
| 13 | [Data Dictionary](13-data-dictionary.md) | Every encoding, and the `format_version` rules that keep them readable |

### Runtime Behaviour

| § | Document | What it settles |
|---|---|---|
| 15 | [Concurrency Model](15-concurrency-model.md) | Four thread pools, the write path in detail, id allocation, and CPU partitioning |
| 16 | [Hardware and Memory Strategy](16-memory-strategy.md) | Where the 64 GB and the 6 GB of VRAM go, with a stated ceiling per consumer on each |
| 17 | [Reliability and Failure Handling](17-reliability.md) | What each subsystem does when it fails, saturates, or is cancelled |
| 18 | [Security and Safety](18-security-and-safety.md) | Input validation, prompt safety, output sanitization |

### Evolution and Verification

| § | Document | What it settles |
|---|---|---|
| 19 | [Extensibility Roadmap](19-extensibility-roadmap.md) | Neural evaluators, PUCT, knowledge discovery, rule variants, format evolution, and what the GPU is and is not for |
| 20 | [Testing Strategy](20-testing-strategy.md) | Ten suites, each mapped to a failure mode the architecture can produce |
| 21 | [Observability](21-observability.md) | The metrics, the four that matter most, and logging discipline |

### Operations

| § | Document | What it settles |
|---|---|---|
| 22 | [Deployment Model](22-deployment-model.md) | Single-machine deployment, why not to split the process, startup, shutdown, and the operational playbook |
| 23 | [Configuration Example](23-configuration-example.md) | One annotated `draughts.toml`, and mandatory startup validation |

### Conclusions

| § | Document | What it settles |
|---|---|---|
| 24 | [Key Architectural Decisions](24-key-decisions.md) | Every decision with its rationale and the revision that introduced it |
| 25 | [MVP Acceptance Criteria](25-acceptance-criteria.md) | The twenty-seven conditions for "done" |
| 26 | [Summary](26-summary.md) | What the structure buys, what it costs, and why the seams held |

### Appendices

| | Document | What it settles |
|---|---|---|
| A | [Memory Budget at a Glance](appendix-a-memory-budget.md) | Both budgets — 64 GB of host RAM and 6 GB of VRAM — in one picture |
| B | [Performance Targets](appendix-b-performance-targets.md) | Numbers a build should be measured against |
| C | [Migration from v1.0](appendix-c-migration-from-v1-0.md) | Ten ordered steps, and what rollback does |
| D | [Risks and Open Questions](appendix-d-risks-and-open-questions.md) | Thirteen recorded risks, so they stay decisions rather than discoveries |
| E | [v1.0 → v1.1 Change Checklist](appendix-e-change-checklist.md) | Historical. Every value that moved in that revision |
| F | [v1.3 → v1.4 Change Checklist](appendix-f-v13-to-v14-checklist.md) | **Current.** Every value that moved when the assumed host was replaced with the real one |

---

## Conventions

- **Cross-references** use the section symbol and are links: [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11) resolves to the transposition table design. Section numbers are stable across revisions; a reference to a subsection with no heading of its own resolves to its nearest enclosing heading.
- **Diagrams** are Mermaid and render inline on GitHub and in any Mermaid-aware viewer. ASCII blocks are retained wherever they carry something a diagram cannot — a byte layout, a sizing calculation, a directory tree.
- **Code** is illustrative Rust, SQL, TOML, and HTML. It specifies shape and intent, not a finished implementation; `todo!()` appears where a body is deliberately unspecified.
- **`§N` in a table's "Rev" or "Primary Sections" column** points at the section that owns the decision, not at the one that mentions it.
- **Bold in tables** marks material introduced in the revision the table is discussing — v1.1 in the historical tables, v1.4 and v1.5 in the current ones.
- **Hardware figures are derived, not asserted.** Every memory, latency, and throughput number traces back to the host in [§2.4](02-scope-and-constraints.md#24-hardware-baseline) through a stated relation, so it can be re-derived for a different machine rather than waiting for a measurement to contradict it. Two revisions have now been caused by a number that was not derived.

---

*Draughts — System Architecture, Version 1.5.*
