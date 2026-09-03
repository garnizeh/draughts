# What reviews have taught this tree

Conditional checks, each one earned from a finding that actually happened here. Read this **before opening a PR**, and again when triaging a new review.

---

## ⚠ Failed before — read these twice

These rules were in this file, were not applied, and a reviewer caught the mistake anyway. They sit at the top because a rule that has already failed once is the likeliest to fail again, and because a miss is evidence about *this file's writing*, not about the world.

Each stays in its conditional section below as well — hoisting it out would hide it from anyone scanning by kind of change, which is the other way this file gets read. This block is the index, not the home.

| Rule | Section | Hit rate | The most recent miss |
|---|---|---|---|
| Find every other document that states the same thing | [documentation](#if-you-changed-documentation) | 3/5 | Four documents describing a `pre-pr` order the recipe never had, a setup block still sending newcomers to `just ci`, and an exception present in two policy files and missing from a third. **Carries a false-positive note — read it before hunting** |
| State what a rule says about everything it will be read against | [rules and policies](#if-you-wrote-a-rule-a-policy-or-a-checklist) | 0/1 | This file's own schema said every rule has "exactly one origin, always", while the same file carried three entry kinds |
| A string that becomes a filename is a path | [write paths](#if-you-changed-a-write-path) | 0/1 | The guard was placed on the leaf file; a symlinked *parent* directory is the same escape one level up |

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

- **Find every other document that states the same thing, and change it too.** This tree says the same things in `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `.claude/README.md` and the skills, on purpose — each for a different reader. A change to the gate, the release procedure or the five rules touches four or five files, and the ones you forget are the ones that quietly start lying. <sub>**origin** #99 (2026-09-02, `just pre-pr` and CHANGELOG rotation needed edits in all five files) · **missed ×2** #99 (2026-09-03, a review found `/gate`'s frontmatter still advertising `just ci` while its body said `pre-pr`, and `CLAUDE.md`'s job list omitting `portable-check`), #99 (2026-09-03, a later review found four more: `.claude/README.md` missing the outside-diff reply exception that `CLAUDE.md` and the PR template had already gained, `CONTRIBUTING.md`'s setup block still sending a newcomer to `just ci` and calling it "the full merge gate", and both `CHANGELOG.md` and `/gate` describing a `pre-pr` order the recipe never had) · **saved ×3** #99 (2026-09-02, the pre-implementation sweep read this rule against its own diff and found `README.md` still saying "three rules" where seventeen other places said five, and `ROADMAP.md`'s definition of done — which ninety-four filed issues link to — still saying `just ci`), #99 (2026-09-03, a `/gate` pass caught `CONTRIBUTING.md` still teaching the superseded two-counter model this file had already replaced — two documents giving different rules for the same file — and `merge-gate`'s **frontmatter** plus its body both listing a `just ci` that no longer existed), #99 (2026-09-03, a harness review found the sharper half of this rule had never been applied to the three documents that state the gate most loudly — `CLAUDE.md`, `CONTRIBUTING.md` and `merge-gate`'s own body all still claimed `pre-pr` runs "in CI's order", which the recipe has never done and whose own comment says the opposite — plus four further contradictions inside `merge-gate` alone: "outside the gate … they run nightly" listing three jobs that run on every PR, a triage row calling `workflows-check` a CI job with no recipe, a summary line still ending the change at `just ci`, and no triage row at all for four of `pre-pr`'s seven prerequisites). Hit rate 3/5. **Mechanization was attempted at ×2 and rejected** — see [RETIRED.md](RETIRED.md), *Considered and not built*: a prototype comparing documented recipe lists to the `justfile` produced nine findings of which seven were false positives, because the same recipe names legitimately appear in other orders for other purposes. It stays a judgment rule. The miss bought a rewrite rather than a promotion, and this is the sentence it bought: **frontmatter and metadata are documentation too** — both halves of that miss were in metadata around a list, not in the list. The second save is that rewrite paying for itself: the reader went looking at frontmatter *because* the sentence told them to, and found `merge-gate`'s description advertising a gate that had changed twice since. That is the ×1-miss-buys-a-rewrite rule doing exactly what it claims. **The second miss adds the sharper half: a list whose order is decided elsewhere should not be restated in prose at all.** Four of the six documents named across those two miss entries are describing the order of something the `justfile` owns. Say what the recipe is *for* and let `just --list` say what it contains. The link-shaped half of this rule graduated to `check-doc-links.py`, and the seam-list half to `check-source-citations.py`; what is left is the half where the restated thing has no machine-readable source of truth, and no script decides that.</sub>

  > ⚠ **Looks like a violation but is not.** Recipe names appear all over the documentation in orders that are correct for their own reason, and a prototype check could not tell them apart — seven of its nine findings were false positives ([RETIRED.md](RETIRED.md)). Before calling one a defect, ask what the passage is enumerating: **the `ci.yml` job order is not the `pre-pr` prerequisite order**, a triage table is sorted for reading rather than for execution, and prose naming three recipes is usually making a point about those three. Only a passage claiming to enumerate a list the `justfile` owns is drifting.
  <!-- scope: paths=["**/*.md", ".claude/**"] change_kinds=[documentation] -->
- **If the change contradicts `docs/architecture/`, stop and say so first.** The document is version 1.4 and approved; the code is the unfinished part. A disagreement is a defect in the code unless you can show the document is wrong — and if you can, that is a conversation before it is a commit. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
  <!-- scope: paths=["src/**", "docs/architecture/**"] change_kinds=[documentation, source] -->
- **A constant with no § is the thing most likely to be "cleaned up" into a bug.** New numbers carry the section that decided them. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
  <!-- scope: paths=["src/**", "scripts/**", "justfile"] change_kinds=[source] -->

## If you changed CI, a workflow, or anything it pulls in

- **A container image tag is exactly as mutable as an action tag.** `uses:` is pinned to a commit here; `container:` and `image:` must be pinned to a digest for the same reason, and the digest to use is the multi-arch index so platform resolution still works. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99's pre-implementation sweep: `rust:1.98.0-slim-bookworm` was a floating tag in both `ci.yml` and `release.yml`, and it is the container that builds the binary a release ships.</sub>
  <!-- scope: paths=[".github/workflows/**"] change_kinds=[ci_workflow, dependency_pin] -->

## If you wrote a rule, a policy, or a checklist

- **State what it says about everything it will be read against, or it will be read against them anyway.** A rule that names one case leaves every other case to be argued about later, by someone who will reasonably read the omission as permission or as prohibition — whichever suits. **Beware the absolute you write while looking at the common case**: "exactly one, always" is the shape this fails in. <sub>**origin** #99 (2026-09-02, the no-hard-wrap rule named Rust source and said nothing about Python, so a reviewer correctly read PEP 8 comments in `scripts/*.py` as a violation of it — the rule was incomplete, not the code) · **missed ×1** #99 (2026-09-03, this file's own schema said every rule has "exactly one origin, always" while the same file carried two standing rules and a preemptive check with none; the three entry kinds are now named where the schema is defined) · **saved ×0**. Hit rate 0/1 — and the miss landed on the document that states the rule, which is the least surprising place for it and the easiest to stop looking at.</sub>
  <!-- scope: paths=["**/*.md", ".claude/**", ".github/*TEMPLATE*"] change_kinds=[rule_or_policy] -->
- **Do not require something the platform cannot do.** A checklist item that is impossible is not a high standard, it is an item everyone learns to skip — and skipping becomes the habit for the items that are possible. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the PR template demanded a reply "on its own thread" for outside-diff findings, which have no thread and no comment id to reply on.</sub>
  <!-- scope: paths=["**/*.md", ".claude/**", ".github/*TEMPLATE*"] change_kinds=[rule_or_policy] -->

## If you changed a permission rule, a token scope, or an allowlist

- **A path prefix cannot express "read-only" for a tool that speaks HTTP.** `gh api <path>:*` matches `--method DELETE` on that path just as happily as a GET, so allowlisting a raw API client by endpoint grants mutation you did not mean to grant. Allowlist the *verbs* that can only read (`gh pr view`, `gh issue list`), and let the raw client prompt. <sub>**origin** #99 (2026-09-03, `Bash(gh api repos/garnizeh/draughts/pulls:*)` was added meaning "read reviews" and also permitted deleting any comment on the repository; both rules were dropped) · **missed ×0** · **saved ×0**. The same rule had a second defect worth remembering: the allowlist matched `gh api repos/...` while every documented command was `gh api --paginate repos/...`, so it granted too much *and* covered nothing anyone actually ran.</sub>
  <!-- scope: paths=["**/settings.json", ".github/workflows/**"] change_kinds=[permission_rule] -->

## If you changed a validator, a parser, or a gate

- **Check that it rejects the *missing* case, not only the wrong one.** A validator that catches a thing in the wrong place will happily accept a file where the thing is absent entirely, and absence is usually the likelier mistake. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `changelog-check` rejected `[Unreleased]` below the first section but accepted a file with no `[Unreleased]` at all.</sub>
  <!-- scope: paths=["scripts/**", "src/config/**"] change_kinds=[validator] -->
- **If two places decide the same question, make them share the decision or prove they agree.** Two definitions of one predicate drift, and the drift shows up as one gate passing what the next one fails. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `release-notes` accepted an undated heading that `release-check` rejected, turning "not ready yet" into a red job on `main`.</sub>
  <!-- scope: paths=["scripts/**", "src/**", "justfile"] change_kinds=[validator, source] -->
- **Prove a new check fails on the defect it exists for.** Reconstruct the pre-fix state and watch it go red. A check that has never been seen to fail is decoration, and nobody will notice when it stops working. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, applied to all four new guards.</sub>
  <!-- scope: paths=["scripts/**", "justfile", ".github/workflows/**"] change_kinds=[validator, ci_workflow] -->

## If you changed a write path

- **Anything the tree calls immutable needs the write path to say so.** Documentation does not stop `write_text`. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the CHANGELOG rotation could overwrite an archived release's notes, which the archive's own README calls permanent.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **Check every target before writing any of them.** A collision discovered halfway through leaves the tree in a state neither the old nor the new one. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, same finding.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **A guard against overwriting must let a run resume itself.** Output identical to what this run would write is not a collision — it is this run, interrupted. Refusing it makes the retry impossible and the only way out a manual delete. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the collision guard added for one review made a half-finished rotation unrecoverable, and the next review caught it.</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path] -->
- **A string from a document that becomes a filename is a path.** Validate it against the grammar it is supposed to have before it is joined to a directory — `## [../../CLAUDE]` is a legal Markdown heading — and **guard the whole path, not the leaf**: resolve the directory you are about to write into and require it to stay under the repository root, before creating it and before any write. A check on the final file catches a symlinked file and misses a symlinked parent, which is the same escape one level up. <sub>**origin** #99 (2026-09-02, `## [../../CLAUDE]` reached `ARCHIVE / f"{version}.md"` as a path, and `exists()` follows symlinks so a dangling one reads as absent and gets written through) · **missed ×1** #99 (2026-09-03, the fix guarded `page.is_symlink()` only; if `docs/changelog/` were itself a link, a fresh `<version>.md` inside it is not a symlink and `write_text` lands outside the repository) · **saved ×0**. Hit rate 0/1. **A guard placed at the level the last bug happened at is not a guard on the property.**</sub>
  <!-- scope: paths=["scripts/**", "src/db/**"] change_kinds=[write_path, validator] -->

## If you wrote a table, a list, or anything with a syntax

- **A linter exists for the format you are writing, and the reviewer has it.** A footnote marker after a table row's final pipe makes a fifth cell; nothing about the source line looks wrong, and the rendered table is where it shows. <sub>**origin** #99 (2026-09-03, `| … | \`type:infra\` |¹` in `docs/ROADMAP.md` tripped markdownlint MD055/MD056 in review) · **missed ×0** · **saved ×0**. Candidate mechanization: markdownlint in `just ci`. Not built at ×1, deliberately — the first occurrence buys a rule, and a tool added on one finding is a tool nobody chose.</sub>
  <!-- scope: paths=["**/*.md", "**/*.yml", "**/*.toml"] change_kinds=[markup] -->

## If you changed a `justfile` recipe with prerequisites

- **`just` stops the chain at the first failure, so order by what can fail for reasons other than "the tree is wrong".** Sort into tiers — recipes needing only the pinned toolchain, then recipes needing a tool `just setup` installs, then recipes needing particular hardware — because a failure in the later tiers may mean the host rather than the change. <sub>**origin** #99 (2026-09-02, `check-cuda` sat ahead of `coverage`, so a host with no CUDA toolkit lost its coverage report to something unrelated to it) · **missed ×0** · **saved ×1** #99 (2026-09-03, a `/gate` pass found the origin fix had been applied to the CUDA recipes *only*: `workflows-check` needs actionlint and sat second in the chain, so a host without it also lost `audit`, `portable-check`, `coverage` and both CUDA recipes — and `audit` and `coverage` have the same tool dependency. `pre-pr` is now sorted into the three tiers above). Hit rate 1/1. **Fixing the instance a rule was born from is not fixing the class** — that is what this save actually caught.</sub>
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
