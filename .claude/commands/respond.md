---
description: Work the CodeRabbit review on a PR end to end — verify, fix, reply, harvest
argument-hint: "[PR number, default: the PR for the current branch]"
allowed-tools: Bash, Read, Edit, Write, Glob, Grep
---

Work the review on PR $ARGUMENTS to completion. If that is empty, resolve the
PR for the current branch first — `gh pr view --json number --jq .number` — and
say which PR you are working on. `argument-hint` documents a default; it does not
supply one. Load the `review-response` skill first — it owns this procedure, including the endpoint that silently omits half a review.

0. Read `.claude/skills/review-response/LESSONS.md` before triaging anything. It is a conditional checklist earned from past findings — it tells you what to look for in this diff, and whether a finding you would dismiss as a one-off has been seen before.
1. Fetch **every** finding: the per-line comments *and* each review's full `body`, where the "Outside diff range comments" live. `--paginate` on both calls.
2. Verify each against the code as it stands now. The branch may have moved, and the described consequence is the part reviewers get wrong most often. Report which findings are stale, which are right, which have a right diagnosis and a wrong fix, and which are not defects.
3. Fix what is real. Never apply a suggested diff without reading it.
4. Reply on each thread, saying what changed and where — or why it stands as is.
5. **Harvest.** For each accepted finding, decide whether it named a class of mistake or a single instance. A class earns a permanent check; an instance is just a bug. A check must fail on the original defect before it lands, and adding one to `just ci` means the `merge-gate` triage table gains a row in the same change.
6. **Write the rule down.** Add what the review taught to `LESSONS.md` as an instruction — *if you changed X, check Y* — not as a story about what went wrong. Cite the PR by **appending it to the existing citation** if a rule already covers this — the PR list is the counter. Record rejections the same way. Then apply the thresholds in that file's header: ×2 mechanizes anything a script can decide, ×5 promotes a judgment rule into `CLAUDE.md`. Either way the line moves to *Retired* with a pointer to what replaced it.
7. `just pre-pr`, then report the real output.

Tell me explicitly what you harvested, what you deliberately did not, and what the ledger now knows that it did not before. A review that changes nothing about how the next mistake is caught was a review half-read.
