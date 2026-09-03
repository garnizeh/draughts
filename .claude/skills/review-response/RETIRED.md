# Retired lessons

Rules that have left [LESSONS.md](LESSONS.md). They live here rather than at the bottom of that file because it is read *before every PR* and the active checklist is the part that has to stay short — provenance is worth keeping and worth keeping out of the way.

Nothing here is a rule you still have to remember. Each entry exists so that anyone who finds a check in `just ci`, or a sentence in `CLAUDE.md`, or an idea that was tried and dropped, can find out why.

**A rule leaves for one of two very different reasons, and they are kept apart** because they mean opposite things about the rule. *Graduated* means it worked so well it became something better than a checklist line. *Dropped* means it earned nothing and stopped being worth the attention. Filing one as the other would flatter the file and teach the next reader wrong.

Every entry records **when**, **why**, **where it went**, and **the evidence it earned on**. The last one is the point: a claim with no evidence behind it is what this whole file exists to avoid.

---

## Graduated — it became something better than a checklist line

### Find every other document that states the same thing, and change it too

- **Retired** 2026-09-03
- **Why** — reached `missed ×5`, the threshold for a rule only judgment can decide. Two link-shaped and seam-shaped halves of this rule had already graduated to `check-doc-links.py` and `check-source-citations.py` (below); what remained was the half no script can decide — that two sentences are the same fact — and it kept failing anyway.
- **Where it went** — `CLAUDE.md`, in the *Working in this tree* list, right after the `just doc-links` bullet it completes. Restated in full there rather than pointed to, because a rule loaded into every session's context has to be read without a detour.
- **Evidence** — **origin** #99 (2026-09-02, `just pre-pr` and CHANGELOG rotation needed edits in all five files) · **missed ×5**: three rounds of #99 (2026-09-03, `/gate`'s frontmatter advertising `just ci` while its body said `pre-pr`; then four more the same day — `.claude/README.md`, `CONTRIBUTING.md`, `CHANGELOG.md` and `/gate` each stating the gate or the `pre-pr` order differently), #102 (2026-09-03, a version bump to 1.5 left an Appendix marked **Current**, a table still reading `1.1–1.4`, and a `GameState` field missing from two declarations in one file), #103 (2026-09-03, on a branch cut before #102 merged, so neither could see the other's drift: `.claude/commands/respond.md` restated a procedure `review-response/SKILL.md` had already reordered, and a deferral loophole survived in `CLAUDE.md` after `CONTRIBUTING.md`'s stricter wording had already closed it), #102 again (2026-09-03, this ledger's own *Failed before* index and its own detailed entry for this very rule had drifted from each other — the rule failing against its own two representations of one finding, inside the file that defines it) · **saved ×4**: #99 (2026-09-02, a pre-implementation sweep caught `README.md` still saying "three rules" against seventeen places saying five), #99 (2026-09-03, twice more, catching a superseded counter model and a stale `merge-gate` frontmatter before either shipped), #101 (2026-09-03, a `/respond` pass caught `CONTRIBUTING.md` alone still defining "done" as the last reply after `CLAUDE.md` and `merge-gate` had both moved to "and the branch is gone"). Final hit rate 4/9.
- **Note** — the last miss is the one worth remembering the shape of: two pull requests, branched from the same commit, each independently found and fixed a drift this rule was supposed to prevent, and neither branch's fix could see the other's, because the other did not exist yet from where it was standing. Resolving the two branches' conflicting rebase is what surfaced the combined count and crossed the threshold — a coincidence with the rule's own subject matter that is worth noting and not worth reading too much into. The link-shaped half of this rule graduated to `check-doc-links.py` and the seam-list half to `check-source-citations.py` (both below); this is the third and last piece, and none remains active in `LESSONS.md`.

  > ⚠ **Looks like a violation but is not.** Recipe names appear all over the documentation in orders that are correct for their own reason, and a prototype check could not tell them apart — seven of its nine findings were false positives (below, *Considered and not built*). Before calling one a defect, ask what the passage is enumerating: **the `ci.yml` job order is not the `pre-pr` prerequisite order**, a triage table is sorted for reading rather than for execution, and prose naming three recipes is usually making a point about those three. Only a passage claiming to enumerate a list the `justfile` owns is drifting.

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

### A write into `docs/changelog/` refuses a symlinked directory and a symlinked leaf

- **Retired** 2026-09-03
- **Why** — reached `missed ×2`, the threshold for a script-decidable property. Both misses were the same rule — *guard the whole path, not the leaf* — failing on a *newly added* write path rather than on a new kind of mistake, which is exactly what "the text needs work, or the property needs a machine to hold it" looks like the second time.
- **Where it went** — `scripts/check-changelog-rotate-symlinks.sh`, run by `just changelog-rotate-symlinks-check` in `just ci`. It reconstructs a symlinked `docs/changelog/` and a symlinked `docs/changelog/README.md` against both of `rotate-changelog.py`'s write paths — the ordinary rotation and the `over <= 0` recovery branch — and asserts every one is refused. Proved red against the pre-fix script before being wired into the gate.
- **Evidence** — **origin** #99 (2026-09-02, `## [../../CLAUDE]` reached `ARCHIVE / f"{version}.md"` as a path, and `exists()` follows symlinks so a dangling one reads as absent and gets written through) · **missed ×2**: #99 (2026-09-03, the first fix guarded `page.is_symlink()` only — if `docs/changelog/` were itself a link, a fresh `<version>.md` inside it is not a symlink and `write_text` lands outside the repository, so a directory-containment check was added), #99 (2026-09-03, a *third* review found the containment check had not been carried into the recovery branch added for the second review's finding, and that `write_index()` itself had never had a leaf guard on the `README.md` it writes — both write paths into `ARCHIVE` needed the same two guards and only one had them) · **saved ×0**. Final hit rate 0/2.
- **Note** — the rule that graduated is the general one, *guard the whole path, not the leaf, on every write path into it*. The instance-specific rules next to it in the write-path section — check every target before writing any of them, let a resumed run see its own output as success rather than a collision, and (the multi-step one, still active) let a different retry branch finish an earlier branch's incomplete step — stay judgment lines: each is about a shape of write-ordering mistake a script cannot recognize without knowing what the write means, where a symlink is a fact the filesystem states outright.

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
