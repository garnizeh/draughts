---
description: Answer a question from the authoritative architecture document
argument-hint: "<question — e.g. how does eviction work, why two model profiles>"
allowed-tools: Skill, Read, Grep, Glob
---

Answer from `docs/architecture/` (v1.5, authoritative), not from the code and
not from memory: $ARGUMENTS

Use the `architecture-map` skill to find the section, read it in full, then
answer with the § citation. If the code disagrees with the document, say so
explicitly and name both — that is a defect report, not a footnote.

If the document does not settle it, say that plainly rather than reasoning your
way to a plausible answer.
