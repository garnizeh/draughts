---
description: Run the full merge gate (just ci) and fix what it finds
argument-hint: "[optional: extra context or a recipe to run instead]"
allowed-tools: Bash, Read, Edit, Glob, Grep
---

Run the merge gate and drive it to green.

$ARGUMENTS

1. `just pre-pr` — every job `ci.yml` runs, locally, in CI's order: `just ci`
   (fmt-check, lint, test, device-check, format-version-check, changelog-check,
   docs), then actionlint, `just audit`, the portable build and its linkage
   assertion, the CUDA path, and coverage.

   Run the whole thing, not `just ci` alone. `just ci` is one job of six, and
   the other five catch breakage it structurally cannot see — a CUDA path that
   stopped compiling, a licence a dependency changed underneath us, a workflow
   expression that parses and means nothing. Catching those here costs a
   minute; catching them on a pushed branch costs a round trip and a red PR.

2. If a recipe fails because its *tool* is missing rather than because the tree
   is wrong, say which one and run `just setup`. Do not skip the job silently —
   an unrun check reported as green is worse than a red one.

   `check-cuda` and `build-cuda` need a CUDA toolkit on this host. If there is
   none, say so plainly and leave those two to CI; do not report `pre-pr` as
   green when part of it did not run.

3. If anything fails, triage with the `merge-gate` skill and fix the cause, not
   the symptom. Never weaken a check to make it pass: a failing Zobrist
   fingerprint, or a search test that fails only at 8 threads, is the check
   working.

4. Re-run until green, then report the actual final output.

5. Read `.claude/skills/review-response/LESSONS.md` against this diff. It is a short conditional checklist — *if you changed documentation, find the other files that say the same thing; if you changed a validator, check it rejects the missing case* — and every line on it is there because a reviewer once caught it here. Cheaper to run your own eyes over it now than to learn it again from a review thread.

If I touched search or the transposition table, also run `just test-tt-off`.
