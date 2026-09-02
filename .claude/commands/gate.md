---
description: Run the full merge gate (just ci) and fix what it finds
argument-hint: "[optional: extra context or a recipe to run instead]"
allowed-tools: Bash, Read, Edit, Glob, Grep
---

Run the merge gate and drive it to green.

$ARGUMENTS

1. `just ci` — fmt-check, lint, test, device-check, format-version-check, docs.
2. If it fails, triage with the `merge-gate` skill and fix the cause, not the
   symptom. Never weaken a check to make it pass; a failing Zobrist fingerprint
   or a search test that fails only at 8 threads is the check working.
3. Re-run until green, then report the actual final output.

If I touched search or the transposition table, also run `just test-tt-off`.
If I touched anything feature-gated, also run `just check-cuda`.
