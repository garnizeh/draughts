---
description: Prepare a release PR — bump the version, close the CHANGELOG section, verify
argument-hint: "x.y.z"
allowed-tools: Bash, Read, Edit, Glob, Grep
---

Prepare the release PR for version $ARGUMENTS. Load the `releasing` skill first
— it owns this procedure and explains why each step is in it.

Nothing else belongs in this change. A release commit that also changes
behaviour is a release whose notes are wrong.

1. `just version` — confirm what the tree currently claims to be, and that
   $ARGUMENTS is a sane next step from it.
2. Bump `version` in `Cargo.toml`, then `cargo update -p draughts` so
   `Cargo.lock` records it.
3. In `CHANGELOG.md`: rename `## [Unreleased]` to `## [$ARGUMENTS] - <today>`,
   open a fresh empty `## [Unreleased]` above it, and update the link references
   at the bottom. Read what is in the section — if it does not describe the
   change a reader will care about, fix the prose. These are the release notes;
   `release.yml` publishes them verbatim.
4. `just release-check $ARGUMENTS` — must print `$ARGUMENTS is ready`.
5. `just ci`.
6. Report the actual output, then stop. Do **not** create a tag: merging this PR
   is what cuts it.
