# 9. API Contract

Base path:

```text
/api/v1
```

Content type:

```text
application/json
```

HTMX endpoints may return HTML partials when:

```text
HX-Request: true
```

---

## 9.1 Error Model

```json
{
  "error": {
    "code": "illegal_move",
    "message": "The selected move is not legal in the current position.",
    "details": {}
  }
}
```

Common error codes:

| Code | HTTP Status | Meaning |
|---|---:|---|
| `not_found` | 404 | Resource does not exist |
| `invalid_request` | 400 | Malformed payload |
| `illegal_move` | 409 | Move is not legal |
| `match_finished` | 409 | Match is already over |
| `batch_not_cancellable` | 409 | Batch cannot be cancelled |
| `engine_unavailable` | 503 | Engine worker unavailable |
| `face_unavailable` | 200 | Commentary provider unavailable; served from fallback. Never an error status — commentary is optional |
| `write_queue_saturated` | 503 | The MPSC write channel is full and a durable write could not be accepted. Retryable |
| `unsupported_format_version` | 409 | A stored row carries a `format_version` this build cannot decode ([§13.7](13-data-dictionary.md#137-format_version--new-in-11)) |

`face_unavailable` is deliberately downgraded from v1.0's "200/503 depending on context" to always-200. With a circuit breaker in place, an unavailable model is a *designed* steady state, and a 503 would tell the client something false.

---

## 9.2 Health Endpoint

```http
GET /api/v1/health
```

Response:

```json
{
  "status": "ok",
  "version": "0.2.0",
  "engine": "mcts",
  "evaluator": "random_rollout",
  "sqlite_wal": true,
  "face": {
    "enabled": true,
    "provider": "candle",
    "device_requested": "auto",
    "device": "cuda:0",
    "device_name": "NVIDIA GeForce RTX 3050",
    "profile": "cuda_profile",
    "model_id": "qwen2.5-1.5b-instruct-q4_k_m",
    "model_loaded": true,
    "resident_mb": 96,
    "vram_used_mb": 1810,
    "vram_budget_mb": 4608,
    "circuit": "closed",
    "consecutive_failures": 0,
    "trips_total": 2,
    "cooldown_remaining_seconds": 0
  },
  "transposition_table": {
    "mode": "throughput",
    "entries": 143182608,
    "capacity": 256000000,
    "resident_mb": 11884,
    "hit_rate": 0.71
  },
  "writer": {
    "queue_depth": 18422,
    "queue_capacity": 262144,
    "rows_committed": 1841002331,
    "last_commit_ms": 412,
    "last_error": null
  }
}
```

`status` reports `"ok"` while the *engine and database* are healthy. A tripped circuit or an unloaded model does not make the service unhealthy — it makes it less entertaining. This distinction is what keeps an external process supervisor from restarting a perfectly functional server because a taunt timed out.

The four `device*` fields and the two `vram_*` fields are new in 1.4. They exist to answer one question without reading logs: **did the Face layer get the device it asked for?** `device_requested: "auto"` with `device: "cpu"` is the specific configuration in which commentary is silently running the fallback profile ([§7.4.1](07-face-llm-layer.md#741-device-selection)) — a legitimate state, but one an operator should be able to see rather than infer from latency. `vram_used_mb` is reported as 0 and `vram_budget_mb` as `null` on the CPU path, and `resident_mb` counts **host** RSS attributable to the Face layer on either device, which is why it is small when the weights are on the card.

---

## 9.3 Start Human vs. CPU Match

```http
POST /api/v1/matches
```

Request:

```json
{
  "mode": "human_cpu",
  "rules": "english_draughts",
  "human_side": "black",
  "seed": 812345,
  "engine": {
    "evaluator": "random_rollout",
    "iterations": 4000,
    "time_budget_ms": 1500,
    "exploration_constant": 1.4,
    "transposition_mode": "deterministic"
  },
  "face": {
    "enabled": true,
    "tone": "playful",
    "verbosity": "low"
  }
}
```

Response `201 Created`:

```json
{
  "match_id": "m_01J9XK4P2Q",
  "mode": "human_cpu",
  "rules": "english_draughts",
  "status": "active",
  "human_side": "black",
  "turn": "black",
  "ply": 0,
  "format_version": 1,
  "board": {
    "representation": "squares_64",
    "squares": [
      { "index": 0, "piece": null },
      { "index": 1, "piece": null },
      { "index": 2, "piece": null },
      { "index": 3, "piece": null }
    ],
    "legal_moves": [
      {
        "from": 9,
        "to": 13,
        "path": [9, 13],
        "capture": false,
        "promotion": false
      }
    ]
  },
  "material": {
    "black": 12,
    "white": 12
  },
  "commentary": {
    "text": "Another board, another opportunity for me to embarrass you.",
    "provider": "candle",
    "fallback": false
  }
}
```

Play Mode iteration budgets are raised from v1.0's 800 to 4000 ([§16.2](16-memory-strategy.md#162-engine-budgets)), because the transposition table makes the marginal iteration far cheaper and the machine is no longer constrained.

For HTMX, the server may return:

```html
<div id="board" hx-swap-oob="true">
  <!-- server-rendered board partial -->
</div>

<div id="status">
  Your move, Black.
</div>

<div id="commentary">
  Another board, another opportunity for me to embarrass you.
</div>
```

---

## 9.4 Get Match State

```http
GET /api/v1/matches/{match_id}
```

Response:

```json
{
  "match_id": "m_01J9XK4P2Q",
  "mode": "human_cpu",
  "status": "active",
  "turn": "white",
  "ply": 7,
  "format_version": 1,
  "board": {
    "representation": "squares_64",
    "squares": [],
    "legal_moves": []
  },
  "material": {
    "black": 11,
    "white": 12
  },
  "last_move": {
    "side": "black",
    "from": 10,
    "to": 14,
    "capture": false,
    "promotion": false
  },
  "result": null
}
```

HTMX variant may return a board partial.

---

## 9.5 Submit Human Move

```http
POST /api/v1/matches/{match_id}/moves
```

Request:

```json
{
  "move": {
    "from": 10,
    "to": 14,
    "path": [10, 14]
  }
}
```

For a multi-jump:

```json
{
  "move": {
    "from": 10,
    "to": 17,
    "path": [10, 14, 17]
  }
}
```

Response `200 OK`:

```json
{
  "match_id": "m_01J9XK4P2Q",
  "status": "active",
  "turn": "black",
  "ply": 9,
  "human_move": {
    "side": "black",
    "from": 10,
    "to": 14,
    "capture": false,
    "promotion": false
  },
  "cpu_move": {
    "side": "white",
    "from": 22,
    "to": 18,
    "capture": false,
    "promotion": false
  },
  "board": {
    "representation": "squares_64",
    "squares": [],
    "legal_moves": []
  },
  "material": {
    "black": 11,
    "white": 11
  },
  "engine": {
    "iterations": 4000,
    "elapsed_ms": 610,
    "tt_hit_rate": 0.64
  },
  "commentary": {
    "text": "Bold. Wrong, but bold.",
    "provider": "candle",
    "fallback": false
  }
}
```

With the circuit open, the same response carries:

```json
{
  "commentary": {
    "text": "Hm. I have seen better.",
    "provider": "canned",
    "fallback": true,
    "fallback_reason": "circuit_open"
  }
}
```

The client must render both identically. `fallback_reason` is diagnostic, not display copy.

If the game ends:

```json
{
  "match_id": "m_01J9XK4P2Q",
  "status": "finished",
  "result": "white_win",
  "turn": null,
  "ply": 48,
  "board": {
    "representation": "squares_64",
    "squares": [],
    "legal_moves": []
  },
  "material": {
    "black": 0,
    "white": 3
  },
  "commentary": {
    "text": "And that is why I do not panic. You do.",
    "provider": "candle",
    "fallback": false
  }
}
```

---

## 9.6 Resign Match

```http
POST /api/v1/matches/{match_id}/resign
```

Response:

```json
{
  "match_id": "m_01J9XK4P2Q",
  "status": "finished",
  "result": "white_win",
  "reason": "resignation"
}
```

---

## 9.7 Request Commentary

This endpoint is optional and can be used by the UI to fetch a taunt without changing game state.

```http
POST /api/v1/matches/{match_id}/commentary
```

Request:

```json
{
  "event": "idle_taunt",
  "tone": "sarcastic",
  "max_tokens": 60
}
```

Response:

```json
{
  "commentary": {
    "text": "Take your time. I only exist to defeat you.",
    "provider": "candle",
    "fallback": false,
    "latency_ms": 1840
  }
}
```

This endpoint always returns `200`, including when the circuit is open. It is also the endpoint most likely to be called on a timer by the UI, so it is subject to the minimum-interval rate limit in [§7.7.5](07-face-llm-layer.md#77-commentary-guardrails): requests inside the interval are answered from the canned set without consulting the breaker or the model.

---

## 9.8 Start Training Lab Batch

```http
POST /api/v1/lab/batches
```

Request:

```json
{
  "name": "nightly-random-rollout-1m",
  "rules": "english_draughts",
  "target_games": 1000000,
  "reproducible": false,
  "engine": {
    "evaluator": "random_rollout",
    "iterations": 800,
    "exploration_constant": 1.2,
    "transposition_mode": "throughput"
  },
  "sampling": {
    "record_positions_every_n_plies": 2,
    "record_terminal_positions": true,
    "max_edges_per_position": 16,
    "store_child_stats": true
  },
  "concurrency": {
    "worker_threads": 10,
    "channel_capacity": 262144,
    "db_batch_rows": 50000,
    "db_flush_interval_ms": 250
  },
  "transposition_table": {
    "enabled": true,
    "capacity_entries": 256000000,
    "shard_count": 512,
    "reset_between_batches": false
  },
  "seed": 424242
}
```

Response `202 Accepted`:

```json
{
  "batch_id": "b_01J9XK9TZZ",
  "name": "nightly-random-rollout-1m",
  "status": "queued",
  "target_games": 1000000,
  "completed_games": 0,
  "reproducible": false,
  "created_at": "2026-01-01T00:00:00Z"
}
```

`target_games` rises from v1.0's illustrative 100k to 1M, `worker_threads` from 2 to **10** — the 14-core partition in [§15.4](15-concurrency-model.md#154-cpu-partitioning), not the 16 assumed through 1.3 — and `db_batch_size` becomes `db_batch_rows: 50000`. Sampling density doubles, because storage and write throughput are no longer the limiting factors.

Validation rules worth stating explicitly:

- `reproducible: true` and `transposition_mode: "throughput"` is rejected with `invalid_request`. The combination is incoherent ([§6.7.5](06-mcts-extensibility.md#675-determinism-and-the-two-table-modes)).
- `reproducible: true` with a `time_budget_ms` and no `iterations` is rejected for the same reason.
- `channel_capacity` is capped by the memory budget in [§16.1](16-memory-strategy.md#161-memory-budget) and rejected above it, rather than being silently clamped.

---

## 9.9 List Lab Batches

```http
GET /api/v1/lab/batches
```

Response:

```json
{
  "batches": [
    {
      "batch_id": "b_01J9XK9TZZ",
      "name": "nightly-random-rollout-1m",
      "status": "running",
      "target_games": 1000000,
      "completed_games": 384210,
      "games_per_minute": 2140.5,
      "created_at": "2026-01-01T00:00:00Z",
      "started_at": "2026-01-01T00:00:04Z"
    }
  ]
}
```

---

## 9.10 Get Lab Batch Status

```http
GET /api/v1/lab/batches/{batch_id}
```

Response:

```json
{
  "batch_id": "b_01J9XK9TZZ",
  "name": "nightly-random-rollout-1m",
  "status": "running",
  "target_games": 1000000,
  "completed_games": 384210,
  "failed_games": 0,
  "games_per_minute": 2140.5,
  "estimated_remaining_seconds": 17262,
  "db_size_mb": 41630.8,
  "reproducible": false,
  "sampling": {
    "record_positions_every_n_plies": 2,
    "record_terminal_positions": true,
    "max_edges_per_position": 16,
    "store_child_stats": true
  },
  "engine": {
    "evaluator": "random_rollout",
    "iterations": 800,
    "exploration_constant": 1.2,
    "transposition_mode": "throughput"
  },
  "transposition_table": {
    "entries": 214773912,
    "capacity": 256000000,
    "hit_rate": 0.71,
    "collisions": 41822,
    "evictions": 0,
    "resident_mb": 17827
  },
  "writer": {
    "queue_depth": 18422,
    "queue_capacity": 262144,
    "queue_high_water": 258392,
    "rows_committed": 1841002331,
    "commits": 36820,
    "avg_commit_ms": 397,
    "backpressure_events": 3
  }
}
```

The `writer` and `transposition_table` blocks are the operational heart of a lab run. `queue_high_water` approaching `queue_capacity` means the writer is the bottleneck and `db_batch_rows` should rise; a `hit_rate` that decays over a long batch means the table is thrashing and `capacity_entries` should rise or sampling should be reduced.

---

## 9.11 Cancel Lab Batch

```http
POST /api/v1/lab/batches/{batch_id}/cancel
```

Response:

```json
{
  "batch_id": "b_01J9XK9TZZ",
  "status": "cancelling",
  "completed_games": 384210,
  "target_games": 1000000,
  "queue_depth_at_cancel": 18422
}
```

Cancellation is cooperative and now has a second phase: workers stop at a game boundary, then the writer actor must drain the channel before the batch can be reported `cancelled`. A batch with 500k buffered messages does not become `cancelled` instantly, and reporting otherwise would be a lie about durability. The status transitions `running → cancelling → cancelled`.

---

## 9.12 Optional Export Endpoint

This is optional for MVP but useful.

```http
GET /api/v1/lab/batches/{batch_id}/export?format=jsonl&limit=10000&after_id=0
```

Response:

```text
application/x-ndjson
```

Example line:

```json
{"type":"position","format_version":1,"batch_id":"b_01J9XK9TZZ","game_id":123,"ply":8,"side_to_move":0,"outcome":1,"root_q":0.32,"root_visits":800}
```

Every exported record carries its `format_version`. A downstream training pipeline that reads a version it does not understand must fail loudly rather than misinterpret a BLOB ([§19.5](19-extensibility-roadmap.md#195-format-version-evolution)).

Export runs on a read-only connection from the read pool, at a smaller `cache_size` ([§11.1](11-database-architecture.md#111-sqlite-runtime-configuration)), and must never block the writer.

---

← [8. Game Modes and Execution Flows](08-game-modes-and-flows.md) · **[Index](README.md)** · [10. Frontend Architecture](10-frontend-architecture.md) →
