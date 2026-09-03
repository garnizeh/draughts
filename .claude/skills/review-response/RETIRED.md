# Retired lessons

Rules that have left [LESSONS.md](LESSONS.md). They live here rather than at the bottom of that file because it is read *before every PR* and the active checklist is the part that has to stay short — provenance is worth keeping and worth keeping out of the way.

Nothing here is a rule you still have to remember. Each entry exists so that anyone who finds a check in `just ci`, or a sentence in `CLAUDE.md`, or an idea that was tried and dropped, can find out why.

**A rule leaves for one of two very different reasons, and they are kept apart** because they mean opposite things about the rule. *Graduated* means it worked so well it became something better than a checklist line. *Dropped* means it earned nothing and stopped being worth the attention. Filing one as the other would flatter the file and teach the next reader wrong.

Every entry records **when**, **why**, **where it went**, and **the evidence it earned on**. The last one is the point: a claim with no evidence behind it is what this whole file exists to avoid.

---

## Graduated — it became something better than a checklist line

### A renamed heading breaks every reference to it, silently

- **Retired** 2026-09-02
- **Why** — preemptive, not earned. The risk was obvious and the property cheap to assert, so the check was written before anything went wrong.
- **Where it went** — `scripts/check-doc-links.py`, run by `just doc-links` in `just ci`. Every relative link and every `#anchor` across the documentation resolves, or the gate is red.
- **Evidence** — none. **origin: none · missed ×0 · saved ×0.** Recorded plainly: this check has never caught a broken link, because there were none to catch — the sweep that produced it found sixty-two files fully consistent. Pretending otherwise would put invented evidence in the one file whose whole value is that its evidence is real.
- **Note** — this is the link-shaped half of *find every other document that states the same thing*. The half that needs judgment stayed in LESSONS.md, because no script can tell that `README.md` saying "three rules" and `CLAUDE.md` saying "five" is a contradiction rather than two true sentences.

### Never cite a line number in a document

- **Retired** 2026-09-03
- **Why** — mechanized. Promotion normally keys on `missed` and this rule never had one, so this is the case the thresholds explicitly call a floor rather than a ceiling: the property is exactly decidable by a script, and the risk was demonstrated rather than hypothetical.
- **Where it went** — `scripts/check-source-citations.py`, run by `just source-citations` in `just ci`. It rejects a source line number in any Markdown file, and separately asserts that both seam lists name every `todo!()` in `src/`.
- **Evidence** — **origin** #99 (2026-09-02) · **missed ×0** · **saved ×0**. Five of the eleven seam citations in `docs/ROADMAP.md` were pointing at the wrong line before a single seam had been touched. The same eleven then survived in `.claude/skills/implement-seam/SKILL.md` after the roadmap was fixed, with the same five still wrong — which is why the rule was worth mechanizing rather than restating: it had already been applied to one of the two documents it governed and stopped there. The check was proved against that state: twenty-four problems on it, none on the tree that replaced it.
- **Note** — the `#L42` link form is deliberately left alone. `check-doc-links.py` already decides what to do with those and permits them, and two scripts answering one question differently is how a gate starts contradicting itself.

---

## Dropped — it earned nothing, or it was superseded

Nothing yet.

An entry here should say what was tried and why it stopped being worth the attention, in enough detail that nobody tries the same thing again by accident. "Cold in both counters since two releases ago" is a reason. So is "superseded by a rule that says it better".

---

## Considered and not built

Not retirements — mechanizations that were attempted and rejected. They belong here for the same reason: so the next reader does not spend the afternoon finding out again.

### A script comparing the recipe lists in the documentation to the `justfile`

- **Rejected** 2026-09-03, while harvesting #99's third review.
- **Why it was attempted** — *find every other document that states the same thing* had reached **missed ×2**, and its ordering half looked script-decidable: extract `ci:` and `pre-pr:` prerequisite lists from the `justfile`, then assert that any paragraph enumerating three or more of them lists them in the same relative order.
- **Why it was rejected** — a prototype produced **nine findings, of which seven were false positives**. The names in those lists are also the names of the `ci.yml` jobs, of the rows in a triage table, and of recipes mentioned in ordinary prose — and no heuristic available here distinguishes "enumerating `pre-pr`'s prerequisites" from "enumerating the CI jobs in the order CI runs them". A check people learn to ignore is worse than no check, and this one would have had to be weakened to pass, which is the fourth bar a new check has to clear.
- **What to do instead** — the rule stays a judgment line in LESSONS.md. If this is ever mechanized, the way through is probably to stop restating the list in prose at all and generate it from the `justfile` into a marked block — a different change, to the documents rather than to the gate.
- **What did work, later, on a different list** — the seam list restated in `docs/ROADMAP.md` and the `implement-seam` skill *was* mechanized, by `check-source-citations.py`, with no false positives. The difference is worth naming because it predicts the next attempt: a seam has a machine-readable source of truth in `src/` and a canonical string that the compiler keeps honest, so the check compares two documents against a fact. The recipe order has no such fact — it is a list that legitimately appears in several orders for several purposes, and a check on it can only compare prose to prose. **Restated lists are mechanizable exactly when the thing being restated has a source of truth a script can read.**
