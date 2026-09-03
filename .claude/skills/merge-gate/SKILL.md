---
name: merge-gate
description: Run and interpret the draughts merge gate — just ci, fmt-check, lint, test, device-check, format-version-check, docs — plus the nightly suites (test-tt-off, test-load, bench, check-cuda). Use when finishing a change, when CI is red, when a clippy or rustdoc warning needs fixing, or when deciding whether something belongs in the gate or in nightly.
---

# The gate

```bash
just pre-pr     # every CI job, locally — this is the pre-PR check
just ci         # one of those six jobs
```

`just ci` is `fmt-check`, `lint`, `test`, `device-check`,
`format-version-check`, `changelog-check`, `docs` — in that order, and it is
exactly what `ci.yml`'s `gate` job invokes. There is
one definition of "green" and it is the justfile. Never hand-roll a `cargo`
command in place of a recipe: a passing hand-rolled command that differs from
the recipe is a false green.

**`just ci` is one of six jobs, not the whole workflow.** `ci.yml` also runs
`portable-build`, `cuda-compile` (`just check-cuda` + `just build-cuda`),
`supply-chain` (`just audit`), `workflows` (actionlint over
`.github/workflows/`), and `coverage` (`just coverage`) — none reproduced by
`just ci` itself.

**`just pre-pr` runs all six, here, in CI's order.** Use it before opening a
PR, and use it instead of `just ci` when the question is "is this ready".
Everything it finds is something a pushed branch would have found a round trip
later, on someone else's runner. Two honest caveats:

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

**A change is not done until `just ci` is green, and you have seen it.** Report
the real output. If it fails and you are out of scope to fix it, say which
recipe failed and why.

## Triage

| Failure | What it means | Fix |
|---|---|---|
| `fmt-check` | Tree is not formatted | `just fmt` |
| `lint` | Clippy, warnings denied, all targets, all features | Fix the lint. `#[allow]` needs a comment saying why, in the house style |
| `test` | The suite | Read the test name — it names the property that broke |
| `device-check` | `candle_core::Device` constructed outside `src/face/device.rs` (§19.6.5) | Take the device as a parameter. See the `face-layer` skill |
| `format-version-check` | An insert does not name `format_version`, or `src/db` decodes without referencing `CURRENT_FORMAT_VERSION` (§20.8) | See the `persisted-format` skill |
| `doc-links` | A relative link points at a file that is not there, or a `#anchor` at a heading that is not there | Fix the link. If a heading was renamed, every reference to it moved — the check names all of them at once |
| `changelog-check` | `CHANGELOG.md` is out of order, or holds more than five released sections | `just changelog-rotate`. If it is an ordering complaint, the new section went in below an older one — newest first |
| `docs` | Broken intra-doc link, `RUSTDOCFLAGS=-D warnings` | Fix the link; do not drop the doc comment |
| `workflows` (CI job, no recipe) | actionlint found a real error in `.github/workflows/` — a bad expression, an undefined output, a shell mistake | Fix the workflow. `filter_mode: nofilter`, so a finding may be in a file this PR did not touch, and it is still worth fixing |
| `coverage` (CI job) | `just coverage` failed to *run* — never a percentage | Usually the same failure `just test` would give; read it as a test failure |

Two failures that look like flakes and are not:

- **A changed Zobrist fingerprint.** The test pins the generated key table.
  Regenerating it silently invalidates every persisted `board_hash`. That is a
  `format_version` bump, not an expected-constant edit.
- **A search test that passes at 1 thread and fails at 8.** The transposition
  table has become load-bearing for correctness. See the
  `transposition-safety` skill — this is rule 2, and it is a blocker.

## Outside the gate, on purpose

These are slow, large, or device-dependent. They run nightly. Do not add them to
`just ci`; do run the relevant one when you have touched what it covers.

```bash
just coverage        # §20 — lcov + a summary table. Reported, never gated
just test-tt-off     # §5.4 — the search suite with the table disabled
just test-load       # §20.4 — millions of rows, a peak-RSS gate. Minutes to hours
just bench           # §20.9 — criterion baselines, tracked as numbers not pass/fail
just check-cuda      # §20.10 — the cuda feature compiles, no device needed
just audit           # cargo-deny: advisories, bans, licences, sources
```

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
