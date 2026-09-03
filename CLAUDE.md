# CLAUDE.md

`draughts` — a single-binary English-draughts engine and self-play training lab
in Rust. One process, one SQLite file, no sidecars. An in-process quantized LLM
writes commentary and is never permitted to touch the game.

**The tree is written against a settled architecture.** `docs/architecture/` is
version 1.4, reviewed and approved; the code is the part that is unfinished.
When the code and the document disagree, the document is right and the code is a
defect — unless you can show the document is wrong, in which case say so before
writing anything.

## Where to look before writing code

| Question | Answer lives in |
|---|---|
| What does this module owe the rest of the system? | [§4](docs/architecture/04-separation-of-concerns.md), [§5](docs/architecture/05-runtime-components.md) |
| Engine, search, transposition table | [§6](docs/architecture/06-mcts-extensibility.md) |
| Commentary, device selection, circuit breaker | [§7](docs/architecture/07-face-llm-layer.md) |
| HTTP contract | [§9](docs/architecture/09-api-contract.md) |
| Persistence, schema, columns | [§11](docs/architecture/11-database-architecture.md), [§12](docs/architecture/12-database-schema.md), [§13](docs/architecture/13-data-dictionary.md) |
| Concurrency, memory budgets | [§15](docs/architecture/15-concurrency-model.md), [§16](docs/architecture/16-memory-strategy.md) |
| What a test must assert | [§20](docs/architecture/20-testing-strategy.md) |
| Configuration and startup validation | [§23](docs/architecture/23-configuration-example.md) |
| What "done" means | [§25](docs/architecture/25-acceptance-criteria.md) |

The index with every section is [docs/architecture/README.md](docs/architecture/README.md).
Use the `architecture-map` skill to find the right § quickly.

## The five rules

The first three are review blockers on their own; the design rests on them.

1. **Reading a persisted BLOB without dispatching on its `format_version` is a
   defect** ([§13.7](docs/architecture/13-data-dictionary.md)). The column's
   `DEFAULT` exists for the v1.0 migration and for nothing else. Every insert
   names it explicitly, from `CURRENT_FORMAT_VERSION`.
