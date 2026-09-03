# 20. Testing Strategy

Every 1.1 shift introduces a way for the system to be *fast and wrong*: a shared cache can leak values across evaluators, a batching writer can silently drop rows, a circuit breaker can trip under healthy load, and a versioned BLOB can be decoded by the wrong reader. Sections 20.1 through 20.4 are the v1.0 suite, extended where the new components touch them. Sections 20.5 through 20.9 are new, and each one exists to catch a specific failure mode introduced by this revision.

## 20.1 Rules Tests

- Legal move generation.
- Mandatory capture enforcement.
- Multi-jump sequences.
- Promotion behavior.
- Terminal detection.
- **Draw rules** ([§5.3.1](05-runtime-components.md#531-draw-rules-for-mvp--new-in-15)) — four properties, of which the last is the one that protects something other than itself: the non-progress counter resets on a capture and on a man move and on nothing else; a position recurring for the third time since the last irreversible move is a draw; the adjudicator reads the configured thresholds rather than a constant; and **`apply_move` never returns `Finished(Draw)`**, because a draw adjudicated by the Rules Core would make terminality path-dependent and put [§20.5](#205-transposition-table-tests) at risk.
- **Random play terminates.** A large corpus of random games under the default draw policy, every one of them finishing without reaching any playout cap — §5.3.1's termination proof asserted rather than argued.
- Bitboard correctness.
- **Zobrist correctness** — incremental update after `apply_move` equals full recomputation, over a large random-play corpus.
- **Zobrist key stability** — the committed key table hashes to a known constant. A build that silently regenerates its keys invalidates every persisted `board_hash`, and [§13.7](13-data-dictionary.md#137-format_version--new-in-11) makes that a `format_version` bump rather than an accident.
- **Perft-style node counts** to fixed depth from the initial position and a set of tactical positions, compared against a committed baseline. One number per depth, catching move-generation regressions that no example-based test will.

Use property-based tests where possible.

Zobrist hashing was a minor detail in v1.0 and is load-bearing since 1.1: it is the key of a 256M-entry shared cache ([§6.7](06-mcts-extensibility.md#67-global-transposition-table--new-in-11)) as well as a persisted column. Its tests are promoted accordingly.

---

## 20.2 Engine Tests

- MCTS returns legal move.
- MCTS prefers immediate win when available.
- MCTS avoids immediate loss when possible.
- Determinism under fixed seed.
- Evaluator trait mockability.
- **Value normalization symmetry** — `score(state, Black) == -score(state, White)` for every non-draw terminal fixture.
- **`is_position_pure()` honesty** — a property test calls `estimate_leaf_value` twice on the same state and asserts that evaluators claiming purity agree, and that impure ones are permitted to differ. An evaluator that claims purity and is not is the single most dangerous bug the transposition table can host, because it converts a cache into a source of wrong answers.

---

## 20.3 Integration Tests

- Start match.
- Submit legal move.
- Reject illegal move.
- Finish game.
- Start lab batch.
- Cancel lab batch.
- Verify SQLite rows.
- **Verify the two-phase cancel** — status moves `running → cancelling → cancelled`, and every game counted in `completed_games` is present in `games` once the drain completes ([§17.6](17-reliability.md#176-cancellation)).
- **Verify restart recovery** — kill the process mid-batch; on restart the batch is `interrupted`, `completed_games` is recomputed from the row count, and the database opens cleanly ([§11.4](11-database-architecture.md#114-durability-classes--new-in-11)).
- **Verify Play Mode responsiveness under lab load** — a human match played through the API while a 10-worker batch runs must keep move latency within budget. This is the end-to-end proof of the core reservation in [§15.4](15-concurrency-model.md#154-cpu-partitioning).
- **Verify a full game with no model present** — with both profiles' `model_path` pointing at missing files, the process starts, `/health` reports `model_loaded: false` and `circuit: "open"`, and a complete game is playable on canned lines.

---

## 20.4 Load/Volume Tests

- Insert 1M games. *(v1.0: 10k. The writer actor changes what counts as a load test.)*
- Insert sampled positions/edges — 5M+ rows at the default sampling density.
- Verify WAL behavior, including that manual checkpointing keeps the WAL under `journal_size_limit` across a multi-hour run.
- Verify batch commit throughput at `db_batch_rows` of 1, 1 000, and 50 000 — proving batch size is a performance knob and not a correctness one.
- Verify no writer starvation.
- **Verify no reader starvation** — status and export queries stay responsive while the writer sustains 50k-row commits.
- **Verify queue high-water behavior** — drive the channel to capacity and confirm the three producer policies in [§11.2.5](11-database-architecture.md#1125-backpressure): lab blocks, durable writes return `write_queue_saturated`, telemetry drops with a counter.
- **Verify peak RSS against the [§16.1](16-memory-strategy.md#161-memory-budget) budget.** This is a gate, not a metric: a run that exceeds the committed budget fails the build regardless of how fast it was.

---

## 20.5 Transposition Table Tests

The governing requirement: **the table may change how long a search takes and must never change what it returns.**

- **Differential search test.** For a corpus of positions, run `search()` with `TranspositionTable::disabled()` and with a live table in `Deterministic` mode. Assert identical `best_move`, identical root visit distribution, identical `root_q`. Run at 1, 2, 8, and 10 threads. This is the primary safety net for the entire feature.
- **Reproducibility test.** A 1 000-game batch with `reproducible: true`, run twice at different thread counts, must produce byte-identical `games.moves` BLOBs for every game.
- **Non-reproducibility is asserted too.** The same batch in `Throughput` mode is expected to diverge. A test that finds throughput mode reproducible is evidence the table is not actually being shared — a silent performance bug worth failing over.
- **Collision handling.** Inject synthetic entries with a forced Zobrist match and a different `Board`; assert the probe reports a miss, increments `collisions`, and never serves the wrong entry.
- **Evaluator scoping.** Store under identity A, probe under identity B, assert miss. Store under A, probe under A, assert hit.
- **Purity gating.** In `Deterministic` mode, assert an `Estimate` entry is never stored and never served, and that `Terminal` and `Exact` entries are.
- **Merge semantics.** A `Terminal` entry is never overwritten by an `Estimate`; two `Estimate` merges produce a correctly sample-weighted mean.
- **Capacity and retirement.** Fill past capacity; assert the entry count stabilizes, `Terminal` entries survive preferentially, and no probe panics or blocks for longer than a shard sweep.
- **Concurrency stress.** Ten threads probing and storing over a small key space under `loom` or a long randomized soak, asserting no deadlock, no torn entry, and no lost `Terminal` proof. Also run at 20 threads, above the configured worker count, so an operator who oversubscribes the pool ([§15.4.1](15-concurrency-model.md#1541-why-not-28-workers)) is not the first to find a bug there.

---

## 20.6 Writer Actor and Durability Tests

- **Round-trip.** Every record type survives write → commit → read with all fields intact.
- **Batch integrity.** Enqueue 250 000 rows across mixed message types, flush, and assert row counts and checksums match exactly.
- **Flush barrier.** A durable write's `ack` resolves only after its transaction has committed — verified by killing the process immediately after the ack and asserting the row is present on restart.
- **Crash semantics.** Kill the process mid-batch with a full channel; assert the database opens cleanly, no partial transaction is visible, and only bulk-class data is missing ([§11.4](11-database-architecture.md#114-durability-classes--new-in-11)).
- **Retry classification.** Inject `SQLITE_BUSY`; assert the buffer is preserved across retries and eventually commits. Inject a constraint violation; assert it is *not* retried and surfaces immediately.
- **Poisoned batch isolation.** A batch that exhausts its retries marks its own `lab_batches` rows `failed` and the writer keeps draining. One bad batch must not stop the actor.
- **Degraded mode.** Simulate a full disk; assert durable writes return `503`, bulk writes drop with a counter, `/health` reports `status: "degraded"`, and the read pool keeps serving.
- **Id allocation.** Concurrent leases produce disjoint ranges; `resume_from` after a simulated crash never reissues a used id; `position_edges` inserted with pre-assigned parent ids satisfy the foreign key.
- **Ordering.** A `Positions` message enqueued after its `Game` message commits in the same or a later transaction, never an earlier one — the foreign key depends on it.

---

## 20.7 Face and Circuit Breaker Tests

The breaker is tested against an injected clock and a scripted adapter. No test sleeps for five minutes.

- **Trip threshold.** Two consecutive failures leave the circuit `Closed`; the third opens it. A success at any point before the third resets the counter to zero.
- **Failure classification.** `Saturated` and `Disabled` never trip the breaker regardless of count; `Timeout`, `Inference`, `ModelNotLoaded`, and empty-after-sanitization always do ([§7.8.3](07-face-llm-layer.md#783-what-counts-as-a-failure)).
- **Short circuit.** While `Open`, assert the primary adapter's `generate_commentary` is never invoked, the response is canned, `fallback_reason` is `circuit_open`, and latency is sub-millisecond.
- **Cooldown and half-open.** Advance the injected clock past 300 s; assert exactly one probe is admitted regardless of concurrent request count; a successful probe closes the circuit, a failed one re-opens it with a fresh cooldown.
- **Concurrency.** A hundred simultaneous requests against an open circuit produce at most one probe.
- **Gameplay isolation.** A move submitted while the primary adapter hangs indefinitely returns within the engine's own latency budget. This test encodes the entire point of [§7.8](07-face-llm-layer.md#78-circuit-breaker--new-in-11).
- **Prompt safety.** A golden-file test asserts no rendered prompt contains a move, a square index, a legal-move list, or wording that would invite one. The fixture set is extended whenever a `CommentaryEvent` is added.
- **Sanitization.** Adversarial model output — control characters, 10 000 tokens, `<script>` tags, ANSI escapes — is truncated, stripped, and HTML-escaped before it reaches a template.
- **Canned coverage.** Every `(CommentaryEvent, Tone)` pair returns a non-empty line. A missing pair is a startup failure, not a runtime blank.

---

## 20.8 Format Version Tests

- **Round-trip per version.** Encode and decode every record type under every known `format_version`.
- **Historical fixtures.** A committed binary fixture file of rows produced by each historical version, decoded by the current build and compared against expected values. This is what actually protects old data; a round-trip test only proves the current encoder agrees with the current decoder.
- **Unknown version.** A row with `format_version = 255` produces `unsupported_format_version` — not a panic, not a default, not a silently skipped row.
- **No implicit defaulting.** A static check over the insert statements asserts every write path sets `format_version` explicitly from `CURRENT_FORMAT_VERSION`.
- **Export tagging.** Every exported JSONL line carries a `format_version` field.

---

## 20.9 Performance Regression Baselines

Tracked as committed numbers, not as pass/fail assertions. Expected values are in [Appendix B](appendix-b-performance-targets.md).

| Metric | Measured by |
|---|---|
| Lab games per minute | Fixed 10 000-game batch, fixed seed, at 8 / 10 / 14 / 20 workers ([§15.4.1](15-concurrency-model.md#1541-why-not-28-workers)) |
| Transposition hit rate | Same batch at 1M / 10M / 100M entry capacity |
| Search speedup, table on vs. off | Same batch, both `TtMode` settings |
| Sustained writer throughput | Synthetic 5M-row load |
| Commit latency at 50k rows | p50 / p99 |
| Play Mode move latency | 4 000-iteration ceiling under a 1 500 ms budget, p50 / p99 |
| **Completed iterations per move, Play Mode** | Same runs. Distinguishes "slow" from "time-bounded", which on this host is the steady state ([§16.2](16-memory-strategy.md#162-engine-budgets)) |
| Commentary latency, CUDA | Qwen2.5-1.5B Q4_K_M on the RTX 3050, 64 tokens, p50 / p99 |
| Commentary latency, CPU | Qwen2.5-0.5B Q4_K_M, 64 tokens, 2 inference cores, p50 / p99 |
| **Transposition probe rate** | Probes/second under a full lab batch, with and without huge pages ([§16.3.1](16-memory-strategy.md#1631-bandwidth-not-just-capacity--new-in-14)) |
| Move latency, circuit open vs. closed | p50 / p99 |
| Peak RSS, full-density batch | Against the [§16.1](16-memory-strategy.md#161-memory-budget) budget — a gate, not a metric |
| Peak VRAM, commentary under lab load | Against the [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) budget — also a gate |

---

## 20.10 Device Parity and CUDA Tests

**New in 1.4.** The GPU is optional by design ([§7.4.1](07-face-llm-layer.md#741-device-selection)), and "optional" is a claim that decays silently unless something tests it. These are the tests that keep the CPU path real.

The suite splits in two. The first half runs everywhere, including CI runners with no GPU, and is the half that matters most:

- **The default build has no CUDA dependency.** `cargo build --release` with no features produces a binary that links and runs on a machine with no driver and no toolkit. Asserted by building in a container with neither.
- **The full suite passes on CPU.** Every test in [§20.1](#201-rules-tests)–[§20.9](#209-performance-regression-baselines) runs with `face.device = "cpu"`, on the default build. This is the gate: a change that can only be tested on a GPU is a change that has broken the property.
- **`Device` is constructed once.** A static check — a grep in CI is sufficient and honest — asserts `Device::Cpu` and `Device::new_cuda` appear only inside `select_device` ([§19.6.5](19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve) property 1).
- **CUDA requested, unavailable, no GPU.** With `face.device = "cuda"` on a machine with no device, the process starts, logs exactly one warning, resolves to CPU, loads the **CPU profile's** model, and `/health` reports `device_requested: "cuda"`, `device: "cpu"`. This is the silent-outage case from [§7.4.1](07-face-llm-layer.md#741-device-selection) rule 4, and it is the single most important test in this section.
- **Profile validation.** A configuration whose `cpu_profile` model cannot meet `deadline_ms` under the [§7.5.1](07-face-llm-layer.md#751-what-the-two-devices-actually-deliver) relation is refused at startup, naming the key. A configuration whose `cuda_profile` exceeds the [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) budget is refused the same way.
- **`--features cuda` builds.** A compile-only CI job, no device required, so the feature-gated path cannot rot.

The second half requires the device and runs on the target host only. It is explicitly **not** a merge gate — making it one would reintroduce the GPU dependency this section exists to prevent:

- **Device parity.** The same prompt and a fixed seed on CPU and on CUDA produce output that passes the same sanitization and length assertions. Token-identical output is **not** asserted: different kernels give different floating-point results, and demanding bit-equality here would be testing the hardware rather than the design.
- **VRAM budget.** Commentary under a full lab batch stays within `vram_budget_mb`, measured, and reported as a gate in [§20.9](#209-performance-regression-baselines).
- **CUDA OOM at load.** With a model deliberately too large for the budget, the process starts, the breaker is permanently open, `/health` says `model_loaded: false`, and a complete game is playable. It does **not** silently retry on the CPU ([§7.4.1](07-face-llm-layer.md#741-device-selection)).
- **CUDA OOM mid-generation.** Surfaces as `FaceError::Inference`, counts as a breaker failure, and does not affect move latency.
- **Gameplay isolation, on the device.** [§20.7](#207-face-and-circuit-breaker-tests)'s hanging-adapter test, repeated with a real CUDA context, because a stalled device call is a different kind of hang from a stalled CPU loop.

---

← [19. Extensibility Roadmap](19-extensibility-roadmap.md) · **[Index](README.md)** · [21. Observability](21-observability.md) →
