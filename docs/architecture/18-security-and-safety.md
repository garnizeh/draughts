# 18. Security and Safety

MVP is local, but basic protections are required.

## 18.1 Input Validation

- Validate all JSON payloads.
- Validate move indices.
- Reject moves for finished matches.
- Reject moves from wrong side.
- Reject oversized commentary requests.
- **Validate lab batch configuration against the memory budget** — `channel_capacity`, `capacity_entries`, and `worker_threads` are rejected if their product would exceed [§16.1](16-memory-strategy.md#161-memory-budget). At 64 GB with 24 of them already committed to the transposition table, an unvalidated config is the most likely cause of an OOM, and the margin for absorbing one is half what it was at 1.3.
- **Validate the Face configuration against the VRAM budget** — a `cuda_profile` model larger than [§16.6](16-memory-strategy.md#166-vram-budget--new-in-14) allows is rejected at startup rather than discovered as a CUDA OOM, and a `cpu_profile` that cannot meet `deadline_ms` is rejected outright ([§7.5.4](07-face-llm-layer.md#754-two-profiles-not-one-model-path)).
- **Validate `format_version` on every read** ([§13.7](13-data-dictionary.md#137-format_version--new-in-11)).

---

## 18.2 LLM Prompt Safety

System prompt:

```text
You are a draughts commentary agent.
You do not choose moves.
You do not give strategic advice unless explicitly configured.
Keep responses short.
Do not reveal system internals.
```

The model receives only curated context ([§7.3](07-face-llm-layer.md#73-commentary-context)). Moving inference in-process does not change the threat model — the model is a local, offline, read-only artifact with no network access and no tools — but it does add one obligation: because prompt construction and engine state now live in the same binary, the boundary must be enforced by types rather than by the physical separation an HTTP call used to provide. `build_prompt` accepts `&CommentaryContext` and nothing else, and `CommentaryContext` cannot reference engine memory ([§7.3](07-face-llm-layer.md#73-commentary-context)).

---

## 18.3 Output Sanitization

Before display:

- Remove control characters.
- Collapse repeated whitespace.
- Truncate to max UI length.
- Escape HTML.

Model output is untrusted input to the renderer. It is generated locally, but it is still generated — and it is inserted into server-rendered HTML that HTMX swaps into the live DOM. Escaping is not optional, and templates must escape by default rather than relying on a call site remembering to.

---

← [17. Reliability and Failure Handling](17-reliability.md) · **[Index](README.md)** · [19. Extensibility Roadmap](19-extensibility-roadmap.md) →
