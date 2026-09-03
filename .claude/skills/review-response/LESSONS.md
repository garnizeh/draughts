# What reviews have taught this tree

Conditional checks, each one earned from a finding that actually happened here. Read this **before opening a PR**, and again when triaging a new review.

---

## ⚠ Failed before — read these twice

These rules were in this file, were not applied, and a reviewer caught the mistake anyway. They sit at the top because a rule that has already failed once is the likeliest to fail again, and because a miss is evidence about *this file's writing*, not about the world.

Each stays in its conditional section below as well — hoisting it out would hide it from anyone scanning by kind of change, which is the other way this file gets read. This block is the index, not the home.

| Rule | Section | Hit rate | The most recent miss |
|---|---|---|---|
| Find every other document that states the same thing | [documentation](#if-you-changed-documentation) | 4/9 | Two PRs off the same base each added their own miss to this rule without seeing the other's: #102 found this ledger's own index and detailed entry for this row had drifted from each other, and #103 (independently, on a branch cut before #102 merged) found `.claude/commands/respond.md` and a deferral loophole in `CLAUDE.md`. **Carries a false-positive note — read it before hunting** |
| State what a rule says about everything it will be read against | [rules and policies](#if-you-wrote-a-rule-a-policy-or-a-checklist) | 0/3 | This very entry's closing sentence summarized a three-part fix but illustrated only one part, leaving a reader free to fix one ambiguity and believe the sentence was satisfied |
| Do not require something the platform cannot do | [rules and policies](#if-you-wrote-a-rule-a-policy-or-a-checklist) | 0/1 | A pasteable sequence in `merge-gate` instructed `git branch -d`, one bullet above the paragraph explaining that squash-only merges make it refuse every time |

**A rule leaves this block after a full release with no new miss, and goes back to living in its section alone.** It does not leave because someone rewrote it and feels better about the wording; it leaves on evidence, like everything else here.

Demotion is not tidying. Attention is the same kind of scarce budget as `CLAUDE.md`'s context: **if everything is at the top, nothing is**, and a block that only ever grows is one people stop reading at exactly the moment it matters. The reader who opens this file before a PR can hold three or four highlighted rules in their head. That is the real capacity, and it is what demotion protects.

The consequence is that nothing lives here permanently, by construction. A rule either stops being missed and drops back down, or keeps being missed and graduates out — to a script at ×2, to `CLAUDE.md` at ×5. A rule that has sat here across several releases with the same miss count is not a stubborn rule; it is a rule nobody has demoted, and that is a bookkeeping failure rather than a finding.

---

## How this file works

Every item carries **an origin and two counters, and none of them is a number you type** — all are lists of dated evidence, so the count and the proof cannot drift apart.

- **origin** — the finding that created the rule: the rule did not exist yet, so nothing could have prevented it.

  Three kinds of entry, and only the first has one. An **earned** rule has exactly one origin and is the normal case. A **standing** rule restates something already in `CLAUDE.md` or `CONTRIBUTING.md`, and is here because it is what a diff should be read against — it has no origin because it was not learned here. A **preemptive** check was written because the risk was obvious, before anything went wrong, and is marked *no origin* rather than given a fictitious one.
- **missed** — the rule *existed*, was not applied, and a reviewer caught it in the PR anyway. **This is the one that says the text needs work.**
- **saved** — the rule was read before opening the PR and caught the mistake first. This is the rule paying for itself.

**Last useful** is the latest date in either counter. It is not written down, because a third number is a third thing to forget to update.

Separating `origin` from `missed` is the whole point of the shape. Lumping them together — as an earlier version of this file did — hides the only fact that criticises the rule itself: *it was there, and it did not fire.* An origin is the world teaching you something; a miss is your own writing failing to teach anyone else.

### The hit rate

`saved / (saved + missed)` is the rule's accuracy: of the times it was relevant, how often it actually fired. It is the number to look at before deciding whether a rule is pulling its weight, and it is why `missed` is worth the bookkeeping.

| missed | saved | Reading |
|---|---|---|
| high | zero | **The text is not working.** Nobody is reading it, or it is not phrased as something you could check in a diff. Rewrite it before anything else — bumping the counter again teaches nobody |
| high | high | A real, recurring class that the wording only half prevents. The best candidate for a script |
| zero | high | Working exactly as intended. Leave it alone |
| zero | zero, and stale | Either it has been internalised or it is about code nobody touches. A candidate for deletion — see below |

### Promotion

**Promotion keys on `missed`.** A miss means the rule was present and the mistake happened regardless, which is the only evidence that a checklist line is not enough. `origin` is not evidence of that — the rule did not exist — and `saved` is the opposite of it.

| missed | A script can decide it | Only judgment can decide it |
|---|---|---|
| ×1 | Rewrite the text first. A rule can fail once for being badly worded | Rewrite the text first |
| **×2** | **Write the check.** Wire it into `just ci`, move the line to *Retired* | Bump the count. Still a checklist line |
| **×5** | — (it should never get here) | **Promote to `CLAUDE.md`.** Move the line to *Retired* pointing at it |

The first miss buys a rewrite, not a promotion: a rule that failed once may simply have been written badly, and mechanizing a badly-understood property produces a badly-aimed check. The second miss is when to stop rewriting.

The two thresholds differ because the two destinations cost different things. A script costs a second of CI and nothing at all to remember, so there is no reason to let a mechanizable property be missed twice. A rule in `CLAUDE.md` is loaded into every session's context on every turn, forever — that budget is the scarcest thing the harness spends, and five is the bar for spending it. Five is arbitrary in its exact value and not arbitrary in its size: high enough that the rule has clearly earned permanent residence, low enough that it does not take a year.

**A rule that later becomes mechanizable leaves `CLAUDE.md` for a script.** Promotion is not one-way, and the context budget is worth reclaiming.

### Retirement, and the honest caveat

Read this file at each release, alongside the CHANGELOG section. Two passes, both cheap:

**Demote.** Any rule in the *Failed before* block with no new miss since the previous release comes out of it. See that block for why this is not optional.

**Delete.** A rule with **no activity in either counter since the previous release** is a candidate for removal: it has either been internalised or it is about code nobody is touching. Deleting is a judgment call and not automatic — a rule about a rare, expensive mistake is worth keeping cold — but it is the question that stops the file growing from the far end.

**`saved` is self-reported, and that is its weakness.** A `missed` entry has a review thread behind it that someone else wrote; a `saved` entry has only the word of whoever wrote it. So: **record a save only when you can name what it caught, as concretely as a miss names its finding.** A save with no named defect is not a save, it is a rule congratulating itself — and since the hit rate divides by the sum of the two, an inflated `saved` does not just flatter one rule, it corrupts the only number here that ranks them.

**Every rule carries a `scope:` selector**, as an HTML comment under it:

```markdown
<!-- scope: paths=["**/settings.json"] change_kinds=[permission_rule] -->
```

Nothing reads these yet, and at this size nothing needs to — the `## If you changed X` headings are a better index than any selector, because they are exact. They are written for two other reasons.

The first is that **writing one is a test of the rule.** A scope forces you to say when the rule applies, which is precisely what a vague rule avoids doing. Compare the permission rule, at two paths and one change kind, with *find every other document that states the same thing*, whose honest scope is `change_kinds=[documentation]` and therefore matches nearly every pull request — the same rule that needed a false-positive note and whose mechanization was rejected at seven false positives out of nine. **A broad scope and a false-positive problem are one fact seen from two sides**, and the selector shows it while you are writing rather than six months later.

The second is cheap forward compatibility. A rule whose scope is code that no longer exists can retire itself, which prose cannot do; and if this ledger ever outgrows one repository, a selector is the join key between a diff and the rules that apply to it. Twenty-one lines now against a retrofit later.

**A rule that is prone to false positives says so, inline, with the reason.** A `⚠ Looks like a violation but is not` note is part of the rule, not a footnote to it: someone reading this file against a diff is about to go looking, and the cheapest thing you can give them is the list of things they will find that do not matter. A rule that sends the reader chasing ghosts three times gets ignored the fourth, and it will be ignored on the occasion that counted.

**Record rejections too.** "Raised and deliberately not fixed, because X" stops the same argument being had twice.

**If this file is growing and nothing is graduating, the harvest phase is being skipped.** That is the signal to look for, not the length.

---

## If you changed documentation

- **Find every other document that states the same thing, and change it too.** This tree says the same things in `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `.claude/README.md` and the skills, on purpose — each for a different reader. A change to the gate, the release procedure or the five rules touches four or five files, and the ones you forget are the ones that quietly start lying. <sub>**origin** #99 (2026-09-02, `just pre-pr` and CHANGELOG rotation needed edits in all five files) · **missed ×5** #99 (2026-09-03, a review found `/gate`'s frontmatter still advertising `just ci` while its body said `pre-pr`, and `CLAUDE.md`'s job list omitting `portable-check`), #99 (2026-09-03, a later review found four more: `.claude/README.md` missing the outside-diff reply exception that `CLAUDE.md` and the PR template had already gained, `CONTRIBUTING.md`'s setup block still sending a newcomer to `just ci` and calling it "the full merge gate", and both `CHANGELOG.md` and `/gate` describing a `pre-pr` order the recipe never had), #102 (2026-09-03, a version bump to 1.5 changed every place that named "1.4" and missed three that encoded the version without naming it: `docs/architecture/README.md`'s Appendix F row still marked **Current** for the superseded v1.3→v1.4 checklist, `architecture-map/SKILL.md`'s §0 row still read a `1.1–1.4` range, and — the same drift one level down, inside a single file — §6.1's `GameState` class diagram and its Rust struct declaration both omitted the `non_progress_plies` field the rest of the PR had added to the real struct), #103 (2026-09-03, on a branch cut before #102 merged, so neither PR could see the other's drift: two more in one review — `.claude/commands/respond.md` restates the `review-response` procedure as numbered steps and still had reply before harvest and no push step at all, after the skill itself was reordered to push once; and the skill's own reply-phase text still read "answer those together in one PR comment... or wait for a later pass to promote them" — an escape hatch `CLAUDE.md` repeated verbatim, that let an outside-diff finding go unanswered indefinitely, which `CONTRIBUTING.md`'s stricter wording never permitted), #102 (2026-09-03, a second review pass on the same PR caught that this ledger's own *Failed before* index and this detailed entry no longer agreed: the index's third item read "a subsection list that gained an entry", which describes nothing here — the real third item is the `GameState` field missing from two declarations in one file. Fixed by rewriting the index cell to match the detail it summarizes) · **saved ×4** #99 (2026-09-02, the pre-implementation sweep read this rule against its own diff and found `README.md` still saying "three rules" where seventeen other places said five, and `ROADMAP.md`'s definition of done — which ninety-four filed issues link to — still saying `just ci`), #99 (2026-09-03, a `/gate` pass caught `CONTRIBUTING.md` still teaching the superseded two-counter model this file had already replaced — two documents giving different rules for the same file — and `merge-gate`'s **frontmatter** plus its body both listing a `just ci` that no longer existed), #99 (2026-09-03, a harness review found the sharper half of this rule had never been applied to the three documents that state the gate most loudly — `CLAUDE.md`, `CONTRIBUTING.md` and `merge-gate`'s own body all still claimed `pre-pr` runs "in CI's order", which the recipe has never done and whose own comment says the opposite — plus four further contradictions inside `merge-gate` alone: "outside the gate … they run nightly" listing three jobs that run on every PR, a triage row calling `workflows-check` a CI job with no recipe, a summary line still ending the change at `just ci`, and no triage row at all for four of `pre-pr`'s seven prerequisites), #99 (2026-09-03, a later review found `.claude/README.md`'s description of the `LESSONS.md` schema had not been updated when the schema itself changed from `caught` to `missed` and gained the earned/standing/preemptive distinction on `origin` — the file was internally inconsistent, using `missed` in its own closing sentence while calling the counter `caught` everywhere above it, and `/gate` and `/respond` were already on the current vocabulary), #101 (2026-09-03, working the review on a change to the post-merge procedure: `CONTRIBUTING.md` defines when a PR is done, in its *Responding to CodeRabbit* section, and would have gone on saying a PR is done at the last reply while `CLAUDE.md` and the `merge-gate` skill had both moved to "and the branch is gone". The reviewer did not catch it; the rule did — one phase later than it should have, at `/respond` rather than before the PR, which is worth recording as honestly as the catch itself). Hit rate 4/9 — recomputed from the lists, which now hold five misses and four saves after two branches cut from the same base each found their own drift and neither could see the other's, which is the rule's own subject matter proving itself one more time. **Mechanization was attempted at ×2 and rejected** — see [RETIRED.md](RETIRED.md), *Considered and not built*: a prototype comparing documented recipe lists to the `justfile` produced nine findings of which seven were false positives, because the same recipe names legitimately appear in other orders for other purposes. It stays a judgment rule. The miss bought a rewrite rather than a promotion, and this is the sentence it bought: **frontmatter and metadata are documentation too** — both halves of that miss were in metadata around a list, not in the list. The second save is that rewrite paying for itself: the reader went looking at frontmatter *because* the sentence told them to, and found `merge-gate`'s description advertising a gate that had changed twice since. That is the ×1-miss-buys-a-rewrite rule doing exactly what it claims. **The second miss adds the sharper half: a list whose order is decided elsewhere should not be restated in prose at all.** Four of the six documents named across those two miss entries are describing the order of something the `justfile` owns. Say what the recipe is *for* and let `just --list` say what it contains. The link-shaped half of this rule graduated to `check-doc-links.py`, and the seam-list half to `check-source-citations.py`; what is left is the half where the restated thing has no machine-readable source of truth, and no script decides that.</sub>

  > ⚠ **Looks like a violation but is not.** Recipe names appear all over the documentation in orders that are correct for their own reason, and a prototype check could not tell them apart — seven of its nine findings were false positives ([RETIRED.md](RETIRED.md)). Before calling one a defect, ask what the passage is enumerating: **the `ci.yml` job order is not the `pre-pr` prerequisite order**, a triage table is sorted for reading rather than for execution, and prose naming three recipes is usually making a point about those three. Only a passage claiming to enumerate a list the `justfile` owns is drifting.
  <!-- scope: paths=["**/*.md", ".claude/**"] change_kinds=[documentation] -->
- **If the change contradicts `docs/architecture/`, stop and say so first.** The document is version 1.5 and approved; the code is the unfinished part. A disagreement is a defect in the code unless you can show the document is wrong — and if you can, that is a conversation before it is a commit. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
  <!-- scope: paths=["src/**", "docs/architecture/**"] change_kinds=[documentation, source] -->
- **A constant with no § is the thing most likely to be "cleaned up" into a bug.** New numbers carry the section that decided them. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
  <!-- scope: paths=["src/**", "scripts/**", "justfile"] change_kinds=[source] -->

## If you changed CI, a workflow, or anything it pulls in

- **A container image tag is exactly as mutable as an action tag.** `uses:` is pinned to a commit here; `container:` and `image:` must be pinned to a digest for the same reason, and the digest to use is the multi-arch index so platform resolution still works. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99's pre-implementation sweep: `rust:1.98.0-slim-bookworm` was a floating tag in both `ci.yml` and `release.yml`, and it is the container that builds the binary a release ships.</sub>
  <!-- scope: paths=[".github/workflows/**"] change_kinds=[ci_workflow, dependency_pin] -->

## If you wrote a rule, a policy, or a checklist

- **State what it says about everything it will be read against, or it will be read against them anyway.** A rule that names one case leaves every other case to be argued about later, by someone who will reasonably read the omission as permission or as prohibition — whichever suits. **Beware the absolute you write while looking at the common case**: "exactly one, always" is the shape this fails in. <sub>**origin** #99 (2026-09-02, the no-hard-wrap rule named Rust source and said nothing about Python, so a reviewer correctly read PEP 8 comments in `scripts/*.py` as a violation of it — the rule was incomplete, not the code) · **missed ×3** #99 (2026-09-03, this file's own schema said every rule has "exactly one origin, always" while the same file carried two standing rules and a preemptive check with none; the three entry kinds are now named where the schema is defined), #102 (2026-09-03, §5.3.1 stated the repetition rule as "the same position occurs for the third time... counted since the last irreversible move" — a domain rule, not a repo policy, but the same failure shape: a reviewer read it three different ways at once, as board-only identity versus identity including side to move, as a window reset by any irreversible move versus specifically a capture or promotion, and as a draw on the third occurrence versus the fourth depending on whether the window's opening position counts. All three ambiguities were in a sentence that read as precise while it was being written), #102 (2026-09-03, a later pass on the same PR caught that this very entry's own closing sentence — "the fix both times was the same" — generalized the fix correctly but illustrated it with only one of the three ambiguities just listed above it, leaving a reader free to fix one and believe the sentence was satisfied) · **saved ×0**. Hit rate 0/3 — all three misses landed on the document doing the stating, which is the least surprising place for this rule to fail and the easiest to stop looking at. The second miss generalizes the first: it is not only a repo-policy failure mode, it is what happens whenever a normative sentence uses a natural-language quantifier ("the same", "since the last", "the third") over a domain that has more than one reasonable reading, and the fix both times was the same — replace the natural-language noun with the concrete thing it was standing in for. All three ambiguities the origin named took that same fix, not just one of them: "the same position" became a Zobrist key with side-to-move folded in, "since the last irreversible move" became specifically a capture or a promotion, and "the third time" became a count that includes the occurrence the window opened on.</sub>
  <!-- scope: paths=["**/*.md", ".claude/**", ".github/*TEMPLATE*"] change_kinds=[rule_or_policy] -->
- **Do not require something the platform cannot do — and that includes every command in a block someone will paste.** An impossible step is not a high standard, it is a step everyone learns to skip. **The miss taught the second half: a skipped step is not skipped into nothing, it is skipped into a substitute the reader picks unaided, and the substitute is chosen at the worst possible moment.** So a step that always fails is more dangerous than one that is merely useless, and *explaining* underneath the block why it always fails does not fix it — that was the exact shape of the miss. If the safe command cannot succeed here, the block gets the one that can, with the guard rebuilt as something that is true: not the proxy the tool happens to offer, the property the proxy was standing in for. <sub>**origin** #99 (2026-09-02, the PR template demanded a reply "on its own thread" for outside-diff findings, which have no thread and no comment id to reply on) · **missed ×1** #101 (2026-09-03, the new post-merge sequence instructed `git branch -d` one bullet above a paragraph explaining that squash-only merges make `-d` refuse every time in this repository; a reviewer caught it) · **saved ×0**. Hit rate 0/1. The miss bought the rewrite above rather than a promotion, and it is worth noticing *how* it failed: the author knew the command could not succeed, wrote that down, and still left it in the block — so the rule did not need to be believed, it needed to be applied to a code fence rather than to a checklist. The fix was `git diff --quiet main "$BRANCH" && git branch -D "$BRANCH"`, where the tree comparison is strictly stronger than the ancestry test `-d` performs; squash breaks the proxy, not the property.</sub>
  <!-- scope: paths=["**/*.md", ".claude/**", ".github/*TEMPLATE*"] change_kinds=[rule_or_policy, documentation] -->
  <!-- scope: paths=["**/*.md", ".claude/**", ".github/*TEMPLATE*"] change_kinds=[rule_or_policy] -->

## If you changed a permission rule, a token scope, or an allowlist

- **A prefix rule cannot express "read-only" for a program that dispatches.** `gh api <path>:*` matches `--method DELETE` on that path as happily as a GET; `Bash(just:*)` matches every recipe in the `justfile`, including the ones nobody has written yet. Allowlist the *operations* that can only read or build — `gh pr view`, `gh issue list`, named recipes — and never the runner. **And a rule naming one operation matches one exact string**, so a `deny` or an `ask` sitting under a broader allow is not the backstop it looks like. <sub>**origin** #99 (2026-09-03, `Bash(gh api repos/garnizeh/draughts/pulls:*)` was added meaning "read reviews" and also permitted deleting any comment on the repository; both rules were dropped) · **missed ×0** · **saved ×1** #99 (2026-09-03, a harness review found the same shape one layer down: `Bash(just:*)` pre-approved every recipe present and future, and the `Bash(just clean-data)` deny under it matched only that one string — so `just clean clean-data`, two real recipes in one invocation, removed the local database with no prompt. The blanket allow was dropped). Hit rate 1/1. The origin carried a second defect worth remembering: the allowlist matched `gh api repos/...` while every documented command was `gh api --paginate repos/...`, so it granted too much *and* covered nothing anyone actually ran.</sub>
  <!-- scope: paths=["**/settings.json", ".github/workflows/**"] change_kinds=[permission_rule] -->

## If you changed a validator, a parser, or a gate

- **Check that it rejects the *missing* case, not only the wrong one.** A validator that catches a thing in the wrong place will happily accept a file where the thing is absent entirely, and absence is usually the likelier mistake. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `changelog-check` rejected `[Unreleased]` below the first section but accepted a file with no `[Unreleased]` at all.</sub>
  <!-- scope: paths=["scripts/**", "src/config/**"] change_kinds=[validator] -->
- **If two places decide the same question, make them share the decision or prove they agree.** Two definitions of one predicate drift, and the drift shows up as one gate passing what the next one fails. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `release-notes` accepted an undated heading that `release-check` rejected, turning "not ready yet" into a red job on `main`.</sub>
  <!-- scope: paths=["scripts/**", "src/**", "justfile"] change_kinds=[validator, source] -->
- **Prove a new check fails on the defect it exists for.** Reconstruct the pre-fix state and watch it go red. A check that has never been seen to fail is decoration, and nobody will notice when it stops working. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, applied to all four new guards.</sub>
  <!-- scope: paths=["scripts/**", "justfile", ".github/workflows/**"] change_kinds=[validator, ci_workflow] -->

## If you added a configuration key

- **A configurable value that changes outcomes needs its own provenance wherever the data it produced gets replayed or exported — a startup warning is not enough.** `[rules.draw]`'s four values change which games end as draws and why; §23.1 warns once, at startup, when they depart from the defaults, but a warning is a line in a log and the dataset outlives it. The check is: when a lab batch or a game record is read back later — for replay, for training, for a bug report — can the reader recover the configuration that produced it, or only that it was probably `english_draughts`? <sub>**origin** #102 (2026-09-03, `Config.rules.draw` accepted four non-default values but neither `lab_batches.config_json` nor `GameRecord` recorded them, so a non-default batch could lose the values needed to reproduce its own draw adjudication; found by CodeRabbit, fixed by recording the resolved policy in `lab_batches.config_json` beside the transposition mode already stored there for the identical reason) · **missed ×0** · **saved ×0**.</sub>
  <!-- scope: paths=["src/config/**", "docs/architecture/23-configuration-example.md", "docs/architecture/12-database-schema.md"] change_kinds=[configuration, documentation] -->

## If you touched the transposition table, an evaluator, or `EvaluatorIdentity`

- **Anything pooled by position identity must make the sampled value a pure function of that identity — reset what is not, rather than let it ride along.** `TtMode::Throughput` merges `TtKind::Estimate` entries keyed on `(board, side_to_move, EvaluatorIdentity)` alone. A rollout that carries extra state from its starting position into its own trajectory — a non-progress counter, an inherited move history, anything computed from *how the leaf was reached* rather than what it *is* — will sample a distribution that depends on that state, and the table has no way to know the two entries it is averaging came from different distributions. The fix is almost always cheaper than the bug: reset the extra state to its canonical starting value before simulating, so the thing that varies (the policy) is exactly the thing already captured in `EvaluatorIdentity`, and the thing that must not vary (path-dependent state) does not reach the sampled value at all. <sub>**origin** #102 (2026-09-03, `RandomRolloutEvaluator::estimate_leaf_value` cloned the leaf's `GameState` — including `non_progress_plies` — into its playout, so the same position probed at counter 0 and at counter 79 would sample materially different games and still pool into one `Estimate` mean; found by CodeRabbit, fixed by zeroing the counter before the playout runs, argued in full in §6.3) · **missed ×0** · **saved ×0**.</sub>
  <!-- scope: paths=["src/engine/**", "docs/architecture/06-mcts-extensibility.md"] change_kinds=[source, documentation] -->

## If you changed a write path

- **Anything the tree calls immutable needs the write path to say so.** Documentation does not stop `write_text`. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the CHANGELOG rotation could overwrite an archived release's notes, which the archive's own README calls permanent.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **Check every target before writing any of them.** A collision discovered halfway through leaves the tree in a state neither the old nor the new one. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, same finding.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **A guard against overwriting must let a run resume itself.** Output identical to what this run would write is not a collision — it is this run, interrupted. Refusing it makes the retry impossible and the only way out a manual delete. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the collision guard added for one review made a half-finished rotation unrecoverable, and the next review caught it.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **An interrupted multi-step write path can resume on a different branch than the one it interrupted in, and that branch must still finish what was skipped.** A fast-return path written for "nothing to do" can also be the state a real interruption leaves behind; if it exits without checking for an incomplete later step, the write path is only resumable from the exact step that failed, never from a later run that happens to see quiescent input. <sub>**origin** #99 (2026-09-03) · **missed ×0** · **saved ×0** — #99: `rotate-changelog.py`'s `over <= 0` return, meant for "nothing to archive", was also the state reached after a prior run rewrote `CHANGELOG.md` but crashed before `write_index()`; that branch never checked for a stale index, so it stayed broken forever.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->

## If you wrote a table, a list, or anything with a syntax

- **A linter exists for the format you are writing, and the reviewer has it.** A footnote marker after a table row's final pipe makes a fifth cell; nothing about the source line looks wrong, and the rendered table is where it shows. <sub>**origin** #99 (2026-09-03, `| … | \`type:infra\` |¹` in `docs/ROADMAP.md` tripped markdownlint MD055/MD056 in review) · **missed ×0** · **saved ×0**. Candidate mechanization: markdownlint in `just ci`. Not built at ×1, deliberately — the first occurrence buys a rule, and a tool added on one finding is a tool nobody chose.</sub>
  <!-- scope: paths=["**/*.md", "**/*.yml", "**/*.toml"] change_kinds=[markup] -->

## If you drew a Mermaid flowchart beside a normative rule

- **A diagram is a second statement of the rule, and every branch the prose requires has to be a branch on the diagram — not just the one that fits on a single arrow.** Prose says "if X then A, otherwise B"; a diagram node that only draws B looks complete because it renders cleanly, and a reader following the picture alone gets the common case and misses the exception. This is `[documentation](#if-you-changed-documentation)`'s "find every other document" rule applied to a format that isn't text: the diagram drifted from the prose exactly the way two Markdown files drift from each other. <sub>**origin** #102 (2026-09-03, one flowchart node in §8.2 — "append the move... and its key to the repetition window" — went through two review rounds on the same PR before this shape was named: round 1 caught that the node never said the window is *seeded* when it opens (fixed, but not filed as its own class at the time); round 2, on the very next push, caught that the fixed wording still never said a capture or promotion discards prior keys and seeds the window with the post-move Zobrist key; otherwise append the post-move key — a behavior §5.3.1's prose had stated correctly the whole time. Two omissions on one node, two review rounds, before either got written down as a rule rather than fixed as an instance) · **missed ×0** · **saved ×0**.</sub>
  <!-- scope: paths=["docs/architecture/**/*.md"] change_kinds=[documentation, diagram] -->

## If you changed a `justfile` recipe with prerequisites

- **`just` stops the chain at the first failure, so order by what can fail for reasons other than "the tree is wrong".** Sort into tiers — recipes needing only the pinned toolchain, then recipes needing a tool `just setup` installs, then recipes needing particular hardware — because a failure in the later tiers may mean the host rather than the change. <sub>**origin** #99 (2026-09-02, `check-cuda` sat ahead of `coverage`, so a host with no CUDA toolkit lost its coverage report to something unrelated to it) · **missed ×0** · **saved ×1** #99 (2026-09-03, a `/gate` pass found the origin fix had been applied to the CUDA recipes *only*: `workflows-check` needs actionlint and sat second in the chain, so a host without it also lost `audit`, `portable-check`, `coverage` and both CUDA recipes — and `audit` and `coverage` have the same tool dependency. `pre-pr` is now sorted into the three tiers above). Hit rate 1/1. **Fixing the instance a rule was born from is not fixing the class** — that is what this save actually caught.</sub>

  > ⚠ **Rejected, not applied.** A later review proposed `ci workflows-check audit portable-check coverage check-cuda build-cuda` — putting `workflows-check` (tier 2, needs `actionlint`) ahead of `portable-check` (tier 1, needs only the toolchain), citing a stored "Learning" that predates the fix above. That is the exact bug the `saved` entry recorded, reintroduced. Not applied — the live order is the correct one — #99 (2026-09-03).
  <!-- scope: paths=["justfile"] change_kinds=[justfile_recipe] -->
- **`just --list` shows the *last* comment line before a recipe.** A multi-line prose comment leaves a sentence fragment as the recipe's description. Put a blank line, then a one-line summary. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×1** #99 (2026-09-03, a harness review ran `just --list` and found two live fragments the origin sweep had left behind: `lint` described as "feature-gated path is linted separately by `check-cuda` (§20.10)." and `release-notes` as "in a regex match anything."). Hit rate 1/1 — learned building #99, when the tree already had two recipes described by half a sentence, and the two it still had were different ones.</sub>
  <!-- scope: paths=["justfile"] change_kinds=[justfile_recipe] -->

## If you are acting on someone else's suggested patch

- **Read the diff before applying it.** A correct diagnosis attached to a wrong fix was two of PR #99's four findings — not an edge case. Take the diagnosis seriously and the patch sceptically. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99.</sub>
  <!-- scope: change_kinds=[applying_patch] -->
- **Watch for a value interpolated into a pattern.** Version numbers are full of dots, and a dot in a regex matches anything: `0.2.0` also matches `0X2X0`. Match literally and pattern only the part that is genuinely variable. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, in a suggested diff that was not applied as written.</sub>
  <!-- scope: paths=["scripts/**", "src/**", "justfile"] change_kinds=[applying_patch, source] -->
- **An exit code must mean one thing.** A status that means both "your change is broken" and "this machine lacks a toolkit" cannot be acted on, and buries the first inside the second. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: a suggestion to continue past a failed `check-cuda` and return "incomplete", rejected for this reason.</sub>
  <!-- scope: paths=["scripts/**", "justfile"] change_kinds=[applying_patch, justfile_recipe] -->

---

## Where rules go when they leave

Rules move to [RETIRED.md](RETIRED.md), which keeps them apart by *why* they left, because the two reasons mean opposite things. **Graduated** — it worked well enough to become a script or a line in `CLAUDE.md`. **Dropped** — it earned nothing and stopped being worth the attention. Each entry records when, why, where it went, and the evidence it earned on.

That file also holds **mechanizations that were attempted and rejected**, with the prototype's result. A rule that looks script-decidable and is not will look script-decidable again to the next reader, and finding that out costs an afternoon each time.

Nothing is deleted. This file stays short because it holds only what you still have to remember, not because history is thrown away.
