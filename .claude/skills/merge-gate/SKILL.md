---
name: merge-gate
description: Run and interpret the draughts gate — just pre-pr, which is every CI job locally, and just ci within it (formatting, clippy, the suite, doc examples, and the static checks for device construction, format_version, the CHANGELOG, the documentation links and the seam lists) — plus the nightly suites (test-tt-off, test-load, bench). Also owns the cleanup after a PR lands — back to main, fast-forward, delete the branch locally and on origin, prune the stale tracking refs. Use when finishing a change, when CI is red, when a clippy or rustdoc warning needs fixing, when deciding whether something belongs in the gate or in nightly, and immediately after a merge.
---

# The gate

```bash
just pre-pr     # every CI job, locally — this is the pre-PR check
just ci         # one of those six jobs
```

`just ci` is the prerequisite list of the `ci` recipe, in that order, and it is exactly what `ci.yml`'s `gate` job invokes — `just --list` names the steps, and the triage table below has a row per failure. There is one definition of "green" and it is the justfile. Never hand-roll a `cargo` command in place of a recipe: a passing hand-rolled command that differs from the recipe is a false green.

**`just ci` is one of six jobs, not the whole workflow.** `ci.yml` also runs
`portable-build`, `cuda-compile` (`just check-cuda` + `just build-cuda`),
`supply-chain` (`just audit`), `workflows` (actionlint over
`.github/workflows/`), and `coverage` (`just coverage`) — none reproduced by
`just ci` itself.

**`just pre-pr` runs all six, here.** Use it before opening a PR, and use it instead of `just ci` when the question is "is this ready". Its order is deliberately not `ci.yml`'s: `just` stops the prerequisite chain at the first failure, so the recipe is sorted by what a failure would *mean* — the `justfile` owns that order and says why, and it is not restated here. Everything `pre-pr` finds is something a pushed branch would have found a round trip later, on someone else's runner. Two honest caveats:

- `portable-check` builds outside a driverless container, so it is weaker than
  `ci.yml`'s `portable-build`. CI remains the authority on §22.1.
- `check-cuda` and `build-cuda` need a CUDA toolkit on this host. Without one
  they fail on the toolkit, not on the tree. Say so and let CI answer — do not
  report `pre-pr` as green when part of it did not run. An unrun check reported
  as green is worse than a red one.

`coverage` reports and never gates: there is no percentage threshold, because a
threshold against a tree whose unimplemented seams are `todo!()` would measure
the seams and be gamed by deleting them. The lcov file is an artifact and the
summary is in the run's step summary.

**A change is not done until `just pre-pr` is green, and you have seen it.** Report the real output. If it fails and you are out of scope to fix it, say which recipe failed and why.

## Triage

