---
description: Work the CodeRabbit review on a PR end to end — verify, fix, reply, harvest
argument-hint: "[PR number, default: the PR for the current branch]"
allowed-tools: Skill, Bash, Read, Edit, Write, Glob, Grep
---

Work the review on PR $ARGUMENTS to completion. If that is empty, resolve the
PR for the current branch first — `gh pr view --json number --jq .number` — and
say which PR you are working on. `argument-hint` documents a default; it does not
supply one. Load the `review-response` skill first — it owns this procedure, including the endpoint that silently omits half a review.

0. Read `.claude/skills/review-response/LESSONS.md` before triaging anything. It is a conditional checklist earned from past findings — it tells you what to look for in this diff, and whether a finding you would dismiss as a one-off has been seen before.
1. Fetch **every** finding: the per-line comments *and* each review's full `body`, where the "Outside diff range comments" live. `--paginate` on both calls.
2. Verify each against the code as it stands now. The branch may have moved, and the described consequence is the part reviewers get wrong most often. Report which findings are stale, which are right, which have a right diagnosis and a wrong fix, and which are not defects.
3. Fix what is real. Never apply a suggested diff without reading it.
4. **Harvest.** For each accepted finding, decide whether it named a class of mistake or a single instance. A class earns a permanent check; an instance is just a bug. A check must fail on the original defect before it lands, and adding one to `just ci` means the `merge-gate` triage table gains a row in the same change.
5. **Write the rule down.** Add what the review taught to `LESSONS.md` as an instruction — *if you changed X, check Y* — not as a story about what went wrong. A finding on a class not yet in the file creates a rule, and is its **origin**. A finding on a class already there is a **miss** — the rule existed and did not fire — and that is the entry that matters, because it is the only one that criticises the writing rather than the world. Append `#PR (date)` to the right list; never edit a digit. Give it a `scope:` selector — paths and change kinds. Nothing reads them yet; a scope you can only write as "most of the tree" is telling you the rule is too broad to be useful. If the finding turned out to have look-alikes that were *not* defects, put them in a `⚠ Looks like a violation but is not` note on the rule — the negative knowledge is worth as much as the rule and belongs beside it. Record rejections the same way.

   Then apply the thresholds, which key on `missed`: the **first** miss buys a rewrite of the sentence, not a promotion — say what you changed about it. At ×2 anything a script can decide becomes a check in `just ci`; at ×5 a judgment rule earns a line in `CLAUDE.md`. Either way the line moves to *Retired* with a pointer to what replaced it.
6. **Push once.** The fix commits and the `LESSONS.md` update go up together, in one push — commit them separately if that reads better, but do not push until both are ready. A second push re-triggers a CodeRabbit review that may have no rate-limit allowance left to run, and that failure is silent.
7. Reply on each thread, citing the commit that landed, saying what changed and where — or why it stands as is. A finding with no comment id (the outside-diff kind) gets answered now, in one PR comment naming each file and line — never deferred to wait for a later review that might promote it to its own thread.
8. `just pre-pr`, then report the real output.

Tell me explicitly what you harvested, what you deliberately did not, and what the ledger now knows that it did not before. A review that changes nothing about how the next mistake is caught was a review half-read.
