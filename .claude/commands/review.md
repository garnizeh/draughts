---
description: Review the current change against the architecture and the five rules
argument-hint: "[optional: paths or a base ref]"
---

Review this change against `docs/architecture/` v1.5 and the five rules in
CLAUDE.md: $ARGUMENTS

Use the `architecture-reviewer` subagent. Give it the diff scope (default: the
working tree against `main`) and relay its Blockers / Concerns / Notes to me
with the § citations intact.

A review is not a substitute for running things. Before you call this change
ready for a PR, `just pre-pr` must be green — every job `ci.yml` runs, run here,
including the five that `just ci` alone does not cover. Report its real output
alongside the review; if part of it could not run on this host (the CUDA
recipes need a toolkit), say which part and do not call the rest "green".
