---
name: merge-gate
description: Run and interpret the draughts merge gate — just ci, fmt-check, lint, test, device-check, format-version-check, docs — plus the nightly suites (test-tt-off, test-load, bench, check-cuda). Use when finishing a change, when CI is red, when a clippy or rustdoc warning needs fixing, or when deciding whether something belongs in the gate or in nightly.
---

# The gate

```bash
just ci
```

is `fmt-check`, `lint`, `test`, `device-check`, `format-version-check`, `docs` —
in that order, and it is exactly what `ci.yml`'s `gate` job invokes. There is
one definition of "green" and it is the justfile. Never hand-roll a `cargo`
command in place of a recipe: a passing hand-rolled command that differs from
the recipe is a false green.

**`just ci` is one of four required jobs, not the whole workflow.** `ci.yml`
also runs `portable-build`, `cuda-compile` (`just check-cuda` + `just
build-cuda`), and `supply-chain` (`just audit`) — each required on every PR,
none reproduced by `just ci` itself. A PR isn't green until all four are; see
"Before opening a PR" below for the two of those `just ci` doesn't cover.

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
| `docs` | Broken intra-doc link, `RUSTDOCFLAGS=-D warnings` | Fix the link; do not drop the doc comment |

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
toolkit**, runs it, and greps `objdump -p` for a CUDA `NEEDED` entry. A missing
CUDA library shows up at load time, not at link time, which is why the run
matters as much as the build.

## Before opening a PR

1. `just ci` — the `gate` job.
2. `just check-cuda`, `just build-cuda`, and `just audit` — unconditionally.
   All three are required CI jobs (`ci.yml`'s `cuda-compile` and
   `supply-chain`), not only when you touched a feature-gated file: a
   shared-code change can break any of them.
3. `just test-tt-off` if you touched search or the table
4. `CHANGELOG.md` updated
5. Every new constant carries its §

## After CodeRabbit reviews

Every finding gets a reply on its own thread — pointing at the fix, or stating
why it stands as is — before the PR is done. See `CONTRIBUTING.md`. A PR is not
finished with unanswered CodeRabbit threads any more than it is finished with a
red `just ci`.

The per-line comments (`gh api repos/OWNER/REPO/pulls/PR/comments`) are not the
whole review. Findings CodeRabbit cannot anchor to a changed line live in the
**"⚠️ Outside diff range comments"** section of the review body itself — pull
every review's full body (`gh api repos/OWNER/REPO/pulls/PR/reviews`, then each
`.body`) and check that section, or these get silently skipped. See
`CONTRIBUTING.md` for the reply convention for findings that have no comment id
to reply on.