2. **The transposition table may change how long a search takes and must never
   change what it returns** ([§20.5](docs/architecture/20-testing-strategy.md#205-transposition-table-tests)).
   Anything touching `probe`, `store`, or `EvaluatorIdentity` needs the
   differential search test green at 1, 2, 8 and 10 threads.
3. **`candle_core::Device` is constructed in exactly one function** —
   `face::device::select_device`
   ([§19.6.5](docs/architecture/19-extensibility-roadmap.md)). CI greps for this.
4. **The LLM never plays.** `CommentaryContext` is the whole of what the Face
   layer is given, and there is no move in it
   ([§2.3](docs/architecture/02-scope-and-constraints.md)). Adding a field to
   that struct is an architectural change, not a convenience.
5. **A change that can only be tested on a GPU has broken the CPU path.** The
   whole suite runs on `face.device = "cpu"`, on the default build, on a runner
   with no driver ([§20.10](docs/architecture/20-testing-strategy.md#2010-device-parity-and-cuda-tests)).

## Commands

`just` is the only entry point. Never invent a `cargo` incantation when a recipe
exists — CI runs the recipes, so a recipe and a green pipeline mean the same
thing by construction.

```bash
just                    # list every recipe
just pre-pr             # every CI job, locally. The pre-PR check
just ci                 # one of six; `just --list` names its steps
just test               # the suite
just test-one NAME      # one test or module, with output
just check              # type-check, no binary
just run                # server against draughts.toml
just check-config       # §23.1 validation against this host, no db, no port
just bench              # criterion baselines (§20.9)
just coverage           # lcov + a summary. Reported, never gated
```

`ci.yml` runs **six jobs**, of which `just ci` is one. The other five are
`portable-build` (`just portable-check` here), `cuda-compile` (`just check-cuda`
*and* `just build-cuda` — two recipes, one job), `supply-chain` (`just audit`),
`workflows` (actionlint), and `coverage` (`just coverage`).

**`just pre-pr` runs all six, as seven prerequisites** — use it, not `just ci`, when the question is whether a change is ready. Its order is deliberately not `ci.yml`'s: `just` stops the prerequisite chain at the first failure, so the recipe is sorted by what a failure would *mean* rather than by what CI happens to list first. The `justfile` owns that order and states its reasoning; it is not restated here, because a list whose order is decided elsewhere goes stale wherever it is copied. The two things `pre-pr` cannot fully reproduce say so: `portable-check` builds outside a driverless container, and the CUDA recipes need a toolkit on this host. Outside the gate entirely, deliberately, and nightly-only: `just test-tt-off`, `just bench`. `just test-load` runs neither — its CI job is commented out in `nightly.yml` until `tests/load.rs`'s `todo!()` bodies are implemented.

Releasing is not part of the gate and never runs on a pull request:

```bash
just version            # what the tree claims to be; Cargo.toml is the truth
just release-check X.Y.Z    # everything that must hold before tagging
just package X.Y.Z portable # dist/…-x86_64-unknown-linux-gnu.tar.gz
```

**Never run `git tag`.** `release.yml` watches `main` for a version bump whose
`CHANGELOG.md` section is *closed* — a dated `## [x.y.z] - YYYY-MM-DD` heading,
not `[Unreleased]` — and cuts the tag itself. Merging that bump is the whole
ritual; a hand-cut tag skips the only thing guaranteeing the notes exist. The
`releasing` skill owns the procedure.

## Working in this tree

- **Everything that lands in the repository is written in English.** Code,
  comments, documentation, tests, commit messages, branch names, CHANGELOG
  entries, and GitHub milestones, issues and pull requests — all English,
  without exception, whatever language the conversation is being held in. The
  tree has one voice and a contributor should not have to switch languages to
  read it.
- **Finish at the gate.** A change is not done until `just pre-pr` is green —
  every job CI runs, run here. Report the actual output; do not describe a run
  you did not make, and never round a check that did not run up to green.
- **A merge is not finished until the branch is gone.** Once a PR is merged: back to `main`, `git pull --ff-only origin main`, `git branch -d` the branch, delete it on `origin` if GitHub's delete-on-merge has not already, then `git fetch --prune` so no tracking ref outlives the branch it names. The `merge-gate` skill owns the sequence and its two traps — `-d` refuses on every branch, because squash is this repository's only merge method, and the remote delete usually fails because delete-on-merge already did it.
- **The unfinished parts are `todo!()` at named seams.** Each carries the section
  that owns it. Implement against that section, not against a guess. The
  `implement-seam` skill has the procedure.
- **Comments explain *why*, and cite the § that decided it.** This codebase is
  full of constants that look arbitrary and are not. A bare number with no
  citation is the thing most likely to be "cleaned up" into a bug. Match the
  density and voice of the surrounding comments — they are prose, not labels.
- **Tests are named as the property they assert**, not as the function they call:
  `the_default_build_resolves_every_request_to_cpu`, not `test_select_device`.
- **`CHANGELOG.md` is newest-first and holds five releases.** `[Unreleased]`
  first, then the five most recent; older ones are archived under
  `docs/changelog/`, one file per release, by `just changelog-rotate`.
  `just changelog-check` is in the gate, so a section added in the wrong place
  fails locally rather than growing a file nobody reads.
- **New performance numbers go in
  [Appendix B](docs/architecture/appendix-b-performance-targets.md)**, not in a
  comment.
- **Every `uses:` in a workflow names a commit**, with the tag it belonged to in
  a trailing comment (`@3d3c42e… # v7`). A tag is a pointer its author can
  repoint; a commit is not. Dependabot bumps the pin and the comment together,
  so do not "tidy" the comment away. The same applies to `container:` and
  `image:` — a Docker tag is as mutable as an action tag, and the one this tree
  uses builds the binary a release ships, so it is pinned by digest.
- **The documentation cites itself several hundred times, and `just doc-links`
  is what keeps that true.** Renaming a heading breaks every reference to it
  silently; the check names all of them at once.
- **Do not hard-wrap prose at a column count.** Not at 80, not at any number. Markdown, commit-message bodies, PR and issue text, YAML comments: one paragraph is one line, and a long paragraph breaks at a sentence boundary if it breaks at all. Every renderer that shows this text reflows it, monitors are wide, and a fixed wrap makes a one-word edit rewrite the whole paragraph in the diff. Two exceptions, and they are the only two. **Source files follow their language's own convention** — Rust `rustfmt.toml`'s `max_width = 100` under `just fmt-check`, Python PEP 8 — and comments inside them match the code they sit in, because source does not reflow and a comment wider than the code it annotates reads badly in a split editor. And a commit *subject* line stays under ~72, because `git log --oneline`, `git shortlog` and GitHub's UI all truncate it. Nothing else has a column limit — Markdown, commit bodies, PR and issue text, YAML comments: none.
- **Existing files are 80-wrapped; unwrap what you edit.** Do not mass-rewrap a file you are not otherwise touching — that buries a real change under a formatting diff.
- **Do not weaken a check to make it pass.** If `just test` objects to a changed
  Zobrist fingerprint, that is a `format_version` bump, not an expected-constant
  edit.
- Errors flow through `error::ApiError` with a stable `code()` string that is
  never derived from the variant name (§9.1).
- `anyhow` at the binary and seam boundaries, `thiserror` for typed domain
  errors. No `unwrap()` on a path that can be reached by a request.
- **Every CodeRabbit review comment gets a reply, on its own thread, before the PR is done.** Verify the finding against the current code first — it may already be stale, and its stated consequence is the part reviewers get wrong most often. Reply pointing at the commit and line that fixes it, or state plainly why it is not being fixed. CodeRabbit resolves or re-argues from the reply; one consolidated PR comment leaves every thread looking unanswered. The exception is a finding with no comment id — the "Outside diff range" kind — which has no thread to reply on: answer those together in one PR comment naming each file and line, or wait for a later pass to promote them to inline threads.
  - The procedure is owned by the **`review-response`** skill, and it is worth loading rather than working from memory. It carries the trap that `gh api .../pulls/{n}/comments` silently omits "Outside diff range comments" — those live inside each review's own `body`, have no comment id, and are the easiest findings to miss entirely. It also carries the phase that makes a review worth having: deciding whether a finding named a *class* worth a permanent check, and recording the verdict in `.claude/skills/review-response/LESSONS.md` so a class seen twice stops being treated as a one-off.

## Layout

```
src/
  api/      axum routes, AppState, health          §5.2 §9
  config/   types + §23.1 startup validation       §23
  db/       pool, migrations, writer actor, ids    §5.6 §11 §12 §13
  engine/   mcts, evaluator, transposition         §5.4 §6
  face/     device, breaker, candle, canned, prompt §5.7 §7
  lab/      runner, sampling                       §5.5 §14
  rules/    board, moves, zobrist                  §5.3
```

Calls run downward only: `api` → services → (`rules`, `engine`, `lab`, `face`) →
(`db::writer`, `engine::transposition`). No module below reaches back up.

## Scope

The status is scaffolding: it builds, validates its configuration, and does not
play yet. Move generation, tree search, the writer actor loop, the lab worker
pool and GGUF loading are unimplemented. [CHANGELOG.md](CHANGELOG.md) records
what exists — update it when you add to that list.
