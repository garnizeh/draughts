---
description: Run every CI job locally (just pre-pr) and fix what it finds
argument-hint: "[optional: extra context or a recipe to run instead]"
allowed-tools: Skill, Bash, Read, Edit, Glob, Grep
---

Run the merge gate and drive it to green.

$ARGUMENTS

1. `just pre-pr` — every job `ci.yml` runs, locally. The order is not CI's: it
   is sorted by what a failure would *mean*, so anything answerable on any
   machine is answered before anything that can fail for a reason unrelated to
   the change.

   First `just ci` and the portable build with its linkage assertion — these need only the pinned toolchain, so a failure means the tree is wrong. Then `just audit`, coverage and actionlint, which need a tool `just setup` installs. Then the CUDA path, which needs a toolkit on this host.

   The `justfile` is the authority on that order; if this paragraph and the
   recipe disagree, the recipe is right.

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

   Some rules carry a `⚠ Looks like a violation but is not` note. Read it before hunting on that rule — it names what will look wrong and is not. If you hit a *new* trap of that kind, add it to the note; that is as useful as the rule itself and it is what keeps the checklist worth reading.

   **If a rule catches something, append a `#PR (date)` entry to its `saved` list and say in your report what it caught.** Together with `missed` — the times a rule was there and a reviewer found the mistake anyway — that gives each rule a hit rate, which is how the file learns which of its rules actually work rather than merely which mistakes are common. Record a save only when you can name the defect: it is the one counter with no review thread behind it, and the hit rate divides by both.

If I touched search or the transposition table, also run `just test-tt-off`.
