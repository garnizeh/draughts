# 19. Extensibility Roadmap

## 19.1 Neural Evaluator Integration

Future addition:

```rust
pub struct NeuralEvaluator { ... }

impl EvaluationStrategy for NeuralEvaluator { ... }
```

No MCTS rewrite required, and 1.1 shortens this path considerably:

- **Candle is already a dependency and already linked.** v1.0 listed "ONNX Runtime, Candle, tract, or an external inference sidecar" as open options and leaned toward a sidecar on constrained hardware. That question is now answered by the Face layer's own implementation: the inference stack is in-process, and the value network should use the same one.
- **The transposition table already caches pure evaluator outputs.** A neural evaluator returns `is_position_pure() == true`, so its forward passes are cached even in `Deterministic` mode — the highest-value case for a 24 GB table, since a forward pass costs orders of magnitude more than a rollout.
- **The CUDA path is now built, not scoped.** [§7.4.1](07-face-llm-layer.md#741-device-selection) resolves a device, the `cuda` cargo feature exists, and the build is known to work on this host. A value network no longer has to bring its own accelerator story — though it does have to fit in what [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) leaves, and it has to solve the batching problem in [§19.6.4](#1964-neural-evaluation-inside-mcts--the-expensive-one) regardless.
- **`EvaluatorIdentity` already scopes cache entries to a checkpoint.** Swapping checkpoints cannot serve stale values.
- **The training data exists**, and 1.1's higher sampling density and larger batches are what make it sufficient.

The sidecar option is explicitly withdrawn. It was a concession to constrained hardware; on this host it would reintroduce exactly the process boundary that shift 4 removed.

---

## 19.2 Policy-Based MCTS

Future MCTS selection can become PUCT:

```text
score(child) = Q(child) + c_puct * P(child) * sqrt(parent_visits) / (1 + child_visits)
```

The existing `move_priors` extension point supports this. Priors are a natural addition to `TtEntry` when that work happens — cached alongside the move list, under the same `EvaluatorIdentity` scoping, and requiring a table-layout change but no format version bump, since the table is not persisted.

---

## 19.3 Knowledge Discovery

Future analytics can query:

- High-win sequences.
- Frequent terminal states.
- Opening lines by board hash.
- Endgame positions by material count.
- MCTS policy targets.

Example:

```sql
SELECT board_hash, COUNT(*) AS count, AVG(outcome) AS avg_outcome
FROM positions
WHERE batch_id = ?
  AND format_version = 1        -- hashes are only comparable within a version
GROUP BY board_hash
ORDER BY count DESC
LIMIT 100;
```

The `format_version` predicate is not defensive noise. `board_hash` is a Zobrist value, and [§13.7](13-data-dictionary.md#137-format_version--new-in-11) rule 6 makes a key-table change a version bump precisely so that grouping across versions cannot silently merge unrelated positions.

With a 4 GB page cache and an 8 GB mmap window ([§11.1](11-database-architecture.md#111-sqlite-runtime-configuration)), aggregate queries of this shape run against memory for the hot working set rather than against disk. At 1.3's figures the whole index was expected to be resident; at 1.4's, the recent-batch working set is, and a query spanning the full history will touch disk. That is a difference in interactive feel, not in what is answerable.

---

## 19.4 Rule Variants

The Rules Core should isolate rule policy:

- Forced captures.
- Flying kings.
- Draw thresholds — **the first of these dials to exist, configurable since 1.5** ([§5.3.1](05-runtime-components.md#531-draw-rules-for-mvp--new-in-15), [§23](23-configuration-example.md)). The `[rules.draw]` keys default to the `english_draughts` values.
- International rules.

For MVP, only one *variant* is enabled: the draw thresholds move and nothing else does. A configuration that changes them is still playing on an 8×8 board with mandatory captures and non-flying kings, which is what makes it a configuration key rather than a variant identifier — and why [§23.1](23-configuration-example.md#231-startup-validation) warns rather than refuses, since `games.rules` records `english_draughts` for a game whatever thresholds it was played under.

A rule variant changes which moves are legal from a position, which makes cached move lists variant-specific. When variants land, the variant identifier must be folded into `TtKey` or into the table's identity, exactly as `EvaluatorIdentity` is today. The draw thresholds are the exception that proves the rule and are worth understanding before the next dial is turned: they need no such folding because [§5.3.1](05-runtime-components.md#531-draw-rules-for-mvp--new-in-15) deliberately keeps them out of everything the table caches — they change no legal move list and no terminal detection — except the rollout's `EvaluatorIdentity`, which already carries them. A dial that changes legal moves will not have that option. International draughts on a 10×10 board would additionally change the board encoding, and is therefore a `format_version` bump.

---

## 19.5 Format Version Evolution

The mechanism from [§13.7](13-data-dictionary.md#137-format_version--new-in-11), seen from the roadmap side. Plausible future versions:

| Hypothetical | Version | Why it is a bump |
|---|---:|---|
| Widen `Move` to u32 for 10×10 boards | 2 | Record width changes; `games.moves` becomes undecodable under v1 rules |
| Store a per-move clock delta | 3 | Record layout changes |
| Re-generate the Zobrist key table | 4 | `board_hash` values change meaning even though layout does not |
| Add a column to `positions` | — | Not a bump. Additive columns with defaults are ordinary migrations |

Procedure for a bump:

1. Add the new encoder and set `CURRENT_FORMAT_VERSION`.
2. Keep every prior decoder. Add a `match` arm; delete nothing.
3. Add a round-trip test per version, plus a fixture file of rows in each historical format ([§20.8](20-testing-strategy.md#208-format-version-tests)).
4. Never rewrite historical rows. Mixed versions in one table are the expected steady state, and the join in [§19.3](#193-knowledge-discovery) shows how queries handle it.
5. Bump the export format's `format_version` field so downstream pipelines fail loudly on an unknown version rather than misreading a BLOB.

---

## 19.6 GPU Acceleration

**Status changed in 1.4.** Through 1.3 this section was recorded intent with a target device. The target device turned out to be real, the CPU inference path turned out to miss its own deadline on the actual host ([§0.4.3](00-revision-history.md#043-consequence-two--the-cpu-inference-path-cannot-meet-its-own-deadline)), and use #1 below is therefore **built rather than planned**. The section is kept, and re-scoped, because the other two uses are still ahead and the reasoning about their relative cost is the valuable part.

The host carries an **NVIDIA RTX 3050, 6 GB GDDR6**, CUDA 13.2, compute capability 8.6. Nothing in the MVP may require it ([§2.4](02-scope-and-constraints.md#24-hardware-baseline)), and the CPU path must stay correct and tested ([§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests)).

### 19.6.1 Three Uses, Ranked by Value per Unit of Work

| # | Use | Cost | Value | Status |
|---|---|---|---|---|
| 1 | **Commentary inference** | A feature flag and a `Device` | High and immediate | **Done in 1.4** ([§7.4.1](07-face-llm-layer.md#741-device-selection)) |
| 2 | **Offline value-network training** | A separate program, no engine change | High, and unlocks [§19.1](#191-neural-evaluator-integration) | Next |
| 3 | **Neural evaluation inside MCTS** | Batched search redesign | High, but only once a network exists | Last, and deliberately |

Conflating these three is how a roadmap item becomes a rewrite. The costs differ by orders of magnitude and the ordering is not negotiable: #3 is worthless without #2, and #2 is worthless without training data that [§14](14-sampling-strategy.md) is already generating.

### 19.6.2 Commentary Inference — Done

The seam did what it was built to do. [§7.4](07-face-llm-layer.md#74-candle-inference-runtime--replaces-the-ollama-rest-adapter) constructed `Device::Cpu` in exactly one place; that place is now `select_device` ([§7.4.1](07-face-llm-layer.md#741-device-selection)), CUDA is behind a `cuda` cargo feature, and the quantized loaders take the device as a parameter. No caller changed. No other component learned that a GPU exists.

What it bought, on this host:

| | CPU (2 cores, ~15 GB/s) | CUDA (RTX 3050, ~168 GB/s) |
|---|---|---|
| Default model | Qwen2.5-0.5B-Instruct Q4_K_M | **Qwen2.5-1.5B-Instruct Q4_K_M** |
| 64 tokens | ~1.7 s | **~0.8–1.5 s** |
| Lab workers available | 8 | **10** |
| Binding constraint | Memory bandwidth | ~5.0 GB of usable VRAM |

Two consequences, both larger than they look:

- **The deadline is met with a model worth listening to.** At 1.3's numbers, re-derived for this host, the CPU path could only meet `deadline_ms = 2500` by dropping to 0.5B — a model that repeats itself and loses its persona. The card buys back a size class.
- **[§15.4](15-concurrency-model.md#154-cpu-partitioning)'s inference reservation returned to the engine.** Enabling the GPU made *search* faster as a side effect, not only commentary. That is why the lab worker count fell from 16 to 10 rather than to 8, on a host with two fewer cores than the one 1.3 assumed.

6 GB of VRAM is the new binding constraint, and it caps the model at the 1.5B–3B class once the desktop session's share is deducted. That is a better constraint than the one it replaces, and it is budgeted in [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14).

### 19.6.3 Offline Value-Network Training — Architecturally Free

This is the use the Training Lab exists to serve, and it costs the running system nothing, because it is not part of the running system. A training job reads sampled positions and edges through the export endpoint ([§9.12](09-api-contract.md#912-optional-export-endpoint)) or directly from a read-pool connection, and trains a value or policy/value network on the GPU in its own process, on its own schedule, possibly on another machine entirely.

Nothing in [§11](11-database-architecture.md), [§12](12-database-schema.md) or [§15](15-concurrency-model.md) changes. The writer actor keeps its single connection; the trainer is just another reader. This is the payoff for the sampling density decisions in [§14](14-sampling-strategy.md), and it is available the moment there is data worth training on.

A 6 GB card is comfortable here. A draughts value network operating on a 32-square bitboard is small by any modern standard; the constraint is dataset size and epochs, not VRAM. One scheduling note specific to this host: the trainer and the running lab **compete for the same 6 GB and the same two memory channels**. Train while the lab is idle, or set `face.device = "cpu"` for the duration and accept the smaller commentary model. Running both flat out will make each slower than running them in sequence.

### 19.6.4 Neural Evaluation Inside MCTS — The Expensive One

This is the use that sounds cheapest and is not, and 1.4 does not make it any cheaper. [§6.2](06-mcts-extensibility.md#62-evaluation-strategy-trait)'s `EvaluationStrategy` trait makes a `NeuralEvaluator` a drop-in at the type level ([§19.1](#191-neural-evaluator-integration)), and it would be — running on the CPU. Moving it to the GPU breaks an assumption the search is built on.

`estimate_leaf_value` evaluates **one position at a time**. A GPU is throughput hardware: a batch of one wastes essentially the entire device, and per-call host-to-device transfer overhead can exceed the CPU cost of the same forward pass. Making the GPU worthwhile requires batches in the hundreds, and that requires the search to have hundreds of leaves outstanding at once — which means:

- **Leaf-parallel MCTS with virtual loss**, so multiple selections can descend before any of them is evaluated.
- **An evaluation queue** between the workers and the device, with a batch-formation deadline.
- **Asynchronous backpropagation**, since a worker no longer receives its value inline.

That is a redesign of the search loop, not a new evaluator. It also interacts with the transposition table ([§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11)) in a way that needs thought: virtual loss changes what a concurrent probe means, and batching changes when a store becomes visible.

**And on this host it has a fourth cost the 1.3 text did not have to state.** The Face layer now occupies the device. A batched evaluator sharing 6 GB with a resident commentary model, a CUDA context, and a desktop session is a VRAM-budget conversation before it is a search conversation ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) leaves ~3.2 GB), and the honest answer may be that the two cannot both be resident — in which case enabling a neural evaluator means disabling GPU commentary, and that trade should be made explicitly rather than discovered.

None of this is a reason to avoid it. It is a reason to schedule it as its own piece of work, after a network exists and is known to be worth the trouble.

### 19.6.5 What the MVP Must Preserve

Three properties kept the door open at no cost through 1.3, and use #1 walked through it without touching anything else. All three still hold and all three are now load-bearing rather than aspirational:

1. **`Device` is constructed in exactly one place** — `select_device` in [§7.4.1](07-face-llm-layer.md#741-device-selection). A second `Device::Cpu` or `Device::new_cuda` anywhere in the tree is a review-blocking defect, because it is what turns the next device change from a one-line edit into a search-and-replace.
2. **The evaluator is a trait, and `is_position_pure()` already distinguishes cacheable outputs** ([§6.2](06-mcts-extensibility.md#62-evaluation-strategy-trait)). A neural evaluator returns `true`, so it gets the full transposition table even in `Deterministic` mode — the case where a 24 GB cache pays for itself most, because a forward pass costs orders of magnitude more than a rollout.
3. **CUDA is a cargo feature, never a runtime assumption.** The default build has no CUDA dependency, `cargo build --release` works on a machine with no driver, and [§20](20-testing-strategy.md) runs entirely on CPU. A GPU that is absent, busy, or out of memory degrades to the CPU path, not to an error — the same discipline [§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11) applies to the model itself.

### 19.6.6 What CUDA Is Not For — New in 1.4

Now that the project has a working CUDA path, it acquires a new failure mode: somebody reaching for the device because it is there. Three things are explicitly not GPU work in this system, and each is recorded with the reason so the question does not have to be re-litigated.

| Not a GPU workload | Why |
|---|---|
| **Random-rollout MCTS** | Rollouts are branch-heavy, pointer-chasing, and serially dependent — the worst possible shape for a wide SIMD device. A rollout is also microseconds long, so per-call launch overhead dominates. Fourteen CPU cores beat this card at this workload, decisively |
| **Move generation** | Bitboard move-gen is already a handful of instructions per position. There is nothing to accelerate and a PCIe round trip to not do it in |
| **Transposition table probes** | The table is a 21 GB host-memory hash. It could not fit in 6 GB of VRAM, and a probe is a dependent random read — pure latency, which is what GPUs are worst at |

The pattern is consistent: this device is worth using for **dense, batchable, bandwidth-bound tensor math** and worth avoiding for everything else in this codebase. Commentary inference is the first workload of that shape, and a batched neural evaluator would be the second.

---

← [18. Security and Safety](18-security-and-safety.md) · **[Index](README.md)** · [20. Testing Strategy](20-testing-strategy.md) →
