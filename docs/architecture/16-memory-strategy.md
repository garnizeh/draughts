# 16. Hardware and Memory Strategy

This section replaces v1.0's *Constrained Hardware Strategy* and re-derives v1.1's *High-Memory* budget for the host that actually exists. The intent has not changed since 1.1 — spend the machine on purpose, with a stated ceiling for every consumer — but the machine has: **64 GB, 14 cores on two memory channels, and a 6 GB CUDA device** ([§2.4](02-scope-and-constraints.md#24-hardware-baseline)).

An unbudgeted process on a large machine does not use the memory well; it uses it accidentally, and then dies at the worst possible moment. That was true at 128 GB and it is twice as true at 64.

---

## 16.1 Memory Budget

**Host RAM — 64 GB.** VRAM is a separate budget on a separate device and is in [§16.6](#166-vram-budget--new-in-14); the two are never added together.

| Consumer | Budget | Enforcement |
|---|---:|---|
| OS, page cache, filesystem headroom | 8 GB | Left free by policy |
| SQLite writer page cache | 4 GB | `PRAGMA cache_size = -4194304` |
| SQLite read pool (6 conns × 256 MB) | 1.5 GB | `PRAGMA cache_size = -262144` per reader |
| **Transposition table** | **24 GB** | Hard entry cap, [§16.3](#163-transposition-table-sizing) |
| MCTS worker trees (10 × 512 MB) | 5 GB | Arena cap per worker; search aborts a node rather than growing past it |
| **MPSC write buffer** | **2 GB** | `channel_capacity` validated at startup, [§15.2.1](15-concurrency-model.md#1521-channel) |
| **Candle host-side** | **2 GB** | Tokenizer, staging, and the CPU-profile weights when the fallback path is live, [§7.5.6](07-face-llm-layer.md#756-memory) |
| Rust runtime, allocator overhead, fragmentation | 4 GB | Observed, not enforced |
| **Committed total** | **50.5 GB** | |
| **Reserve** | **~13.5 GB** | Deliberately unallocated |

`limits.max_total_memory_gb` is **56**, validated at startup ([§23](23-configuration-example.md)).

### 16.1.1 What Moved, and What Did Not Move Proportionally

Halving the host did not halve every row, and the exceptions are the interesting part:

| Consumer | 1.3 | 1.4 | Ratio | Why |
|---|---:|---:|---:|---|
| Transposition table | 32 GB | 24 GB | 0.75 | Cut least. It is the only consumer that converts each additional gigabyte directly into search throughput |
| OS / page cache | 16 GB | 8 GB | 0.50 | Straight halving; it scales with the host |
| SQLite writer cache | 8 GB | 4 GB | 0.50 | Straight halving |
| MCTS arenas | 8 GB | 5 GB | 0.63 | Tracks the worker count (16 → 10), not the host size |
| MPSC buffer | 4 GB | 2 GB | 0.50 | Straight halving; it also has less write throughput to absorb |
| Candle | 8 GB | 2 GB | 0.25 | **Cut most.** The weights moved to VRAM; what remains is staging and the CPU fallback |
| Reserve | 42 GB | 13.5 GB | 0.32 | Cut hardest in absolute terms, but held at ~21 % of the host — see below |

The Face row's collapse is the one worth stating plainly, because it is easy to misread as a saving. It is not: those 6 GB did not go to the transposition table, which lost 8 GB of its own. They went to the reserve, which is where a halved host most needs them.

**The reserve is not slack to be reclaimed later.** It absorbs page-cache growth, transient allocation spikes during index maintenance, and the fact that every figure above is an estimate. At 128 GB a 42 GB reserve was generous. At 64 GB, 13.5 GB is the *minimum* defensible figure — roughly a fifth of the host — and it is the row to defend first when a future revision wants memory for something new. A budget with no reserve is a budget that has already been exceeded.

The `mmap_size` of 8 GB is not in this table. It is a virtual mapping served by the OS page cache — it consumes address space and competes for the same physical pages as the 8 GB OS reservation, but it is not private process memory and must not be double-counted. It fell from 32 GB in 1.3 for the reason in [§11.1](11-database-architecture.md#111-sqlite-runtime-configuration): a window that was free on a 128 GB host is not free on this one.

---

## 16.2 Engine Budgets

Play Mode:

```toml
[engine.play]
iterations         = 4000
time_budget_ms     = 1500        # the binding constraint on this host, see below
worker_threads     = 1
transposition_mode = "deterministic"
```

Lab Mode:

```toml
[engine.lab]
iterations         = 800
time_budget_ms     = 0           # 0 = iteration-bounded only; deterministic by construction
worker_threads     = 10          # was 16; §15.4
transposition_mode = "throughput"
```

Iteration counts are unchanged from 1.1. They are a *strength* setting, not a hardware setting, and the transposition table is what makes them affordable.

What did change is which of Play Mode's two budgets actually binds. On a 2.6 GHz Broadwell core, 4 000 iterations is unlikely to complete inside 1 500 ms from a mid-game position, so **`time_budget_ms` is the operative limit and `iterations` is a ceiling** — the reverse of the assumption in 1.1. That is a legitimate configuration and it keeps move latency predictable, which is what a human player notices; it is recorded here so nobody reads a p99 of 1 500 ms as a regression against a target of 4 000 iterations. [§20.9](20-testing-strategy.md#209-performance-regression-baselines) measures completed iterations per move so the two can be told apart.

Lab mode has no time budget at all. An iteration-bounded search is reproducible; a time-bounded one is a function of machine load, and a batch whose strength varies with what else the machine was doing produces training data with an invisible confound.

---

## 16.3 Transposition Table Sizing

```text
TtEntry, packed:
    board            16 bytes
    side_to_move      1
    value             4
    samples           4
    moves (SmallMoveList, inline)  24
    kind              1
    evaluator         8
    epoch             2
    padding/align     4
    ---------------------------
    entry            ~64 bytes

DashMap overhead (hash, bucket, shard bookkeeping): ~30%
    effective per entry: ~83 bytes

24 GB budget / 83 bytes  =  ~310,000,000 entries
Configured cap (with headroom):  256,000,000 entries  ->  ~21.2 GB
```

```toml
[engine.transposition]
enabled                = true
capacity_entries       = 256000000
shard_count            = 512         # power of two; ~50x the worker count
reset_between_batches  = false
retire_batch_size      = 65536       # entries per retirement sweep
```

Sizing guidance:

- `shard_count` should be a power of two and comfortably exceed the worker count. 512 shards with 10 workers puts expected shard contention near zero; going higher wastes memory on bookkeeping without measurably reducing contention. (1024 was correct at 16 workers and is not wrong now — it is simply paying for headroom that no longer exists.)
- `reset_between_batches = false` is correct when consecutive batches share an evaluator and rules — the endgame knowledge accumulated by one batch is directly useful to the next. Set it to `true` when changing evaluators, though `EvaluatorIdentity` already makes stale entries unreadable rather than dangerous.
- Watch `hit_rate` over a batch's life. A rate that climbs and plateaus is healthy. A rate that climbs and then decays means the table is thrashing at capacity, and the fix is more entries or fewer stored estimates, not a larger machine.

### 16.3.1 Bandwidth, Not Just Capacity — New in 1.4

On the 1.3 host, table sizing was purely a memory-capacity question. On this one it is also a **bandwidth** question, and that changes the tuning advice.

A 21 GB hash table has no useful locality: every probe is a likely TLB miss followed by a DRAM round trip, and ten workers issue them concurrently against **two populated memory channels** ([§2.4](02-scope-and-constraints.md#24-hardware-baseline)). The table is still overwhelmingly worth it — a DRAM round trip costs on the order of 100 ns and the rollout it replaces costs microseconds — but three consequences follow:

1. **Hit rate is worth more than capacity here.** A larger table that raises `hit_rate` from 0.70 to 0.72 buys less than a sampling change that raises it to 0.80, because both spend the same scarce resource on the probe path. Tune `hit_rate` first; raise `capacity_entries` only when the rate is decaying rather than plateauing.
2. **Transparent huge pages are worth enabling for this allocation.** A 21 GB region mapped with 4 KiB pages needs ~5.2 M TLB entries; the CPU has on the order of a thousand. THP (or an explicit `MADV_HUGEPAGE` on the table's backing allocation) cuts the miss rate by roughly 512×, and costs nothing. This is the cheapest available performance change in the whole document and it should be verified at startup, not assumed.
3. **Populating the other two memory channels would help the table as much as it helps inference** ([§2.4.1](02-scope-and-constraints.md#241-two-notes-for-whoever-operates-this-host)). It is one hardware change that lifts two separate ceilings.

---

## 16.4 Inference Controls

The controls differ by device, and conflating them is how a working configuration becomes a permanently open circuit.

**On CUDA (the default):**

- The VRAM budget is the constraint, not the core count. [§16.6](#166-vram-budget--new-in-14) is the authority.
- `warm_on_start = true` is more important than on CPU: it pays the host-to-device copy and the first kernel launch at boot rather than on a player's first taunt.
- `inference_threads` is inert. One thread submits work and waits on the device.
- Everything else — `max_tokens`, the deadline, the queue depth, the circuit breaker — is unchanged, because none of it was ever about the CPU.

**On CPU (the fallback):**

- Cap the inference thread pool ([§15.4](15-concurrency-model.md#154-cpu-partitioning)). This is the single most important control, and it costs the lab two workers.
- Use the CPU profile's model ([§7.5.3](07-face-llm-layer.md#753-the-cpu-ladder--the-fallback-profile)). A 1.5B model at 15 GB/s of effective bandwidth cannot meet a 2 500 ms deadline on this host.

**On both:**

- Limit `max_tokens` — 80 is generous for a taunt.
- Enforce a wall-clock deadline checked between tokens.
- Disable the Face during large lab batches (`lab_mode_enabled = false`, the default).
- Rely on the circuit breaker rather than on retries.

```toml
[face]
enabled           = true
provider          = "candle"
device            = "auto"       # cuda where available, cpu otherwise; §7.4.1
deadline_ms       = 2500
max_tokens        = 80
max_queue_depth   = 2
inference_threads = 2            # CPU path only; inert on CUDA
lab_mode_enabled  = false
warm_on_start     = true

[face.cuda_profile]
model_path        = "./models/qwen2.5-1.5b-instruct-q4_k_m.gguf"

[face.cpu_profile]
model_path        = "./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
```

A deployment that cares more about quality than latency can raise the CUDA profile toward the 6 GB VRAM ceiling and raise `deadline_ms` with it, relying on the decoupled commentary pane ([§10.3](10-frontend-architecture.md#103-htmx-interaction-pattern)). Both are configuration changes, and neither is safe without moving the deadline with the model.

---

## 16.5 Storage Controls

- Sampling remains mandatory for lab mode, at the 1.1 defaults ([§14.2](14-sampling-strategy.md#142-recommended-defaults)) — unchanged by the 1.4 re-baseline, because halving RAM changes what is cached, not what is written.
- Indexes kept minimal — index maintenance is the dominant cost inside a 50k-row transaction.
- Batch deletion by `batch_id`, which cascades.
- Optional archival export to JSONL before pruning.
- `PRAGMA optimize` at batch completion and on clean shutdown.
- `PRAGMA wal_checkpoint(TRUNCATE)` at batch boundaries, so the WAL does not grow without bound across a multi-day run.
- Monitor `db_size_mb` per batch and prune oldest completed batches by policy, not by emergency.

Disk is now the resource that a long campaign consumes fastest: at the 1.4 throughput target the default profile writes roughly 18 GB per day of continuous lab time ([§14.2](14-sampling-strategy.md#142-recommended-defaults)). Provision for it, and keep the reserve in [§16.1](#161-memory-budget) intact — a full disk makes the system read-only ([§17.3](17-reliability.md#173-database-failures)), not dead, but only if there is memory left to report the fact.

---

## 16.6 VRAM Budget — New in 1.4

The device is an **NVIDIA GeForce RTX 3050 with 6 GB of GDDR6**. This is a budget on a separate device, allocated by a separate allocator, and it must never be added to [§16.1](#161-memory-budget) or subtracted from it. Host RAM and VRAM are two ceilings, and exceeding either one is fatal in its own way.

| Consumer | Budget | Notes |
|---|---:|---|
| Desktop session / display output | ~1.0 GB | **Observed, not controlled.** The card drives a display; this is not ours to spend |
| Quantized model weights | 1.0 GB | Qwen2.5-1.5B-Instruct Q4_K_M ([§7.5.2](07-face-llm-layer.md#752-the-cuda-ladder--the-default-profile)) |
| KV cache | 0.3 GB | Reset between requests; bounded by `max_tokens` and prompt length |
| CUDA context, cuBLAS workspace, allocator slack | 0.5 GB | Fixed cost of having a context at all |
| **Face layer total** | **1.8 GB** | Against a 4.5 GB cap |
| **Usable VRAM** | **~5.0 GB** | 6.0 nameplate less the desktop's share |
| **Headroom** | **~3.2 GB** | |

Three rules keep this budget from being decorative:

1. **Budget against ~5.0 GB usable, never 6.0 GB nameplate.** Roughly 1 GB is held by the desktop session before the process starts. A model sized against the nameplate figure loads successfully and then fails on its first long generation — a strictly worse failure than refusing to load ([§2.4.1](02-scope-and-constraints.md#241-two-notes-for-whoever-operates-this-host)).
2. **VRAM exhaustion is a Face failure, never a process failure.** A CUDA OOM at load leaves the breaker permanently open and the service on canned lines ([§7.4.1](07-face-llm-layer.md#741-device-selection), [§17.2](17-reliability.md#172-face-failures)). A CUDA OOM mid-generation is a `FaceError::Inference` and a breaker failure. Neither takes the game down; both are visible on `/health` as `vram_used_mb` against `vram_budget_mb`.
3. **The headroom is not spare capacity for a bigger model.** It absorbs the desktop session growing, another process claiming the card, and fragmentation in the CUDA allocator across a multi-day run. Qwen2.5-7B Q4_K_M would fit in the arithmetic (~5.2 GB) and does not fit in reality — it is listed in [§7.5.2](07-face-llm-layer.md#752-the-cuda-ladder--the-default-profile) as a headless-card option for exactly that reason.

**Nothing else in the process may allocate VRAM.** The engine, the transposition table, the rules core and the writer are CPU-only for the whole MVP ([§15.4.2](15-concurrency-model.md#1542-the-gpu-is-not-in-this-table)). If a future revision puts a neural evaluator on the device ([§19.6.4](19-extensibility-roadmap.md#1964-neural-evaluation-inside-mcts--the-expensive-one)), it inherits this table and this ceiling, and 3.2 GB of headroom for weights, activations and an evaluation batch is the number that work has to start from.

---

← [15. Concurrency Model](15-concurrency-model.md) · **[Index](README.md)** · [17. Reliability and Failure Handling](17-reliability.md) →
