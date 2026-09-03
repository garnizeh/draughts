# Changelog

Notable changes to the implementation. The architecture has its own history in
[§0](docs/architecture/00-revision-history.md); this file records code.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Newest first, and five releases deep.** This file holds `[Unreleased]` and the
five most recent releases; everything older is archived under
[docs/changelog/](docs/changelog/README.md), one file per release. `just
changelog-rotate` moves them, `just changelog-check` is in the merge gate, and
neither is optional — a changelog nobody can read is a changelog nobody reads.

## [Unreleased]

### Added

- Cargo workspace, pinned toolchain, and the module layout from [§4](docs/architecture/04-separation-of-concerns.md) and [§5](docs/architecture/05-runtime-components.md).
- `justfile` as the single entry point for local and CI automation; CI invokes the same recipes, so `just ci` and a green pipeline mean the same thing.
- Configuration types mirroring [§23](docs/architecture/23-configuration-example.md), with the startup validation from §23.1: the host-memory ceiling, the VRAM budget, and deadline feasibility for **both** device profiles.
- `draughts --check-config`, which runs every §23.1 check without opening the database, allocating the table, or binding a port.
- Device selection ([§7.4.1](docs/architecture/07-face-llm-layer.md#741-device-selection)), with `candle_core::Device` constructed in exactly one function and a CI grep that keeps it that way.
- Circuit breaker ([§7.8](docs/architecture/07-face-llm-layer.md#78-circuit-breaker--new-in-11)) against an injected clock, with the failure classification from §7.8.3.
- Canned commentary covering every `(CommentaryEvent, Tone)` pair, and output sanitization ([§7.7](docs/architecture/07-face-llm-layer.md)).
- Zobrist key table generated from a fixed seed, pinned by a fingerprint test so that a silent regeneration fails the build rather than invalidating every persisted `board_hash`.
- The MVP schema ([§12](docs/architecture/12-database-schema.md)) as migration 1, applied at startup inside one transaction.
- The `/api/v1` route surface from [§9](docs/architecture/09-api-contract.md), with `/health` answering fully.
- The error model from §9.1, including the deliberate absence of a `face_unavailable` error status.
- Automated releases (`.github/workflows/release.yml`). The version in `Cargo.toml` is the source of truth and `CHANGELOG.md` is the gate: a push to `main` cuts an annotated tag only when the version has no tag *and* its CHANGELOG section is closed — a dated `## [x.y.z] - YYYY-MM-DD` heading, not `[Unreleased]`. The gate is then re-run at that tag before anything is built, and the release notes are the CHANGELOG section itself rather than a generated commit list. No bot rewrites the CHANGELOG.
- Two release artefacts, Linux x86-64 only, each with a `.sha256` verified in CI before publishing: a portable build with no CUDA dependency, built *and run* in a container with neither driver nor toolkit, and a `cuda` build that is linked but never run. Both halves of [§22.1](docs/architecture/22-deployment-model.md) now ship rather than only compiling.
- `just version`, `just release-notes`, `just release-check`, `just package` and `just coverage` — the release workflow invokes recipes, like every other job, so a green machine and a green pipeline still mean the same thing.
- `scripts/check-no-cuda-linkage.sh`: one definition of the NEEDED assertion, shared by `ci.yml`'s `portable-build` job and `just package portable`, so the binary CI checks and the binary a release ships are checked the same way.
- Coverage as a CI job and a `just coverage` recipe. Reported, never gated: a percentage threshold against a tree whose unimplemented seams are `todo!()` would measure the seams and be met by deleting them.
- `actionlint` over `.github/workflows/`, and `SECURITY.md`, `.github/CODEOWNERS` and a pull request template carrying the five rules as a checklist.
- OpenSSF Scorecard, weekly and on `main`, publishing its result. It grades the claims this repository makes about itself — pinned actions, scoped tokens, branch protection, a security policy — and never gates a merge.
- Pull requests are assigned to their author on open.
- `just pre-pr`: every job `ci.yml` runs, run locally in CI's order — the gate, actionlint, `just audit`, the portable build with its linkage assertion, the CUDA path, and coverage. `just ci` is one of those six, and running it alone was the gap this closes. It is honest about the two things it cannot fully reproduce: `portable-check` builds outside a driverless container, and the CUDA recipes need a toolkit on the host.
- `just doc-links` in the merge gate (`scripts/check-doc-links.py`): every relative link and every `#anchor` across the documentation resolves, or the gate is red. This tree cites itself several hundred times across sixty-odd files, and renaming one heading says nothing about the twenty references it just broke. It is the first rule to graduate from `LESSONS.md` into a script.
- A `review-response` skill and a `/respond` command owning the review loop end to end, and `.claude/skills/review-response/LESSONS.md` — a conditional checklist (*if you changed X, check Y*) where every line was earned from a finding that actually happened on this repository and cites the pull requests it came from. Each rule carries an **origin** and two counters, all of them lists of dated evidence rather than numbers, so the count and the proof cannot drift apart. `origin` is the finding that created the rule — nothing could have prevented it. **missed** is the times the rule existed, was not applied, and a reviewer caught the mistake anyway; **saved** is the times it was read before a PR and caught it first. `saved / (saved + missed)` is the rule's hit rate. Separating the origin from a miss is the point of the shape: a miss is the only fact that criticises this file's own writing rather than the world, and lumping the two together hides it. Promotion keys on `missed`, and the first miss buys a rewrite of the sentence rather than a promotion — a rule that failed once may simply have been worded badly. At two, anything a script can decide becomes a check in `just ci`; at five, a rule that only judgment can decide earns a line in `CLAUDE.md`. Rules with a miss are hoisted into a *Failed before* block at the top of the file and demoted out of it after a release with no new miss, because attention is as scarce a budget as context: if everything is at the top, nothing is. `saved` is self-reported and is only recorded when the defect it caught can be named. Either way the line leaves the file, so it is bounded by graduation rather than by pruning. `/gate` reads it against the diff before a PR; `/respond` writes to it after a review. The CodeRabbit procedure, which had been restated in three places, now lives in one.
- CHANGELOG rotation. This file keeps `[Unreleased]` and the five most recent releases, newest first; `just changelog-rotate` archives the rest under `docs/changelog/`, one file per release, with an index. `just changelog-check` joins the merge gate and asserts both the limit and the ordering — a section added below an older one would otherwise archive the newest entry and publish the oldest one's notes.

### Fixed

- `scripts/rotate-changelog.py` validates a release heading against the semantic-version grammar before the version string becomes a filename — `## [../../CLAUDE]` is a legal Markdown heading — and refuses a symlink whatever it points at, since `exists()` follows one and a dangling link reads as absent. Rotation is also resumable: an archive page byte-identical to what the run would write is that run interrupted, not a collision, so a retry finishes instead of refusing. The first version of the collision guard made a half-finished rotation unrecoverable.
- Documentation swept before implementation begins. `README.md` said "three rules" where seventeen other places said five, and omitted the two that are enforced by review rather than by CI. `docs/ROADMAP.md`'s definition of done — which ninety-four filed issues link to — still said `just ci` rather than `just pre-pr`, and its table of `todo!()` seams cited line numbers, five of eleven of which were already wrong before a seam had been touched; the seams are now located by their `todo!()` message, which is stable and greppable. Two roadmap rows (M6-12, and most of M7-11 and M7-10) describe work the scaffold has already done, and now say so.
- The `rust:1.98.0-slim-bookworm` container in `ci.yml` and `release.yml` is pinned by digest. A Docker tag is exactly as mutable as an action tag, every `uses:` here already names a commit, and this is the container that builds the binary a release ships.
- Read-only `gh` commands are allowed in `.claude/settings.json`, so the documented `/respond` and `/file-issue` workflows do not prompt on every call. Anything that writes still asks.

- `send_durable`'s payload and acknowledgement now travel as one channel message (`WriteOp::Durable`) instead of two, closing a window where a saturated channel could accept the payload and then fail to enqueue the barrier that acks it — reporting `Degraded` for a write already queued for commit and inviting a retry that would double-submit it.
- `/health`'s `transposition_table.mode` now reads `engine.play.transposition_mode`, matching the section the adjacent `evaluator` field already reads, instead of `engine.lab.transposition_mode`.
- `FaceStatus::unloaded` reports `vram_budget_mb` from `limits.max_vram_mb` on the CUDA path instead of always `None`.
- `Board::from_bytes` now takes `format_version` as an explicit parameter, like `GameRecord::decode_moves`, so a caller cannot decode a persisted board without having looked at it (§13.7).
- Commentary sanitization now strips U+061C (Arabic Letter Mark) alongside the other bidirectional formatting controls (§7.7).
- CI: `actions/checkout` no longer persists Git credentials past the job; the `gate`/`cuda-compile`/`portable-build` jobs pin the toolchain to the exact version `rust-toolchain.toml` declares instead of `dtolnay/rust-toolchain`'s `stable`, which does not read that file; `check-cuda` now runs Clippy against the `cuda` feature, not just `cargo check`, so that path is actually linted; the nightly `load` job is disabled until `tests/load.rs`'s `todo!()` bodies are implemented, instead of failing every scheduled run.
- CI: every `uses:` across the workflows now names a commit rather than a tag, with the tag it belonged to in a trailing comment — a tag is a pointer its author can repoint at any time, and Dependabot maintains the pin and the comment together. `nightly.yml` gained a concurrency group, so a scheduled run and a manual dispatch of the same suite cannot produce two baselines for one night.

### Not yet implemented

Move generation, tree search, the writer actor loop, the lab worker pool, and
GGUF loading. Each is a `todo!()` at the seam the architecture defines for it,
with the owning section named.

[Unreleased]: https://github.com/garnizeh/draughts/commits/main
