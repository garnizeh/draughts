# 2. Scope and Constraints

## 2.1 In Scope

- English draughts rules on an 8x8 board for MVP. This is the variant also known as American checkers or straight checkers; the ruleset identifier is `english_draughts`.
- Human vs. CPU gameplay.
- CPU vs. CPU self-play batch generation.
- MCTS engine with a pluggable evaluator and a shared transposition table.
- SQLite persistence for games, moves, terminal results, and sampled MCTS evaluations.
- Minimal HTMX/Alpine.js frontend.
- In-process LLM commentary via Candle behind an abstract adapter, with circuit-breaker protection, running on **CUDA where available and on the CPU otherwise** — the same code path, one resolved `Device` ([§7.4.1](07-face-llm-layer.md#741-device-selection)).
- Single-machine, single-process deployment.
- Explicit memory budgeting across engine, database, and inference, plus a **separate VRAM budget** for the GPU device ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)).

## 2.2 Out of Scope for MVP

- Human vs. Human online play.
- Distributed training and multi-host lab runs.
- **GPU-accelerated *training*, and GPU evaluation inside MCTS.** Commentary inference on CUDA is in scope and is the default on the target host ([§7.4.1](07-face-llm-layer.md#741-device-selection)); offline value-network training and batched neural evaluation inside the search remain out of scope, for the reasons in [§19.6.3](19-extensibility-roadmap.md#1963-offline-value-network-training--architecturally-free) and [§19.6.4](19-extensibility-roadmap.md#1964-neural-evaluation-inside-mcts--the-expensive-one).
- **Any component requiring a GPU.** Every component must remain correct, and testable, with no accelerator present and with the `cuda` feature off.
- Advanced animations, board themes, or complex client-side state.
- Cloud database or multi-node persistence.
- User accounts, ranking, matchmaking, or social features.
- LLM move suggestion or move calculation.
- Fine-tuning or training the language model. The `.gguf` is a fixed, read-only artifact.

## 2.3 Explicit Constraint: LLM Does Not Play Draughts

The LLM is a presentation/personality layer only. Moving it in-process ([§5.7](05-runtime-components.md#57-face--llm-adapter--in-process-candle-runtime)) does **not** relax this constraint — it tightens the need to state it, because the model now shares an address space with the engine.

It may generate:

- Trash talk.
- Commentary.
- Game-end remarks.
- Positional flavor text.

It must never:

- Choose moves.
- Validate moves.
- Alter engine state.
- Influence MCTS selection.
- Modify persisted game records.
- Hold a mutable reference to a `GameState`, a `MctsEngine`, or the transposition table.

The engine is the only authoritative move generator. The Face layer receives an owned, narrow `CommentaryContext` value ([§7.3](07-face-llm-layer.md#73-commentary-context)) and returns a `String`. That is the entire interface, and it is enforced by types, not by convention.

## 2.4 Hardware Baseline

| Resource | Assumption |
|---|---|
| RAM | **64 GB** — 2 × 32 GB DDR4-2400 ECC RDIMM |
| Memory bandwidth | **Two populated channels of four; ~38 GB/s peak, ~25–30 GB/s achievable.** This is a first-class constraint, not a footnote: it sizes the Face model ([§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget)) and bounds transposition-table probe throughput ([§16.3](16-memory-strategy.md#163-transposition-table-sizing)) |
| CPU | **Intel Xeon E5-2690 v4** — 14 physical cores / 28 threads, 2.6 GHz base, 3.5 GHz turbo, 35 MB L3, one NUMA node. Correctness must hold at 2 cores |
| GPU | **NVIDIA GeForce RTX 3050, 6 GB GDDR6**, CUDA 13.2, compute capability 8.6. Used by the Face layer by default and by nothing else. **No component may require it**, hard-fail without it, or behave differently in its absence beyond being slower ([§7.4.1](07-face-llm-layer.md#741-device-selection)) |
| Storage | Local SSD/NVMe. The lab database is expected to reach hundreds of GB over its life |
| OS | Linux primary; macOS supported for development (CPU path only) |

A single-machine, single-process design at this size is not a compromise — it is the simplest thing that meets the throughput target, and it removes an entire class of distributed-systems failure modes that the MVP has no budget to handle.

### 2.4.1 Two Notes for Whoever Operates This Host

**Only two of four memory channels are populated.** Populating the remaining two would roughly double achievable bandwidth, which is the binding constraint on both CPU inference ([§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget)) and transposition-table probe rate ([§16.3](16-memory-strategy.md#163-transposition-table-sizing)). It is the single highest-value hardware change available, ahead of more RAM and well ahead of a larger GPU. It is recorded here rather than assumed — every number in this document is derived for the two-channel configuration as it stands.

**Roughly 1 GB of the 6 GB of VRAM is held by the desktop session.** The VRAM budget in [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) assumes ~5.0 GB usable, not 6.0. A model chosen against the nameplate figure will load and then fail on its first long generation, which is a worse failure than refusing to load.

---

← [1. Executive Summary](01-executive-summary.md) · **[Index](README.md)** · [3. High-Level System Context](03-system-context.md) →
