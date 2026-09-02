---
description: Run the full merge gate (just ci) and fix what it finds
argument-hint: "[optional: extra context or a recipe to run instead]"
allowed-tools: Bash, Read, Edit, Glob, Grep
---

Run the merge gate and drive it to green.

$ARGUMENTS

1. `just ci` — fmt-check, lint, test, device-check, format-version-check, docs.
2. `just check-cuda` and `just audit` — unconditionally. CI runs both on every
   PR (the `cuda-compile` and `supply-chain` jobs in `ci.yml`), not only on
   feature-gated changes: a shared-code change can break the CUDA build or a
   dependency policy without touching a CUDA-specific file.
3. If any of it fails, triage with the `merge-gate` skill and fix the cause,
   not the symptom. Never weaken a check to make it pass; a failing Zobrist
   fingerprint or a search test that fails only at 8 threads is the check
   working.
4. Re-run until green, then report the actual final output.

If I touched search or the transposition table, also run `just test-tt-off`.
