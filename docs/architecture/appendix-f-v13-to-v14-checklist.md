# Appendix F — v1.3 → v1.4 Change Checklist

Every value that moved when the assumed host was replaced with the real one. [Appendix E](appendix-e-change-checklist.md) is the equivalent list for v1.0 → v1.1 and is historical; this one is current.

## Hardware

| Area | v1.3 (assumed) | v1.4 (actual) |
|---|---|---|
| RAM | 128 GB | **64 GB** — 2 × 32 GB DDR4-2400 ECC |
| Memory channels | Implied 4+ | **2 populated of 4**; ~38 GB/s peak, ~25–30 achievable |
| CPU | "16+ physical cores" | **Xeon E5-2690 v4** — 14C/28T, 2.6 GHz, 35 MB L3, 1 NUMA node |
| GPU | "None required"; RTX 3050 noted as 8 GB | **RTX 3050, 6 GB**, CUDA 13.2, driver 595.84, compute capability 8.6 |
| Hardware posture | Spend 128 GB deliberately | Spend 64 GB and a 6 GB card deliberately, with a stated ceiling per consumer on each |

## Memory and storage

| Area | v1.3 | v1.4 |
|---|---|---|
| Committed host budget | 86 GB | **50.5 GB** |
| Reserve | 42 GB | **13.5 GB** (~21 % of host) |
| `limits.max_total_memory_gb` | 96 | **56** |
| Transposition table budget | 32 GB | **24 GB** |
| `capacity_entries` | 384 000 000 | **256 000 000** |
| `shard_count` | 1024 | **512** |
| Huge pages for the table | Not mentioned | **`huge_pages = "advise"`**, new key ([§16.3.1](16-memory-strategy.md#1631-bandwidth-not-just-capacity--new-in-14)) |
| SQLite writer cache | 8 GB | **4 GB** |
| SQLite reader cache | 512 MB × 8 | **256 MB × 6** |
| `mmap_size` | 32 GB | **8 GB** |
| `PRAGMA threads` (writer) | 8 | **4** |
| MPSC `channel_capacity` | 524 288 (~4 GB) | **262 144 (~2 GB)** |
| MCTS arenas | 16 × 512 MB = 8 GB | **10 × 512 MB = 5 GB** |
| Candle host budget | 8 GB | **2 GB** |
| VRAM budget | Not budgeted | **4.5 GB cap against ~5.0 GB usable**; `limits.max_vram_mb = 4608` ([§16.6](16-memory-strategy.md#166-vram-budget--new-in-14)) |
| Sampling density | Every 2 plies, 16 edges | **Unchanged** — halving RAM changes what is cached, not what is written |

## Concurrency

| Area | v1.3 | v1.4 |
|---|---|---|
| MCTS worker pool formula | `physical_cores - 3` | **`physical_cores - 4`** |
| Lab `worker_threads` | 16 | **10** (8 on the CPU fallback path) |
| Tokio `worker_threads` | 8 | **4** |
| Inference pool | 1–2 cores, always | **1 thread on CUDA (~0 cores); 2 cores on the CPU path** |
| SMT guidance | Not stated | **Explicit**: size to physical cores, measure 8 / 10 / 14 / 20 ([§15.4.1](15-concurrency-model.md#1541-why-not-28-workers)) |
| Play Mode binding constraint | `iterations = 4000` | **`time_budget_ms = 1500`**; iterations is a ceiling ([§16.2](16-memory-strategy.md#162-engine-budgets)) |

## The Face layer

| Area | v1.3 | v1.4 |
|---|---|---|
| Device | `Device::Cpu`, hard-coded | **`face.device = "cuda" \| "cpu" \| "auto"`**, resolved by `select_device` ([§7.4.1](07-face-llm-layer.md#741-device-selection)) |
| Default device, target host | CPU | **CUDA** |
| Model configuration | One `model_path` under `[face]` | **`[face.cuda_profile]` and `[face.cpu_profile]`** ([§7.5.4](07-face-llm-layer.md#754-two-profiles-not-one-model-path)) |
| Default model, CUDA | — | **Qwen2.5-1.5B-Instruct Q4_K_M**, ~0.8–1.5 s / 64 tokens |
| Default model, CPU | Qwen2.5-1.5B-Instruct Q4_K_M | **Qwen2.5-0.5B-Instruct Q4_K_M**, ~1.7 s / 64 tokens |
| Assumed 2-core bandwidth | 35 GB/s | **15 GB/s** — the 1.3 figure exceeded this platform's total |
| Quality profile | 7B Q5_K_M on CPU, `deadline_ms ≥ 12000` | **7B Q4_K_M on a headless card only**; does not fit alongside a desktop session |
| Build | `cargo build --release` | Unchanged by default; **`CUDA_COMPUTE_CAP=86 cargo build --release --features cuda`** for the target host. CI builds both |
| Missing / broken GPU | n/a | **Falls back to the CPU profile**, one warning, never an error |
| Deployment artifacts | One `.gguf` | **Two `.gguf` files**, ~1.4 GB together |

## Interfaces, testing, targets

| Area | v1.3 | v1.4 |
|---|---|---|
| `/health` face block | provider, model, circuit | **+ `device_requested`, `device`, `device_name`, `profile`, `vram_used_mb`, `vram_budget_mb`** |
| Observability | — | **+ resolved device vs. requested, VRAM against budget, completed iterations per move** |
| Startup validation | Host memory budget | **+ VRAM budget, + deadline feasibility for both profiles** ([§23.1](23-configuration-example.md#231-startup-validation)) |
| Test suites | 20.1–20.9 | **+ [§20.10](20-testing-strategy.md#2010-device-parity-and-cuda-tests) Device Parity and CUDA** |
| Acceptance criteria | 22 | **27** ([§25](25-acceptance-criteria.md)) |
| Lab games/minute | 1 500–2 500 | **600–1 200** |
| Sustained write throughput | 150k–400k rows/s | **100k–250k rows/s** |
| Commit latency p99, 50k rows | < 1 500 ms | **< 2 000 ms** |
| Play Mode move latency p99 | < 1 200 ms | **< 1 600 ms** |
| Peak RSS gate | < 96 GB | **< 56 GB** |
| Peak VRAM gate | — | **< 4.5 GB** |
| Recorded risks | 8 | **13** ([Appendix D](appendix-d-risks-and-open-questions.md)) |

## What did not change

No structural decision was reopened. The `EvaluationStrategy` trait, the two transposition modes, the MPSC writer actor, the two durability classes, the circuit breaker, `format_version`, the schema, and the frontend are all untouched. Request shapes are unchanged; `/health` alone was extended, with the six Face fields in the table above. The GPU arrived through the one-line `Device` seam [§19.6.5](19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve) was holding open for it, and no caller changed.

---

← [Appendix E — v1.0 → v1.1 Change Checklist](appendix-e-change-checklist.md) · **[Index](README.md)** · →
