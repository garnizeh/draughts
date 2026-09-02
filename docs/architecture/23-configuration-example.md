# 23. Configuration Example

One file, one binary, no environment-specific code paths. Every value below is derived for the target host in [§2.4](02-scope-and-constraints.md#24-hardware-baseline) — 64 GB, 14 cores, RTX 3050 — and the section that owns each derivation is named in a comment.

```toml
[server]
host           = "127.0.0.1"
port           = 8080
static_dir     = "./static"
worker_threads = 4                   # Tokio runtime; HTTP only (was 8) — §15

[database]
path                     = "./data/draughts.db"
page_size                = 8192      # applied at creation only
writer_cache_mb          = 4096      # PRAGMA cache_size = -4194304 (was 8192)
reader_cache_mb          = 256       # per reader connection, NOT 4 GB each (was 512)
mmap_size_gb             = 8         # was 32; §11.1 explains why this is not a rescale
read_pool_size           = 6         # was 8
busy_timeout_ms          = 30000
checkpoint_every_commits = 64
journal_size_limit_mb    = 4096

[database.writer]
channel_capacity  = 262144           # ~2 GB RAM ceiling (was 524288) — §16.1
db_batch_rows     = 50000            # target rows per transaction
flush_interval_ms = 250              # commit even if the batch is not full
max_retries       = 5
retry_backoff_ms  = [10, 50, 250, 1000, 5000]

[engine.play]
evaluator            = "random_rollout"
iterations           = 4000          # a ceiling; time_budget_ms binds first here — §16.2
time_budget_ms       = 1500
exploration_constant = 1.4
worker_threads       = 1
transposition_mode   = "deterministic"

[engine.lab]
evaluator            = "random_rollout"
iterations           = 800
time_budget_ms       = 0             # 0 = iteration-bounded; a time budget is not reproducible
exploration_constant = 1.2
worker_threads       = 10            # 14 physical cores, GPU Face — §15.4 (was 16)
transposition_mode   = "throughput"

[engine.transposition]
enabled               = true
capacity_entries      = 256000000    # ~21 GB against a 24 GB budget — §16.3 (was 384000000)
shard_count           = 512          # power of two, comfortably above the worker count
reset_between_batches = false
retire_batch_size     = 65536
huge_pages            = "advise"     # "advise" | "off"; MADV_HUGEPAGE — §16.3.1

[lab.sampling]
record_positions_every_n_plies = 2   # unchanged in 1.4 — §14.2
record_terminal_positions      = true
max_edges_per_position         = 16
store_child_stats              = true
only_store_high_visit_edges    = false

[face]
enabled           = true
provider          = "candle"
device            = "auto"           # "cuda" | "cpu" | "auto" — §7.4.1, new in 1.4
device_index      = 0                # which CUDA device, when there is more than one
warm_on_start     = true             # matters more on CUDA, not less — §16.4
deadline_ms       = 2500             # raise it with the model, on either device
max_tokens        = 80
max_queue_depth   = 2
inference_threads = 2                # CPU path only; inert on CUDA — §15.4
min_interval_ms   = 4000             # rate limit between taunts
lab_mode_enabled  = false
fallback          = "canned"
verbosity         = "low"

# Two profiles, because the resolved device can change between boots without
# anyone editing this file. One model_path would silently put a 4.3 s model
# against a 2.5 s deadline on fallback. See §7.5.4.
[face.cuda_profile]
model_path        = "./models/qwen2.5-1.5b-instruct-q4_k_m.gguf"
tokenizer_path    = "./models/qwen2.5-1.5b-instruct/tokenizer.json"
model_id          = "qwen2.5-1.5b-instruct-q4_k_m"

[face.cpu_profile]
model_path        = "./models/qwen2.5-0.5b-instruct-q4_k_m.gguf"
tokenizer_path    = "./models/qwen2.5-0.5b-instruct/tokenizer.json"
model_id          = "qwen2.5-0.5b-instruct-q4_k_m"

[face.sampling]
temperature    = 0.7
top_p          = 0.9
repeat_penalty = 1.1
repeat_last_n  = 64
seed           = 0                   # 0 = per-request; non-zero = reproducible taunts

[face.circuit_breaker]
failure_threshold = 3                # consecutive failures before opening
cooldown_seconds  = 300              # 5 minutes open
half_open_probes  = 1                # single trial request before closing

[limits]
max_total_memory_gb = 56             # host RAM, validated at startup — §16.1 (was 96)
max_vram_mb         = 4608           # device memory, validated at load — §16.6, new in 1.4
```

---

## 23.1 Startup Validation

**Startup validation is not optional**, and 1.4 gives it a second budget to enforce.

**Host memory.** Before binding the listener, the process computes the projected memory ceiling from `transposition.capacity_entries`, `writer.channel_capacity`, the database cache settings, `read_pool_size`, `engine.lab.worker_threads`, and the CPU profile's model file size, and refuses to start if the total exceeds `limits.max_total_memory_gb`. A configuration that can OOM the process should fail in the first second, with a message naming the offending key — not at hour six of a batch.

**Device memory.** When the resolved device is CUDA, the projected VRAM footprint — weights, KV cache at `max_tokens`, and a fixed context allowance — is checked against `limits.max_vram_mb` before the model is loaded ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)). Exceeding it is a refusal to load, not a CUDA OOM: the error names the figure and the budget, which is information a driver error message does not carry.

**Deadline feasibility, both profiles.** This is the check that would have caught the defect in [§0.3](00-revision-history.md#03-what-changed-in-13) and again the one in [§0.4.3](00-revision-history.md#043-consequence-two--the-cpu-inference-path-cannot-meet-its-own-deadline). For each profile, the projected generation time for `max_tokens` is estimated from the model's resident size and the device's bandwidth ([§7.5.1](07-face-llm-layer.md#751-what-the-two-devices-actually-deliver)) and compared against `deadline_ms`:

- **The active profile fails the check** → refuse to start. A Face layer that cannot meet its deadline is a circuit that will be open within three moves, and it will report itself healthy while doing it.
- **The inactive profile fails the check** → start, and warn loudly, once. A CUDA deployment whose CPU fallback cannot meet the deadline is one driver update away from a silent outage.

The estimate is deliberately crude and deliberately conservative. It is not trying to predict latency; it is trying to catch a configuration that is wrong by a factor of two or more, which is the only kind of error this class of defect has ever actually taken.

---

← [22. Deployment Model](22-deployment-model.md) · **[Index](README.md)** · [24. Key Architectural Decisions](24-key-decisions.md) →
