# Changelog

Notable changes to the implementation. The architecture has its own history in
[§0](docs/architecture/00-revision-history.md); this file records code.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Cargo workspace, pinned toolchain, and the module layout from
  [§4](docs/architecture/04-separation-of-concerns.md) and
  [§5](docs/architecture/05-runtime-components.md).
- `justfile` as the single entry point for local and CI automation; CI invokes
  the same recipes, so `just ci` and a green pipeline mean the same thing.
- Configuration types mirroring [§23](docs/architecture/23-configuration-example.md),
  with the startup validation from §23.1: the host-memory ceiling, the VRAM
  budget, and deadline feasibility for **both** device profiles.
- `draughts --check-config`, which runs every §23.1 check without opening the
  database, allocating the table, or binding a port.
- Device selection ([§7.4.1](docs/architecture/07-face-llm-layer.md#741-device-selection)),
  with `candle_core::Device` constructed in exactly one function and a CI grep
  that keeps it that way.
- Circuit breaker ([§7.8](docs/architecture/07-face-llm-layer.md#78-circuit-breaker--new-in-11))
  against an injected clock, with the failure classification from §7.8.3.
- Canned commentary covering every `(CommentaryEvent, Tone)` pair, and output
  sanitization ([§7.7](docs/architecture/07-face-llm-layer.md)).
- Zobrist key table generated from a fixed seed, pinned by a fingerprint test so
  that a silent regeneration fails the build rather than invalidating every
  persisted `board_hash`.
- The MVP schema ([§12](docs/architecture/12-database-schema.md)) as migration
  1, applied at startup inside one transaction.
- The `/api/v1` route surface from [§9](docs/architecture/09-api-contract.md),
  with `/health` answering fully.
- The error model from §9.1, including the deliberate absence of a
  `face_unavailable` error status.

### Fixed

- `send_durable`'s payload and acknowledgement now travel as one channel
  message (`WriteOp::Durable`) instead of two, closing a window where a
  saturated channel could accept the payload and then fail to enqueue the
  barrier that acks it — reporting `Degraded` for a write already queued for
  commit and inviting a retry that would double-submit it.
- `/health`'s `transposition_table.mode` now reads `engine.play.transposition_mode`,
  matching the section the adjacent `evaluator` field already reads, instead of
  `engine.lab.transposition_mode`.
- `FaceStatus::unloaded` reports `vram_budget_mb` from `limits.max_vram_mb` on
  the CUDA path instead of always `None`.
- `Board::from_bytes` now takes `format_version` as an explicit parameter, like
  `GameRecord::decode_moves`, so a caller cannot decode a persisted board
  without having looked at it (§13.7).
- Commentary sanitization now strips U+061C (Arabic Letter Mark) alongside the
  other bidirectional formatting controls (§7.7).
- CI: `actions/checkout` no longer persists Git credentials past the job; the
  `gate`/`cuda-compile`/`portable-build` jobs pin the toolchain to the exact
  version `rust-toolchain.toml` declares instead of `dtolnay/rust-toolchain`'s
  `stable`, which does not read that file; `check-cuda` now runs Clippy against
  the `cuda` feature, not just `cargo check`, so that path is actually linted;
  the nightly `load` job is disabled until `tests/load.rs`'s `todo!()` bodies
  are implemented, instead of failing every scheduled run.

### Not yet implemented

Move generation, tree search, the writer actor loop, the lab worker pool, and
GGUF loading. Each is a `todo!()` at the seam the architecture defines for it,
with the owning section named.

[Unreleased]: https://github.com/garnizeh/draughts/commits/main
