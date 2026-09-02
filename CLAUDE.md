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
just ci                 # the merge gate: fmt-check lint test device-check format-version-check docs
just test               # the suite
just test-one NAME      # one test or module, with output
just check              # type-check, no binary
just run                # server against draughts.toml
just check-config       # §23.1 validation against this host, no db, no port
just bench              # criterion baselines (§20.9)
```

Outside `just ci` but still required on every PR, as separate `ci.yml` jobs:
`just check-cuda`, `just build-cuda`, `just audit`. Outside the gate
entirely, deliberately, and nightly-only: `just test-tt-off`, `just bench`.
`just test-load` runs neither — its CI job is commented out in `nightly.yml`
until `tests/load.rs`'s `todo!()` bodies are implemented.

## Working in this tree

- **Everything that lands in the repository is written in English.** Code,
  comments, documentation, tests, commit messages, branch names, CHANGELOG
  entries, and GitHub milestones, issues and pull requests — all English,
  without exception, whatever language the conversation is being held in. The
  tree has one voice and a contributor should not have to switch languages to
  read it.
- **Finish at the gate.** A change is not done until `just ci` is green. Report
  the actual output; do not describe a run you did not make.
- **The unfinished parts are `todo!()` at named seams.** Each carries the section
  that owns it. Implement against that section, not against a guess. The
  `implement-seam` skill has the procedure.
- **Comments explain *why*, and cite the § that decided it.** This codebase is
  full of constants that look arbitrary and are not. A bare number with no
  citation is the thing most likely to be "cleaned up" into a bug. Match the
  density and voice of the surrounding comments — they are prose, not labels.
- **Tests are named as the property they assert**, not as the function they call:
  `the_default_build_resolves_every_request_to_cpu`, not `test_select_device`.
- **New performance numbers go in
  [Appendix B](docs/architecture/appendix-b-performance-targets.md)**, not in a
  comment.
- **Do not weaken a check to make it pass.** If `just test` objects to a changed
  Zobrist fingerprint, that is a `format_version` bump, not an expected-constant
  edit.
- Errors flow through `error::ApiError` with a stable `code()` string that is
  never derived from the variant name (§9.1).
- `anyhow` at the binary and seam boundaries, `thiserror` for typed domain
  errors. No `unwrap()` on a path that can be reached by a request.
- **Every CodeRabbit review comment gets a reply, on its own thread, before the
  PR is done.** Verify the finding against the current code first — it may
  already be stale. Reply pointing at the commit and line that fixes it, or
  state plainly why it is not being fixed (a documented seam, a design
  question for a human, out of scope). CodeRabbit reads the reply and resolves
  or re-argues the thread from it; a comment nobody answered is a review
  nobody read. One consolidated PR comment is not a substitute — reply on the
  thread itself so CodeRabbit's own resolution logic sees it.
  - **Fetching "every comment" via `gh api --paginate repos/.../pulls/{n}/comments`
    is not enough — it silently omits "Outside diff range comments".**
    CodeRabbit puts findings it can't anchor to a changed line inside the
    review's own `body` text (`gh api --paginate
    repos/.../pulls/{n}/reviews/{review_id}` → `.body`, under a collapsible
    `⚠️ Outside diff range comments` section), not as separate comment objects
    with their own ids — they never appear in the comments endpoint and are
    the easiest findings to miss entirely. `--paginate` on both calls, not
    just one: a PR with enough reviews or comments to span more than one page
    silently drops the rest without it. Before calling a CodeRabbit review
    answered, list every review on the PR and read each one's full `body`,
    not just the per-line comments it produced. Those findings have no
    comment id to reply on inline; answer them together in one PR comment
    that names each file and line, or wait for CodeRabbit to promote them to
    inline threads on a later pass.
  - **CodeRabbit keeps its status in the *first* comment on the PR, editing
    that one comment in place for the life of the review. It never posts a new
    comment to say the review is finished.** Reading "the latest comment" is
    therefore the wrong move and the easy mistake: the newest thing on the PR
    is a round of findings, or somebody's reply, or your own — none of which
    say whether CodeRabbit is still working. On #95 the status comment was
    created at 14:43, before the first review, and last edited at 21:55, after
    the last one: one comment id, rewritten all day, while six separate review
    bodies came and went beneath it. It holds the walkthrough, the pre-merge
    checks, the commit range last reviewed, and the one sentence that decides
    whether we may merge:

    > No actionable comments were generated in the recent review. 🎉

    **A PR is mergeable only once that sentence is in that comment.** A green
    gate does not say it — CI and the review are independent, so the checks can
    pass while a review is still running, and a review that produced findings
    simply leaves the sentence absent. Read it with `gh api --paginate
    repos/.../issues/{n}/comments --jq '.[0].body'` — `.[0]`, the oldest
    comment, never the newest — and check the `📥 Commits` block inside the
    same comment for the range it covered (`Reviewing files that changed ...
    between <base> and <head>`). The sentence speaks only for that run, so one
    left over from before the last push says nothing about what was pushed
    after it. Merge without it only when the user says so explicitly.

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
