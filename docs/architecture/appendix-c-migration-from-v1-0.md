# Appendix C — Migration from v1.0

For an existing v1.0 deployment, in order:

| # | Step | Notes |
|---:|---|---|
| 1 | Back up `draughts.db` | Everything else is reversible; this is not |
| 2 | Apply the schema migration in [§12.1](12-database-schema.md#121-migration-from-a-v10-database) | Additive except `lab_batches`, which is rebuilt for its new CHECK constraint |
| 3 | Download **both** profiles' `.gguf` files and tokenizers | Qwen2.5-1.5B-Instruct Q4_K_M for CUDA, Qwen2.5-0.5B-Instruct Q4_K_M for the CPU fallback; both Apache 2.0 ([§7.5.4](07-face-llm-layer.md#754-two-profiles-not-one-model-path)) |
| 4 | Replace the `[face]` config block | `base_url` and `provider = "ollama"` are removed. Since 1.4, `model_path` lives in `[face.cuda_profile]` and `[face.cpu_profile]`, not in `[face]` |
| 5 | Add `[face.circuit_breaker]`, `[engine.transposition]`, `[database.writer]` | All have working defaults; set them explicitly anyway |
| 6 | Raise the `[database]` cache and mmap settings | Per-role, per [§11.1](11-database-architecture.md#111-sqlite-runtime-configuration) — do not apply the writer's 4 GB cache to readers |
| 7 | Stop and remove the Ollama service | It is no longer contacted; leaving it running wastes memory |
| 8 | Build with `--features cuda` if the host has a device, and deploy; verify `/health` | `face.model_loaded: true`, `face.circuit: "closed"`, `face.device` matching `face.device_requested`, `transposition_table.entries` climbing ([§22.1](22-deployment-model.md#221-mvp-single-machine-deployment)) |
| 9 | Run a 1 000-game batch with `reproducible: true` | Confirms determinism survived the transposition table |
| 10 | Re-baseline the [§20.9](20-testing-strategy.md#209-performance-regression-baselines) metrics | The v1.0 numbers are meaningless now |

**Rollback:** the v1.0 binary reads a migrated database without modification. `format_version`, `fallback_reason`, and `circuit_state` are columns it does not select, and the new `lab_batches` statuses appear only on rows v1.0 would not have created. The one-way element is data written *after* migration — nothing in it is v1.0-incompatible, but it carries a `format_version` column that v1.0 does not check.

---

← [Appendix B — Performance Targets](appendix-b-performance-targets.md) · **[Index](README.md)** · [Appendix D — Risks and Open Questions](appendix-d-risks-and-open-questions.md) →
