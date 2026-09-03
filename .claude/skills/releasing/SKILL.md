---
name: releasing
description: Cut a draughts release — the version in Cargo.toml, the CHANGELOG section that gates it, `just release-check`, `just package`, and what release.yml does once a version bump lands on main. Use when asked to release, tag, cut a version, publish a build, prepare release notes, or when a release workflow run is red or produced no tag.
---

# Releasing

**The version in `Cargo.toml` is the source of truth. `CHANGELOG.md` is the
gate. Nobody runs `git tag`.**

Merging a version bump to `main` is the whole of the ritual. `release.yml`'s
`tag` job runs on every push to `main`, asks two questions, and almost always
answers "not a release":

1. Does `v$(just version)` already exist as a tag? Then this commit is not a
   release.
2. Does `CHANGELOG.md` have a **closed** section for that version — a dated
   `## [x.y.z] - YYYY-MM-DD` heading, not `[Unreleased]`? If not, this commit is
   not a release.

Only when the version is new *and* its notes are written does it cut an
annotated tag and build from it. That ordering is the point: nothing ships whose
notes nobody wrote, and no bot rewrites the CHANGELOG — it stays prose, in one
voice, like every other file in the tree.

## Cutting one

A release is an ordinary pull request. It contains only these:

```bash
just version                    # what the tree currently claims to be
```

1. Bump `version` in `Cargo.toml`.
2. `cargo update -p draughts` so `Cargo.lock` records the new version. A stale
   lock means the tarball is built from a dependency graph nobody wrote down,
   and `release-check` fails on it.
3. In `CHANGELOG.md`, rename `## [Unreleased]` to `## [x.y.z] - YYYY-MM-DD` and
   open a fresh empty `## [Unreleased]` above it. **Newest first** — the new
   section goes directly under `[Unreleased]`, above every older one. Add the
   link references at the bottom.
4. `just changelog-rotate` if that took the file past five released sections.
   Read `.claude/skills/review-response/LESSONS.md` in the same pass: demote any
   rule out of its *Failed before* block that has gone this release without a
   new miss, and consider deleting any rule cold in both counters since the last
   one. A release is the only clock that file has.
5. `just release-check x.y.z` — must print `x.y.z is ready`.
6. `just pre-pr`.
7. Open the PR, merge it. The tag appears by itself; so does the release.

Nothing else belongs in that PR. A release commit that also changes behaviour is
a release whose notes are wrong.

## The recipes

```bash
just version                     # the crate version, from Cargo.toml
just release-notes x.y.z         # the CHANGELOG section; non-zero if not closed
just release-check x.y.z         # everything that must hold before tagging
just changelog-check             # newest first, at most five released sections
just changelog-rotate            # archive the rest into docs/changelog/
just package x.y.z portable      # dist/draughts-x.y.z-x86_64-unknown-linux-gnu.tar.gz
just package x.y.z cuda          # ...-cuda.tar.gz, plus a CUDA.md host note
```

`release-check` asserts four things, and each failure names itself:

| It says | It means |
|---|---|
| `'…' is not a semantic version` | The argument, not the tree |
| `Cargo.toml says A, release says B` | You are packaging a version the tree does not claim to be |
| `Cargo.lock does not record draughts x.y.z` | `cargo update -p draughts` |
| `CHANGELOG.md has no dated '## [x.y.z] - YYYY-MM-DD' heading` | The section is still `[Unreleased]`, or undated |

## The CHANGELOG stays five deep

`CHANGELOG.md` holds `[Unreleased]` and the five most recent releases, newest
first. Older sections are archived by `just changelog-rotate` into
`docs/changelog/<version>.md`, one file per release, with an index regenerated
alongside them. `just changelog-check` is part of `just ci`, so the limit and
the ordering are enforced rather than remembered.

The ordering is not cosmetic. `release.yml` publishes the section for the
version being released, and the rotation archives the *tail* of the list — a
section added below an older one would archive the newest entry and publish the
oldest one's notes. That is why the check fails on it rather than sorting it.

One release per archive file, and a file's name never changes once written: a
link to it does not rot and `git log` follows it. Do not edit an archived
section — its GitHub release notes were rendered from it at publish time.

## What ships

Two tarballs, Linux x86-64 only, each with a `.sha256` verified in CI before the
release is published. Both carry `draughts.example.toml`, `README.md`,
`CHANGELOG.md` and `LICENSE` alongside the binary.

- **portable** — default features. Built inside `rust:1.98.0-slim-bookworm`,
  a container with no driver and no toolkit, then *run* there:
  `--version` and `--check-config` against the committed example. Its linkage is
  asserted by `scripts/check-no-cuda-linkage.sh`, not assumed. This is the
  binary §22.1 promises runs on a machine with no driver.
- **cuda** — `--features cuda`, toolkit 12.6.0, `CUDA_COMPUTE_CAP=86`. Built,
  never run: a device is not needed to link, and requiring one would put a GPU
  back in the release path that §20.10 spent a section removing. It carries a
  `CUDA.md` saying plainly that it needs the CUDA runtime on the host — the
  feature adds a device to the *engine*, but it adds a requirement to the
  *executable*.

The `.gguf` models are not in either tarball, for the reason §22.1 gives.

## When the workflow did something you did not expect

| Symptom | Cause |
|---|---|
| A version bump merged and no tag appeared | The CHANGELOG section was still `[Unreleased]`, or undated. The `tag` job says so as a `::notice::` and exits green — this is the designed answer, not a failure |
| `tag` job red at `release-check` | The tree claims to be a release and contradicts itself. Read which of the four assertions fired |
| `resolve` job red: `'…' is not a vX.Y.Z tag` | A hand-pushed tag that is not `vMAJOR.MINOR.PATCH` |
| `verify` red but `ci.yml` was green on the same commit | Almost always a hand-cut tag on a commit that never passed the gate |
| `publish` red at the checksum step | An artifact arrived corrupt. Re-run; do not publish past it |
| Release created but marked pre-release | The version carries a SemVer pre-release identifier (`0.3.0-rc.1`). That is correct |

To rebuild an existing tag — a lost artifact, not a new version — use the
workflow's `workflow_dispatch` with the tag name. It never creates a tag.

## Things not to do

- **Do not `git tag`.** The automatic path exists so that the tag and the
  CHANGELOG cannot disagree. A hand-cut tag skips the CHANGELOG gate and makes
  `verify` the only thing standing between an untested commit and a release.
- **Do not edit a published CHANGELOG section**, in this file or in an archived
  one. The notes on the GitHub release were rendered from it at publish time;
  editing it afterwards makes the two disagree with no way to tell which is
  right.
- **Do not raise `KEEP` in `scripts/rotate-changelog.py` to avoid rotating.**
  Five is "enough that the recent past is on one screen". The archive is where
  the rest belongs, and it is indexed.
- **Do not add a target.** Linux x86-64 is deliberate — it is the deployment
  §22.1 describes, and every additional triple is a build nobody runs.
