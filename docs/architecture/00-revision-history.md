# 0. Revision History

| Version | Date | Summary |
|---|---|---|
| 1.0 | — | Initial proposed MVP architecture. Constrained-hardware assumptions, Ollama REST for commentary, conservative SQLite tuning, per-search MCTS trees with no shared cache. |
| 1.1 | — | Re-baselined for a 128 GB host. Six architectural shifts, listed below. |
| 1.2 | — | Editorial and consistency revision. No architectural change. Project named **Draughts**; the document is split into per-section files with an index; six corrections applied, listed in [§0.2](#02-what-changed-in-12); Mermaid diagrams added throughout. |
| 1.3 | — | Face model re-baselined on the 1–2 core inference budget that §15.4 always specified. The default drops from an 8B class model to Qwen2.5-1.5B-Instruct; the Face memory budget drops from 16 GB to 8 GB and the released 8 GB goes to the transposition table. The GGUF loader is resolved against `candle-transformers` 0.11 and made architecture-dispatching, and GPU acceleration is put in scope as recorded intent (§19.6). See §0.3. |
| 1.4 | — | Re-baselined onto the **actual target host**: 64 GB (not 128), 14 physical cores (not 16+), two populated memory channels (not four), and an **RTX 3050 with 6 GB of VRAM and a working CUDA stack**. Every memory figure is halved or re-derived; CUDA moves from roadmap intent to the **default device for the Face layer**, because the measured bandwidth of this host makes the CPU path miss its own deadline. See §0.4. |
| 1.5 | Current | States the **draw rules** [§5.3](05-runtime-components.md#53-game-rules-core) has always required and no section ever gave: the ACF 40-move non-progress rule and three-fold repetition, both **adjudicated by the game loop rather than by `apply_move`**, so that terminality stays a pure function of the position and [§20.5](20-testing-strategy.md#205-transposition-table-tests)'s guarantee survives by construction rather than by care. Both are configuration, with the English-draughts values as defaults. See §0.5. |

## 0.1 What Changed in 1.1

| # | Shift | Rationale | Primary Sections |
|---|---|---|---|
| 1 | **Global lock-free transposition table** (`DashMap`) shared by all MCTS workers | Self-play revisits the same positions constantly. Memory is no longer the scarce resource; recomputation is. | [§5.4](05-runtime-components.md#54-mcts-engine), [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11), [§16.3](16-memory-strategy.md#163-transposition-table-sizing) |
| 2 | **High-memory SQLite configuration** — 8 GB page cache, `temp_store = MEMORY`, 32 GB mmap window | The working set of a multi-hundred-GB lab database can now live in RAM. Disk I/O stops being the bottleneck for reads and index maintenance. | [§11.1](11-database-architecture.md#111-sqlite-runtime-configuration), [§16.1](16-memory-strategy.md#161-memory-budget) |
| 3 | **MPSC actor pattern for all database writes** — a bounded channel buffering hundreds of thousands of messages, drained by one dedicated writer thread committing 50k+ rows per transaction | SQLite permits exactly one writer. Rather than fight it, the architecture makes that single writer maximally efficient and absorbs burst load in RAM. | [§11.2](11-database-architecture.md#112-write-strategy--mpsc-actor-pattern), [§15.2](15-concurrency-model.md#152-the-write-path--mpsc-actor-detail), [§17.3](17-reliability.md#173-database-failures) |
| 4 | **Ollama removed; Hugging Face Candle embedded in-process** — the binary loads a quantized `.gguf` model directly into its own address space | Eliminates an external daemon, a REST hop, a serialization boundary, and an independent failure domain. 128 GB permits 8B–14B parameter models where v1.0 assumed 0.5B–1.5B. Deployment becomes a single executable. | [§3](03-system-context.md), [§5.7](05-runtime-components.md#57-face--llm-adapter--in-process-candle-runtime), [§7.4](07-face-llm-layer.md#74-candle-inference-runtime--replaces-the-ollama-rest-adapter), [§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget), [§22](22-deployment-model.md) |
| 5 | **Circuit breaker on the Face layer** — 3 consecutive failures trip the circuit open for 5 minutes, routing all commentary to the canned fallback | In-process inference shares the CPU with the engine. A degraded model must never be allowed to degrade gameplay, and retrying a failing subsystem per-move is a self-inflicted denial of service. | [§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11), [§17.2](17-reliability.md#172-face-failures) |
| 6 | **`format_version` column on `games` and `positions`** | The board and move-history BLOBs are deliberately compact and therefore brittle. Every row now carries the version of the encoding that produced it. | [§12](12-database-schema.md), [§13.7](13-data-dictionary.md#137-format_version--new-in-11), [§19.5](19-extensibility-roadmap.md#195-format-version-evolution) |

Numbering, section order, and the v1.0 contracts that were not affected are preserved deliberately, so a 1.0 → 1.1 diff is reviewable section by section.

---

## 0.2 What Changed in 1.2

Version 1.2 is an editorial revision. **No architectural decision was reopened, no contract was redesigned, and nothing was removed or condensed.** Section numbering is unchanged, so a 1.1 → 1.2 diff is reviewable section by section exactly as a 1.0 → 1.1 diff was.

### 0.2.1 Naming

The project is named **Draughts**. Prose, the executable name, the configuration file, the database file, and the on-disk layout follow it.

| Artifact | 1.1 | 1.2 |
|---|---|---|
| Document title | AI-Augmented Checkers MVP | Draughts — AI-Augmented Draughts Engine and Training Lab |
| Executable | `./checkers` | `./draughts` |
| Configuration file | `checkers.toml` | `draughts.toml` |
| Database file | `./checkers.db` | `./data/draughts.db` |
| Deployment directory | `checkers/` | `draughts/` |
| Ruleset identifier | `american_checkers` | `english_draughts` |

The ruleset identifier names the same variant it always did — the 8x8 game with mandatory captures and non-flying kings, known as English draughts, American checkers, or straight checkers. It appears in `games.rules`, in the match-creation request ([§9.3](09-api-contract.md#93-start-human-vs-cpu-match)), and in the batch-creation request ([§9.8](09-api-contract.md#98-start-training-lab-batch)). No code exists yet, so this is a rename inside a specification rather than a data migration.

### 0.2.2 Corrections

Eight defects were found while re-reading the document, and each is corrected in place. They are listed here so a reader holding a 1.1 copy knows exactly what moved and why.

| # | Section | Defect in 1.1 | Correction |
|---|---|---|---|
| 1 | [§0](#), [§26](26-summary.md) | The revision history said "Five architectural shifts" above a table of six, and [§26](26-summary.md) repeated the count. | The count reads six. [§26](26-summary.md) says "every one of those shifts" rather than restating a number that has to be kept in sync. |
| 2 | [§19.1](19-extensibility-roadmap.md#191-neural-evaluator-integration), [§22.2](22-deployment-model.md#222-optional-process-separation) | Both described the removal of the Ollama process boundary as "shift 3". Shift 3 is the MPSC writer actor; shift 4 is the Ollama removal. | Both read "shift 4". |
| 3 | [§9.2](09-api-contract.md#92-health-endpoint), [§9.8](09-api-contract.md#98-start-training-lab-batch), [§9.10](09-api-contract.md#910-get-lab-batch-status) | The API examples used `capacity_entries: 384000000`. At the ~83-byte effective entry size derived in [§16.3](16-memory-strategy.md#163-transposition-table-sizing) that is 31.9 GB — over the 24 GB budget in [§16.1](16-memory-strategy.md#161-memory-budget) and above the 256 000 000 cap that [§16.3](16-memory-strategy.md#163-transposition-table-sizing) and [§23](23-configuration-example.md) both specify. | The examples read `256000000`, matching [§16.3](16-memory-strategy.md#163-transposition-table-sizing) and [§23](23-configuration-example.md) as they stood in 1.2. **Superseded in 1.3**: the table budget rises to 32 GB and the cap returns to `384000000`, within budget this time. See §0.3. |
| 4 | [§11.2.3](11-database-architecture.md#1123-the-writer-loop) | The writer loop read `buf.was_shutdown()` after `buf.clear()`, so a `Shutdown` message would have its flag cleared before it was tested and the actor would never stop. | The flag is captured into a local before the buffer is cleared. |
| 5 | [§12](12-database-schema.md) | `positions.batch_id` was `NOT NULL`, which makes `sample_kind = 2` (`human_game_sample`) unrepresentable: a human match belongs to no batch, which is why `games.batch_id` is nullable. | `positions.batch_id` is nullable and carries the same foreign key `games.batch_id` does. |
| 6 | [§12.1](12-database-schema.md#121-migration-from-a-v10-database) | The migration issued `PRAGMA foreign_keys = OFF` between `BEGIN IMMEDIATE` and `COMMIT`. SQLite documents that pragma as a no-op inside a transaction, so enforcement would have stayed on and `DROP TABLE lab_batches` would have failed. | The pragma is toggled outside the transaction, and `PRAGMA foreign_key_check` runs after it is restored. |
| 7 | [§15](15-concurrency-model.md) | The thread table sized the MCTS worker pool at `physical_cores - 2`, which is 14 on the 16-core host that [§15.4](15-concurrency-model.md#154-cpu-partitioning) partitions as 12 lab workers plus 1 reserved Play Mode core. | `physical_cores - 3`, with a pointer to [§15.4](15-concurrency-model.md#154-cpu-partitioning), which remains the authority. |
| 8 | [§23](23-configuration-example.md) | `[database] path = "./checkers.db"` contradicted the [§22.1](22-deployment-model.md#221-mvp-single-machine-deployment) on-disk layout, which places the database under `data/`. | `path = "./data/draughts.db"`. |

### 0.2.3 Structure and Diagrams

- The single document is split into one file per top-level section, plus an index. Every section keeps its number, so `[§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11)` is still `[§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11)` and now also resolves to a link.
- Cross-references are rendered as links. A reference to a subsection that has no heading of its own — `[§7.7.5](07-face-llm-layer.md#77-commentary-guardrails)`, for instance, which is an item in a numbered list — links to its nearest enclosing heading.
- Mermaid diagrams are added wherever the document describes a graph, a flow, a state machine, a sequence, or a schema. Where an ASCII block carried information a diagram cannot — a byte layout, a sizing calculation, a memory-budget tree — the ASCII is kept and a diagram is added beside it rather than replacing it.

---

## 0.3 What Changed in 1.3

Version 1.3 changes one decision and its consequences. It is not a restructuring, and it reopens nothing else.

### 0.3.1 The Defect

§7.5 quoted a commentary latency of 1.5–2.5 s for an 8B Q5_K_M model and made it the recommended default. §15.4 caps the inference pool at **1–2 cores**. Those two numbers were never reconciled, and they do not reconcile.

CPU token generation is memory-bandwidth-bound: every decoded token reads the entire weight set once. Two cores sustain roughly 30–40 GB/s of effective read bandwidth no matter how many memory channels the host has, because two cores cannot saturate a large memory subsystem on their own. At ~5.6 GB resident, an 8B Q5_K_M model therefore produces on the order of 6–7 tokens per second, and a 64-token taunt takes about ten seconds — **four times its own `deadline_ms = 2500`**.

The consequence was not a slow taunt. It was a dead Face layer: every request exceeds the deadline, every deadline miss counts toward the circuit breaker (§7.8.3), and three consecutive misses open the circuit for five minutes. The system would have served canned lines permanently while reporting a model that was loaded and, by its own `/health` output, healthy.

This is precisely the failure §7.8 exists to contain, which is why it would have been survivable — and precisely why it could have gone unnoticed for a long time.

### 0.3.2 The Change

| | 1.2 | 1.3 |
|---|---|---|
| Default model | 8B instruct, Q5_K_M, ~5.6 GB | **Qwen2.5-1.5B-Instruct, Q4_K_M, ~1.0 GB** |
| Estimated latency, 64 tokens, 2 cores | ~10 s, stated as 1.5–2.5 s | ~1.8 s |
| Quality profile | 14B Q6_K | Qwen2.5-7B-Instruct Q5_K_M, with `deadline_ms >= 12000` |
| Floor profile | 1.5B | Qwen2.5-0.5B-Instruct |
| Face memory budget | 16 GB | **8 GB** |
| Transposition table budget | 24 GB | **32 GB** |
| `capacity_entries` | 256 000 000 | **384 000 000** |
| Committed total | 86 GB | 86 GB — unchanged |

The memory move is a straight swap. The Face layer never needed 16 GB once the core cap is taken seriously: the binding constraint is bandwidth, and a model small enough to meet the deadline fits in 8 GB many times over. The transposition table is bounded by memory in exactly the way the Face layer is not, and converts each additional gigabyte directly into search throughput. Moving 8 GB from the consumer that cannot use it to the one that can leaves the committed total at 86 GB and `limits.max_total_memory_gb` at 96, so nothing downstream of the budget moves.

### 0.3.3 Model Selection Is Now Derived, Not Asserted

§7.5 previously presented sizes and latencies with no stated derivation, which is how its numbers drifted out of agreement with §15.4 without anyone noticing. It now states the bandwidth relation the estimates come from, so a reader can re-derive them for a different host, a different core cap, or a different quantization instead of waiting for a measurement to contradict them.

Licensing is promoted to a selection criterion in the same section. The recommended default is the one people will actually ship, and this project is MIT: Qwen2.5 at 0.5B, 1.5B, 7B and 14B is Apache 2.0, while **Qwen2.5-3B is not** — it carries the Qwen Research License. That is why the documented ladder steps from 1.5B straight to 7B rather than through the obvious intermediate.

### 0.3.4 The Loader Defect, and Its Resolution

v1.1 pinned `candle_transformers::models::quantized_llama::ModelWeights` and, four subsections later, recommended a `qwen2.5-*.gguf` file. Those cannot both be right: `quantized_llama` reads GGUF metadata under the `llama.*` key prefix, and a Qwen2.5 file declares `general.architecture = "qwen2"`. The loader would not have loaded the recommended model.

Checked against `candle-transformers = "0.11"`, which ships **22 quantized loaders** including `quantized_qwen2`, `quantized_qwen3`, `quantized_gemma3`, `quantized_phi3` and `quantized_mistral`. So the model family does not have to change — but resolving it exposed a second, larger problem.

Every family exposes its own `ModelWeights` type. They are structurally identical and share no trait, so pinning any single loader at compile time would make §7.5's stated promise — that swapping models is a config edit and a restart, not a rebuild — **true only within one architecture**. That is a materially weaker guarantee than the one the document made.

§7.4 therefore dispatches on the file's own `general.architecture` through a `LoadedModel` enum, with `Qwen2` and `Llama` wired up and every other family one variant away. This makes the promise true across families, gives the load-time architecture check a natural home rather than a bolted-on assertion, and keeps the supported set visible in a single `match`.

### 0.3.5 GPU Put in Scope, as Intent

The MVP remains CPU-only and every component must stay correct with no accelerator present. What changes is that the *intention* is now recorded with a target device rather than left as an implied someday: the development host carries an RTX 3050 (stated as 8 GB in 1.3; it is **6 GB** — corrected in 1.4, see [§2.4](02-scope-and-constraints.md#24-hardware-baseline)), §2.4 states that no component may require a GPU, and the new **§19.6** sets out what one would be used for.

The important content of §19.6 is that "GPU support" is not one feature but three, with costs that differ by orders of magnitude:

| Use | Cost | When |
|---|---|---|
| Commentary inference | A cargo feature and a `Device` — the seam already exists in §7.4 | First. It also returns §15.4's reserved cores to the engine |
| Offline value-network training | A separate program reading through the export path; no engine change at all | Second |
| Neural evaluation inside MCTS | Leaf-parallel search, virtual loss, an evaluation queue, asynchronous backpropagation | Last, and deliberately |

The third is the one that looks cheapest and is not. §6.2's trait makes a neural evaluator a drop-in *on the CPU*; putting it on a GPU means the search must keep hundreds of leaves outstanding to form a worthwhile batch, which is a redesign of the search loop rather than a new evaluator. Recording that now is the point of this subsection — the cost belongs in the roadmap, not in the commit that discovers it.


---

## 0.4 What Changed in 1.4

Versions 1.1 through 1.3 were written against an assumed host: 128 GB of RAM, sixteen or more cores, and no accelerator. That host does not exist. Version 1.4 replaces the assumption with the machine the project will actually be built and run on, and follows the consequences wherever they lead.

### 0.4.1 The Actual Host

| Resource | Assumed through 1.3 | Actual |
|---|---|---|
| RAM | 128 GB | **64 GB** — 2 × 32 GB DDR4-2400 ECC RDIMM (Samsung) |
| Memory channels populated | Implied four or more | **Two** (`DIMM_B1`, `DIMM_D1`), of the four the platform supports |
| Theoretical memory bandwidth | Implied 60–100 GB/s | **~38 GB/s** peak; ~25–30 GB/s achievable |
| CPU | "16+ physical cores" | **Intel Xeon E5-2690 v4** — 14 physical cores, 28 threads, 2.6 GHz base / 3.5 GHz turbo, 35 MB L3, Broadwell-EP, single socket, one NUMA node |
| GPU | "None required" | **NVIDIA GeForce RTX 3050, 6 GB GDDR6**, driver 595.84, CUDA 13.2, compute capability 8.6, ~1 GB already held by the desktop session |
| Storage | Local SSD/NVMe | Unchanged |

Three of those lines change decisions rather than just numbers.

### 0.4.2 Consequence One — The Memory Budget Halves, and the Table Absorbs Most of It

[§16.1](16-memory-strategy.md#161-memory-budget) is re-derived from 64 GB rather than rescaled from 128. Every consumer shrinks, but not proportionally: the OS reservation, the SQLite caches, and the worker arenas are cut roughly in half, while the transposition table — the one consumer that converts each additional gigabyte directly into search throughput — is cut by a third, from 32 GB to **24 GB**, and `capacity_entries` returns to **256 000 000**.

The `mmap_size` window drops from 32 GB to 8 GB for a related but distinct reason: on a 128 GB host a 32 GB virtual window costs nothing, because there is page cache to spare. On a 64 GB host with 24 GB already committed to a hash table, an oversized window competes for physical pages with the very reservation that is supposed to protect the OS.

Committed total: **50.5 GB**, with a ~13.5 GB reserve. `limits.max_total_memory_gb` becomes **56**.

### 0.4.3 Consequence Two — The CPU Inference Path Cannot Meet Its Own Deadline

This is the same class of defect §0.3 corrected, found the same way, and it is the reason 1.4 is not merely an arithmetic revision.

[§7.5](07-face-llm-layer.md#75-model-selection-and-memory-budget) derives the model ladder from one relation:

```text
tokens/sec  ≈  effective read bandwidth / resident weight bytes
```

and it assumed two cores could reach **35 GB/s**. On this host that is not merely optimistic, it is above the platform ceiling: two populated DDR4-2400 channels give ~38 GB/s *in total*, for every core and every device on the socket at once. Two Broadwell cores realistically see **12–18 GB/s**.

Re-running the relation at 15 GB/s:

| Model | Resident | 1.3 estimate (35 GB/s) | This host (15 GB/s) | Against `deadline_ms = 2500` |
|---|---|---|---|---|
| Qwen2.5-0.5B-Instruct Q4_K_M | ~0.4 GB | ~0.8 s | **~1.7 s** | Fits |
| Qwen2.5-1.5B-Instruct Q4_K_M | ~1.0 GB | ~1.8 s | **~4.3 s** | **Misses by 1.7×** |
| Qwen2.5-7B-Instruct Q5_K_M | ~5.4 GB | ~10 s | ~23 s | Never |

The 1.3 default would have produced exactly the failure §0.3.1 described — three consecutive deadline misses, an open circuit, and permanently canned commentary from a model reporting itself healthy — for the same reason, one revision later, on a different number.

### 0.4.4 Consequence Three — CUDA Stops Being a Roadmap Item

The host has a working CUDA 13.2 stack and an Ampere card with roughly **168 GB/s** of memory bandwidth — an order of magnitude more than two of its own CPU cores can reach. [§19.6.2](19-extensibility-roadmap.md#1962-commentary-inference--done) already argued that commentary inference on a GPU is close to free and should be done first. With the CPU path now failing its deadline at the default model, "first" means **now**.

So 1.4 makes CUDA the **default device for the Face layer on this host**, and restructures the layer around a device choice rather than a device assumption:

| | 1.3 | 1.4 |
|---|---|---|
| Face device | `Device::Cpu`, hard-coded | `face.device = "cuda"` \| `"cpu"` \| `"auto"`, resolved once at startup ([§7.4](07-face-llm-layer.md#74-candle-inference-runtime--replaces-the-ollama-rest-adapter)) |
| Default model, CUDA | — | Qwen2.5-1.5B-Instruct Q4_K_M, ~0.8–1.5 s for 64 tokens |
| Default model, CPU | Qwen2.5-1.5B-Instruct Q4_K_M | **Qwen2.5-0.5B-Instruct Q4_K_M** — the only size that meets the deadline on two cores of this host |
| Build | `cargo build --release` | Unchanged by default; `--features cuda` adds the CUDA path |
| Missing / broken GPU | n/a | Falls back to the CPU profile and logs it; never an error ([§7.4.1](07-face-llm-layer.md#741-device-selection)) |
| VRAM | Not budgeted | **A separate budget on a separate device** ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)) |

Three properties are non-negotiable and are what keep this from being a hardware lock-in:

1. **The default `cargo build --release` still has no CUDA dependency**, still compiles on a machine with no driver, and still passes the entire test suite ([§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests)).
2. **A GPU that is absent, busy, or out of VRAM degrades to the CPU path**, exactly as a failing model degrades to canned lines. It is never a hard failure.
3. **The engine never touches the GPU.** Rules, search, and the transposition table are CPU-only in the MVP and stay that way; [§19.6.4](19-extensibility-roadmap.md#1964-neural-evaluation-inside-mcts--the-expensive-one) is unchanged, and [§19.6.6](19-extensibility-roadmap.md#1966-what-cuda-is-not-for--new-in-14) now says plainly what CUDA is *not* for, so that nobody reaches for it to accelerate rollouts.

Moving inference off the CPU also returns [§15.4](15-concurrency-model.md#154-cpu-partitioning)'s reserved inference cores to the engine — which is why the worker count does not fall as far as the core count did.

### 0.4.5 Consequence Four — Throughput Targets Come Down

A Broadwell-EP core at 2.6 GHz is not a modern core, ten workers are not sixteen, and a two-channel memory subsystem is the binding constraint for a 24 GB random-access hash table. [Appendix B](appendix-b-performance-targets.md) is re-derived rather than copied forward: lab throughput drops from 1 500–2 500 games/min to **600–1 200**, and sustained write throughput from 150k–400k rows/s to **100k–250k**.

Recording a target that this host cannot reach would defeat the purpose of having targets at all — a build that hits 900 games/min should read as *on plan*, not as a 40 % miss against a number derived from somebody else's machine.

### 0.4.6 What Did Not Change

No structural decision was reopened. The seams held again, which is the second time a hardware re-baseline has landed entirely inside them:

- The `EvaluationStrategy` trait, the transposition table's two modes, the MPSC writer actor, the durability classes, the circuit breaker, and `format_version` are untouched.
- The GPU arrived behind `candle_core::Device`, the seam [§19.6.5](19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve) was explicitly holding open for it — one constructor, one config key, no caller changed.
- Section numbering is unchanged, so a 1.3 → 1.4 diff is reviewable section by section. [Appendix F](appendix-f-v13-to-v14-checklist.md) lists every value that moved.

---

## 0.5 What Changed in 1.5

Version 1.5 adds one normative subsection and the configuration surface that goes with it. No decision is reopened, no contract is redesigned, and section numbering is unchanged, so a 1.4 → 1.5 diff is reviewable section by section.

### 0.5.1 The Gap

[§5.3](05-runtime-components.md#53-game-rules-core) listed "draw rules for MVP" among the Rules Core's responsibilities. [§20.1](20-testing-strategy.md#201-rules-tests) required them tested. [§19.4](19-extensibility-roadmap.md#194-rule-variants) listed "draw thresholds" among the policies a post-MVP variant layer would make swappable, under the sentence "for MVP, only one variant is enabled" — which reads as though the MVP's variant had fixed, known thresholds a later release would expose. It did not have them. No section stated which draw rules, no section stated a threshold, and [§23](23-configuration-example.md) had no `[rules]` table.

Meanwhile every layer above the Rules Core was already written against a draw the Rules Core had no rule for: [§12](12-database-schema.md) and [§13](13-data-dictionary.md) encode `games.result = 3` as a draw, [§6](06-mcts-extensibility.md) has `GameResult::Draw` in the search's terminal enum and scores it `0.0`, and [§14](14-sampling-strategy.md) samples draws as an outcome class in `position_edges`.

This is the same class of defect as [§0.3](#03-what-changed-in-13) and [§0.4.3](#043-consequence-two--the-cpu-inference-path-cannot-meet-its-own-deadline): two parts of the document depending on a number that neither of them states. It was found the same way, by reading the sections that consume a value looking for the section that owns it.

### 0.5.2 What the Rules Are

Both in [§5.3.1](05-runtime-components.md#531-draw-rules-for-mvp--new-in-15), which is the only normative statement of them:

| | Rule | Default |
|---|---|---|
| Non-progress | Drawn after N plies with no capture and no man move; the counter resets on a capture **or** a man move, never on a king move | **80 plies** — 40 moves per side, the ACF rule |
| Repetition | Drawn when the same Zobrist key — which folds in the side to move — occurs for the Nth time since the last capture or promotion | **Three-fold**, counted since the last irreversible move |

The reset condition is the load-bearing half of the first rule. Men only move forward, so man moves are bounded; captures are bounded by the pieces on the board; so resets are bounded and every game, including a random playout, reaches the threshold in finite time. That is a termination proof, and it is what [§6.3](06-mcts-extensibility.md#63-random-rollout-evaluator)'s rollout never had — `max_playout_ply` was carrying it, and an arbitrary cutoff in a rollout biases every value the rollout produces.

### 0.5.3 The Decision That Was Not Obvious

Which layer adjudicates. [§20.5](20-testing-strategy.md#205-transposition-table-tests) requires the transposition table to change how long a search takes and never what it returns, and [§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11) caches terminal detection under a key derived from the Zobrist hash alone. Terminality must therefore be a pure function of `(board, side_to_move)`, and **neither draw rule is one** — repetition depends on the path, and the non-progress counter is state carried beside the board. A draw adjudicated inside `apply_move` would have made `Finished` path-dependent and let the table serve a proven draw for a position that is not drawn.

So the Rules Core keeps the counter and adjudicates neither rule; the game loop above it — the lab runner's per-worker loop and the Play Mode service — owns the key history and declares both draws. [§6.7.3](06-mcts-extensibility.md#673-probe-and-store) states it from the table's side. Three things follow: `TtKey` and `TtEntry` are untouched; `GameState` gains one `u32` and no `Vec`, which matters because it is cloned on every path the search walks ([§16.2](16-memory-strategy.md#162-engine-budgets)); and the search does not see repetition, which is an accepted and now-stated MVP limitation rather than a surprise.

Had this been settled in a pull request instead, all three would have been settled by accident.

### 0.5.4 Configuration Rather Than Constants

The four values are `[rules.draw]` keys ([§23](23-configuration-example.md)) with the English-draughts numbers as defaults, which turns the first of [§19.4](19-extensibility-roadmap.md#194-rule-variants)'s four dials on early at the cost of one table and four validated keys. [§23.1](23-configuration-example.md#231-startup-validation) refuses `non_progress_plies = 0` and `repetition_count < 2`, and warns once, naming keys, when the policy departs from the defaults — because `games.rules` records `english_draughts` for a game whatever thresholds it was played under.

One consequence reaches the engine: the rollout applies the non-progress rule, so the policy changes the distribution it samples and joins `EvaluatorIdentity` beside `max_playout_ply` ([§6.3](06-mcts-extensibility.md#63-random-rollout-evaluator)). That is the existing mechanism for exactly this problem, used rather than extended.

### 0.5.5 Two Fine Rules, Written Down

[§5.3.2](05-runtime-components.md#532-two-fine-rules-stated--new-in-15) states two English-draughts rules that [§2.1](02-scope-and-constraints.md#21-in-scope) settled by reference and that decide code in the move generator: a man crowned by a jump does not continue jumping, and a capture sequence must be completed but need not be maximal. Neither was in dispute; both were nowhere in the document, and the second is the kind of rule a move generator can get wrong while passing every test written by the person who got it wrong.

---

← · **[Index](README.md)** · [1. Executive Summary](01-executive-summary.md) →
