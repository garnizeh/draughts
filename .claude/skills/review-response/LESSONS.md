# What reviews have taught this tree

Conditional checks, each one earned from a finding that actually happened here. Read this **before opening a PR**, and again when triaging a new review.

---

## ⚠ Failed before — read these twice

These rules were in this file, were not applied, and a reviewer caught the mistake anyway. They sit at the top because a rule that has already failed once is the likeliest to fail again, and because a miss is evidence about *this file's writing*, not about the world.

Each stays in its conditional section below as well — hoisting it out would hide it from anyone scanning by kind of change, which is the other way this file gets read. This block is the index, not the home.

| Rule | Section | Hit rate | The miss |
|---|---|---|---|
| Find every other document that states the same thing | [documentation](#if-you-changed-documentation) | 1/2 | `/gate`'s frontmatter still advertised `just ci` while its body said `pre-pr`, and `CLAUDE.md`'s job list omitted `portable-check` — both in metadata around a list, not in the list |

**A rule leaves this block after a full release with no new miss, and goes back to living in its section alone.** It does not leave because someone rewrote it and feels better about the wording; it leaves on evidence, like everything else here.

Demotion is not tidying. Attention is the same kind of scarce budget as `CLAUDE.md`'s context: **if everything is at the top, nothing is**, and a block that only ever grows is one people stop reading at exactly the moment it matters. The reader who opens this file before a PR can hold three or four highlighted rules in their head. That is the real capacity, and it is what demotion protects.

The consequence is that nothing lives here permanently, by construction. A rule either stops being missed and drops back down, or keeps being missed and graduates out — to a script at ×2, to `CLAUDE.md` at ×5. A rule that has sat here across several releases with the same miss count is not a stubborn rule; it is a rule nobody has demoted, and that is a bookkeeping failure rather than a finding.

---

## How this file works

Every item carries **an origin and two counters, and none of them is a number you type** — all are lists of dated evidence, so the count and the proof cannot drift apart.

- **origin** — the finding that created the rule. Exactly one, always: the rule did not exist yet, so nothing could have prevented it.
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

**Record rejections too.** "Raised and deliberately not fixed, because X" stops the same argument being had twice.

**If this file is growing and nothing is graduating, the harvest phase is being skipped.** That is the signal to look for, not the length.

---

## If you changed documentation

- **Find every other document that states the same thing, and change it too.** This tree says the same things in `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `.claude/README.md` and the skills, on purpose — each for a different reader. A change to the gate, the release procedure or the five rules touches four or five files, and the ones you forget are the ones that quietly start lying. <sub>**origin** #99 (2026-09-02, `just pre-pr` and CHANGELOG rotation needed edits in all five files) · **missed ×1** #99 (2026-09-03, a review found `/gate`'s frontmatter still advertising `just ci` while its body said `pre-pr`, and `CLAUDE.md`'s job list omitting `portable-check`) · **saved ×1** #99 (2026-09-02, the pre-implementation sweep read this rule against its own diff and found `README.md` still saying "three rules" where seventeen other places said five, and `ROADMAP.md`'s definition of done — which ninety-four filed issues link to — still saying `just ci`). Hit rate 1/2. The miss bought a rewrite rather than a promotion, and this is the sentence it bought: **frontmatter and metadata are documentation too** — both halves of that miss were in metadata around a list, not in the list. The link-shaped half of this rule graduated to `check-doc-links.py`; the semantic half is what is left, and no script decides it.</sub>
- **Never cite a line number in a document.** It is wrong by the next commit. Cite the file and something stable inside it — a `todo!()` message, a function name, a heading. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99's pre-implementation sweep: five of the eleven seam citations in `ROADMAP.md` already pointed at the wrong line, before a single seam had been touched.</sub>
- **If the change contradicts `docs/architecture/`, stop and say so first.** The document is version 1.4 and approved; the code is the unfinished part. A disagreement is a defect in the code unless you can show the document is wrong — and if you can, that is a conversation before it is a commit. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
- **A constant with no § is the thing most likely to be "cleaned up" into a bug.** New numbers carry the section that decided them. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>

## If you changed CI, a workflow, or anything it pulls in

- **A container image tag is exactly as mutable as an action tag.** `uses:` is pinned to a commit here; `container:` and `image:` must be pinned to a digest for the same reason, and the digest to use is the multi-arch index so platform resolution still works. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99's pre-implementation sweep: `rust:1.98.0-slim-bookworm` was a floating tag in both `ci.yml` and `release.yml`, and it is the container that builds the binary a release ships.</sub>

## If you wrote a rule, a policy, or a checklist

- **State what it says about everything it will be read against, or it will be read against them anyway.** A rule that names one case leaves every other case to be argued about later, by someone who will reasonably read the omission as permission or as prohibition — whichever suits. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the no-hard-wrap rule named Rust source and said nothing about Python, so a reviewer correctly read PEP 8 comments in `scripts/*.py` as a violation of it. The rule was incomplete, not the code.</sub>
- **Do not require something the platform cannot do.** A checklist item that is impossible is not a high standard, it is an item everyone learns to skip — and skipping becomes the habit for the items that are possible. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the PR template demanded a reply "on its own thread" for outside-diff findings, which have no thread and no comment id to reply on.</sub>

## If you changed a validator, a parser, or a gate

- **Check that it rejects the *missing* case, not only the wrong one.** A validator that catches a thing in the wrong place will happily accept a file where the thing is absent entirely, and absence is usually the likelier mistake. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `changelog-check` rejected `[Unreleased]` below the first section but accepted a file with no `[Unreleased]` at all.</sub>
- **If two places decide the same question, make them share the decision or prove they agree.** Two definitions of one predicate drift, and the drift shows up as one gate passing what the next one fails. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `release-notes` accepted an undated heading that `release-check` rejected, turning "not ready yet" into a red job on `main`.</sub>
- **Prove a new check fails on the defect it exists for.** Reconstruct the pre-fix state and watch it go red. A check that has never been seen to fail is decoration, and nobody will notice when it stops working. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, applied to all four new guards.</sub>

## If you changed a write path

- **Anything the tree calls immutable needs the write path to say so.** Documentation does not stop `write_text`. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the CHANGELOG rotation could overwrite an archived release's notes, which the archive's own README calls permanent.</sub>
- **Check every target before writing any of them.** A collision discovered halfway through leaves the tree in a state neither the old nor the new one. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, same finding.</sub>
- **A guard against overwriting must let a run resume itself.** Output identical to what this run would write is not a collision — it is this run, interrupted. Refusing it makes the retry impossible and the only way out a manual delete. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: the collision guard added for one review made a half-finished rotation unrecoverable, and the next review caught it.</sub>
- **A string from a document that becomes a filename is a path.** Validate it against the grammar it is supposed to have, before it is joined to a directory. `## [../../CLAUDE]` is a legal Markdown heading. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99. And `exists()` follows symlinks, so a dangling one reads as absent and gets written through; refuse a symlink whatever it points at.</sub>

## If you changed a `justfile` recipe with prerequisites

- **`just` stops the chain at the first failure, so order by what can fail for reasons other than "the tree is wrong".** Hardware-specific and tool-specific recipes go last, or they take unrelated checks down with them. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: `check-cuda` sat ahead of `coverage`, so a host with no CUDA toolkit lost its coverage report to something unrelated to it.</sub>
- **`just --list` shows the *last* comment line before a recipe.** A multi-line prose comment leaves a sentence fragment as the recipe's description. Put a blank line, then a one-line summary. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — learned building #99; the tree already had two recipes described by half a sentence.</sub>

## If you are acting on someone else's suggested patch

- **Read the diff before applying it.** A correct diagnosis attached to a wrong fix was two of PR #99's four findings — not an edge case. Take the diagnosis seriously and the patch sceptically. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99.</sub>
- **Watch for a value interpolated into a pattern.** Version numbers are full of dots, and a dot in a regex matches anything: `0.2.0` also matches `0X2X0`. Match literally and pattern only the part that is genuinely variable. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99, in a suggested diff that was not applied as written.</sub>
- **An exit code must mean one thing.** A status that means both "your change is broken" and "this machine lacks a toolkit" cannot be acted on, and buries the first inside the second. <sub>**origin** #99 (2026-09-02) · **missed ×0** · **saved ×0** — #99: a suggestion to continue past a failed `check-cuda` and return "incomplete", rejected for this reason.</sub>

---

## Retired

A rule lands here when it graduates, with what replaced it and the evidence it earned on — the provenance is the point, and it must survive the promotion.

**A check may also be written before its counter asks for one**, when the risk is obvious and the property is cheap to assert. Say so in the entry and leave the counters at zero. The thresholds above are a floor on when you *must* act, never a ceiling on when you may.

- **Written preemptively, not earned — "a renamed heading breaks every reference to it, silently."** Now `scripts/check-doc-links.py`, run by `just doc-links` in `just ci`: every relative link and every `#anchor` across the documentation resolves, or the gate is red. <sub>**no origin** · **missed ×0** · **saved ×0**. Recorded plainly: this check has never caught a broken link, because there were none to catch — the sweep that produced it found sixty-two files fully consistent. It was written because the risk is obvious and the property is cheap to assert, not because the counter said so. That is allowed; pretending otherwise would put invented evidence in the one file whose whole value is that its evidence is real. It is the link-shaped half of the documentation-sync rule above; the half that needs judgment stayed there, because no script can tell that `README.md` saying "three rules" and `CLAUDE.md` saying "five" is a contradiction rather than two true sentences.</sub>

The shape of an entry: **to a script**, at ×2, name the check and the recipe that runs it. **To `CLAUDE.md`**, at ×5, quote the rule as it was written there, so anyone wondering why that line exists in the project instructions can find the five findings that put it there.