| Failure | What it means | Fix |
|---|---|---|
| `fmt-check` | Tree is not formatted | `just fmt` |
| `lint` | Clippy, warnings denied, all targets, all features | Fix the lint. `#[allow]` needs a comment saying why, in the house style |
| `test` | The suite | Read the test name — it names the property that broke |
| `test-docs` | A doc example failed. `just test` cannot catch this: `--all-targets` means every target, and a doctest is not one | Fix the example or the code. Never delete the example to make it pass — it is the only executable part of a doc comment |
| `device-check` | `candle_core::Device` constructed outside `src/face/device.rs` (§19.6.5) | Take the device as a parameter. See the `face-layer` skill |
| `format-version-check` | An insert does not name `format_version`, or `src/db` decodes without referencing `CURRENT_FORMAT_VERSION` (§20.8) | See the `persisted-format` skill |
| `doc-links` | A relative link points at a file that is not there, or a `#anchor` at a heading that is not there | Fix the link. If a heading was renamed, every reference to it moved — the check names all of them at once |
| `source-citations` | A `todo!()` seam that neither seam list names, or a document citing a source line number | Add the seam to `docs/ROADMAP.md` and the `implement-seam` skill, quoting the opening words of its `todo!()` message. Replace a line number with that message: it carries its own § and the compiler will not let it drift from the code |
| `changelog-check` | `CHANGELOG.md` is out of order, or holds more than five released sections | `just changelog-rotate`. If it is an ordering complaint, the new section went in below an older one — newest first |
| `changelog-rotate-symlinks-check` | `rotate-changelog.py` wrote through a symlinked `docs/changelog/` or a symlinked `docs/changelog/README.md` on one of its two write paths | Fix the write path that failed — both the ordinary rotation and the `over <= 0` recovery branch must call `archive_containment_problem()` and let `write_index()` refuse a symlinked target before writing |
| `docs` | Broken intra-doc link, `RUSTDOCFLAGS=-D warnings` | Fix the link; do not drop the doc comment |
| `portable-check` | The default binary failed to build, to run, or to prove its linkage (§22.1, §19.6.5) | Read which of the three failed. A linkage failure means something pulled CUDA into the default build — see the `face-layer` skill |
| `audit` | `cargo-deny` found an advisory, a banned or duplicated crate, a licence outside the allowlist, or an unexpected source | Fix the dependency. `deny.toml` is the policy, and widening it is a decision rather than a fix |
| `coverage` | `just coverage` failed to *run* — never a percentage | Usually the same failure `just test` would give; read it as a test failure |
| `workflows-check` | actionlint found a real error in `.github/workflows/` — a bad expression, an undefined output, a shell mistake | Fix the workflow. CI runs the same binary through reviewdog with `filter_mode: nofilter`, so a finding may be in a file this PR did not touch and is still worth fixing |
| `check-cuda`, `build-cuda` | Either the `cuda` feature stopped compiling, or this host has no CUDA toolkit | Two different things — read the error before blaming the tree. No toolkit here means naming the recipe and letting CI answer, never rounding to green |

Two failures that look like flakes and are not:

- **A changed Zobrist fingerprint.** The test pins the generated key table.
  Regenerating it silently invalidates every persisted `board_hash`. That is a
  `format_version` bump, not an expected-constant edit.
- **A search test that passes at 1 thread and fails at 8.** The transposition
  table has become load-bearing for correctness. See the
  `transposition-safety` skill — this is rule 2, and it is a blocker.

## Outside `just ci`, on purpose

Two different things get called "outside the gate", and conflating them is how a job quietly stops being run.

**Inside `pre-pr`, outside `just ci`.** `portable-check`, `audit`, `coverage`, `workflows-check`, `check-cuda` and `build-cuda` each run on every pull request as their own `ci.yml` job. They are not in `just ci` because they need a container, a network, or a tool the `gate` job does not have — never because they are optional.

**Outside the gate entirely, and nightly-only.** `nightly.yml` runs exactly two suites, and neither can block a merge:

```bash
just test-tt-off     # §5.4 — the search suite with the table disabled
just bench           # §20.9 — criterion baselines, tracked as numbers not pass/fail
```

`just test-load` (§20.4 — millions of rows and a peak-RSS gate, minutes to hours) runs in neither: its `nightly.yml` job is commented out until `tests/load.rs`'s `todo!()` bodies exist. Do not add any of these three to `just ci`; do run the relevant one when you have touched what it covers.

`just bench` produces baselines, not assertions. A run slower than yesterday is a
question, not a failure — and a new number belongs in
[Appendix B](../../../docs/architecture/appendix-b-performance-targets.md), not
in a comment.

## The CUDA half

CI's `cuda-compile` job runs both of these on every PR, and runs nothing on a
device — `build-cuda` links against the installed toolkit but never executes
the binary it produces:

```bash
CUDA_COMPUTE_CAP=86 just check-cuda   # cargo check + clippy, no device needed
CUDA_COMPUTE_CAP=86 just build-cuda   # an actual release build; needs a toolkit
```

The device-requiring half of §20.10 runs on the target host only. Making it a
merge gate would reintroduce the GPU dependency the section exists to prevent.
If `cudarc`'s supported toolkit range lags the installed driver, install a
toolkit in range — that is the fix, not a driver downgrade (§22.1).

