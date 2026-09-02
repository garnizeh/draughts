# Appendix E — v1.0 → v1.1 Change Checklist

| Area | v1.0 | v1.1 |
|---|---|---|
| LLM runtime | Ollama daemon over REST | Candle in-process, quantized `.gguf` |
| LLM size | 0.5B–1.5B | 8B–14B *(re-baselined to 1.5B in 1.3, then per-device in 1.4 — see [Appendix F](appendix-f-v13-to-v14-checklist.md))* |
| Deployment units | Binary + Ollama service | One binary + one model file |
| HTTP client dependency | `reqwest` | Removed |
| Face failure policy | Timeout → fallback, per request | Circuit breaker: 3 failures → 5 min open |
| `Face::commentary` signature | `Result<Commentary, FaceError>` | `Commentary` — infallible by construction |
| Commentary delivery | Inline with the move response | Decoupled; the board returns at engine speed |
| MCTS caching | None; per-search trees discarded | Global lock-free `DashMap` table, 24 GB |
| Determinism | Global promise | Per-batch property, `TtMode::Deterministic` |
| SQLite page cache | 64 MB (16 MB constrained) | 8 GB writer / 512 MB per reader |
| `temp_store` | MEMORY | MEMORY (unchanged, now load-bearing) |
| `mmap_size` | 256 MB | 32 GB |
| Checkpointing | `wal_autocheckpoint = 1000` | Manual, writer-scheduled |
| Write path | "A writer actor or dedicated connection" | Specified MPSC actor, bounded 524 288 messages |
| Transaction size | 100–5 000 rows | 50 000+ rows |
| Durability | Undifferentiated | Two named classes ([§11.4](11-database-architecture.md#114-durability-classes--new-in-11)) |
| Lab workers | 2 | 16 |
| Play iterations | 800 | 4 000 |
| Lab iterations | 200 | 800 |
| Sampling density | Every 4 plies, 8 edges | Every 2 plies, 16 edges |
| Load test scale | 10k games | 1M games, 5M+ rows |
| BLOB versioning | None | `format_version` on `games` and `positions` |
| `face_events` | provider, fallback flag | + `fallback_reason`, `circuit_state` |
| Batch statuses | queued/running/completed/cancelled/failed | + `cancelling`, `interrupted` |
| Process separation | Supported and cheap | Supported and expensive; explicitly not recommended ([§22.2](22-deployment-model.md#222-optional-process-separation)) |
| Hardware posture | Survive scarcity | Spend 128 GB deliberately, with a stated ceiling per consumer |

**This appendix is historical.** It records the v1.0 → v1.1 transition and is not updated by later revisions; several of its right-hand values were superseded in 1.3 and again in 1.4. For current figures see [Appendix F](appendix-f-v13-to-v14-checklist.md), which is the equivalent checklist for v1.3 → v1.4.

---

← [Appendix D — Risks and Open Questions](appendix-d-risks-and-open-questions.md) · **[Index](README.md)** · [Appendix F — v1.3 → v1.4 Change Checklist](appendix-f-v13-to-v14-checklist.md) →