CI also builds the default binary in a container with **no driver and no
toolkit**, runs it, and asserts its linkage with
`scripts/check-no-cuda-linkage.sh` — one definition of that check, shared with
`just package portable`, so the binary CI checks and the binary a release ships
are checked the same way. A missing CUDA library shows up at load time, not at
link time, which is why the run matters as much as the build.

## Before opening a PR

1. **`just pre-pr`** — and read its output. This is the whole of the mechanical
   check, and running `just ci` in its place is the mistake this recipe exists
   to remove.
2. `just test-tt-off` if you touched search or the table.
3. `CHANGELOG.md` updated, under `[Unreleased]`, newest first.
4. Every new constant carries its §.
5. If a recipe could not run for want of a tool, `just setup` — and if it still
   cannot (no CUDA toolkit here), name it rather than rounding it to green.
6. `.claude/skills/review-response/LESSONS.md` read against this diff. It is a
   short conditional checklist earned from real findings on this repository —
   the things no script catches yet. Reading it costs a minute and is the
   cheapest place left to catch anything on it.

## Releasing is a different gate

`release.yml` is not part of the merge gate and never runs on a pull request. It
watches `main` for a version bump whose CHANGELOG section is closed, cuts the
tag itself, and re-runs `just ci` at that tag before building anything. Do not
run `git tag` — see the `releasing` skill, which owns that procedure.

## After CodeRabbit reviews

Every finding gets a reply on its own thread — pointing at the fix, or stating why it stands as is — before the PR is done. A PR is not finished with unanswered threads any more than it is finished with a red `just pre-pr`.

The procedure belongs to the **`review-response`** skill, which owns it end to end: the endpoint that silently omits half a review, how to verify a finding against code that has moved since, and the phase most people skip — deciding whether a finding named a class worth a permanent check, and writing what the review taught into `.claude/skills/review-response/LESSONS.md` so the next one starts from it. Load that skill rather than working from memory; two of PR #99's four findings had a correct diagnosis attached to a fix that would have made things worse, and that is the ordinary case.

## After the merge

A merged PR is not finished when GitHub says "Merged". It is finished when this working copy is back on an up-to-date `main` and the branch is gone from both sides. Neither half happens on its own: the local branch survives the merge, and so does the remote-tracking ref for a remote branch the server has already deleted. Skipped once, that is untidy; skipped for a month, `git branch` is a column of dead names and `git branch -r` advertises refs that do not exist, which is exactly the state in which the next branch gets cut from the wrong place.

```bash
gh pr view PR --json state,mergedAt,headRefName   # MERGED, not CLOSED
git switch main
git pull --ff-only origin main                    # take the merge, never rebuild it
git branch -d BRANCH                              # -d, never -D — see below
git push origin --delete BRANCH                   # only if the remote branch survived
git fetch --prune                                 # drop tracking refs the server already deleted
```

- **Confirm it merged before deleting anything.** `state` is `MERGED`; a PR that was closed unmerged reads `CLOSED` and its branch is the only copy of the work.
- **`git pull --ff-only`.** Without it, a stray local commit on `main` turns into a merge commit that nobody asked for and that the next push offers to the world. With it, the same situation is a loud failure, which is what it should have been.
- **`-d`, not `-D`.** `-d` refuses to delete a branch whose commits are not in `main`, which is the one guard standing between a mistyped branch name and lost work. **It also refuses routinely after a squash merge**, because the squashed commit is not the branch's commit — that refusal is expected and is not evidence of a problem. Prove the content landed before overriding it: `git diff main BRANCH` (two dots — same trees, not same history) prints nothing, and only then `git branch -D BRANCH`.
- **The remote half is usually already done.** This repository deletes the head branch on merge, so `git push origin --delete` will often fail with "remote ref does not exist" — that is success arriving early, not an error to chase. Run it when the branch is still listed, skip it when `git fetch --prune` already took the tracking ref away.
- **`git status` before any of it.** Uncommitted work in the tree is a reason to stop and ask, not to `git switch` over the top of it.

The same sequence applies after a release PR lands: the version bump is merged like anything else, and `release.yml` cuts the tag from `main` afterwards — see the `releasing` skill, and do not run `git tag` here either.
