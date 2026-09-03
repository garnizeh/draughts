# What reviews have taught this tree

Conditional checks, each one earned from a finding that actually happened here. Read this **before opening a PR**, and again when triaging a new review.

## How this file works

**Every item cites every PR it was earned from, and that list is the counter.** `×3 — #99, #104, #112` is three occurrences, and there is no separate number to drift out of step with the evidence. Bump it by appending the PR, never by editing a digit.

**Where a rule goes next depends on the count and on one question: can a script decide it?**

| Count | A script can decide it | Only judgment can decide it |
|---|---|---|
| ×1 | Fix it, write the line here | Fix it, write the line here |
| **×2** | **Write the check.** Wire it into `just ci`, move the line to *Retired* | Bump the count. Still a checklist line |
| **×5** | — (it should never get here) | **Promote to `CLAUDE.md`.** Move the line to *Retired* pointing at it |

The two thresholds are different because the two destinations cost different things. A script costs a second of CI and nothing at all to remember, so there is no reason to let a mechanizable property recur more than twice. A rule in `CLAUDE.md` is loaded into every session's context on every turn, forever — that budget is the scarcest thing the harness spends, and five occurrences is the bar for spending it. Five is arbitrary in its exact value and not arbitrary in its size: it should be high enough that the rule has clearly earned permanent residence, low enough that it does not take a year.

**A rule that later becomes mechanizable leaves `CLAUDE.md` for a script.** Promotion is not one-way, and the context budget is worth reclaiming.

**Record rejections too.** "Raised and deliberately not fixed, because X" stops the same argument being had twice.

**If this file is growing and nothing is graduating, the harvest phase is being skipped.** That is the signal to look for, not the length.

---

## If you changed documentation

- **Find every other document that states the same thing, and change it too.** This tree says the same things in `CLAUDE.md`, `CONTRIBUTING.md`, `README.md`, `.claude/README.md` and the skills, on purpose — each for a different reader. A change to the gate, the release procedure or the five rules touches four or five files, and the ones you forget are the ones that quietly start lying. <sub>×3 — #99, three separate times. `just pre-pr` and CHANGELOG rotation needed edits in all five files; the pre-implementation sweep found `README.md` still saying "three rules" while seventeen other places said five, and `ROADMAP.md`'s definition of done — which ninety-four filed issues link to — still saying `just ci`; then a review found `/gate`'s frontmatter still advertising `just ci` while its body said `pre-pr`, and `CLAUDE.md`'s job list omitting `portable-check`. The link-shaped half graduated to `check-doc-links.py`; the semantic half is what is left, and no script decides it. **Frontmatter and metadata are documentation too** — that is where two of the three misses were.</sub>
- **Never cite a line number in a document.** It is wrong by the next commit. Cite the file and something stable inside it — a `todo!()` message, a function name, a heading. <sub>×1 — #99's pre-implementation sweep: five of the eleven seam citations in `ROADMAP.md` already pointed at the wrong line, before a single seam had been touched.</sub>
- **If the change contradicts `docs/architecture/`, stop and say so first.** The document is version 1.4 and approved; the code is the unfinished part. A disagreement is a defect in the code unless you can show the document is wrong — and if you can, that is a conversation before it is a commit. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>
- **A constant with no § is the thing most likely to be "cleaned up" into a bug.** New numbers carry the section that decided them. <sub>Standing rule, already in `CLAUDE.md` — restated here because it is what a diff should be read against.</sub>

## If you changed CI, a workflow, or anything it pulls in

- **A container image tag is exactly as mutable as an action tag.** `uses:` is pinned to a commit here; `container:` and `image:` must be pinned to a digest for the same reason, and the digest to use is the multi-arch index so platform resolution still works. <sub>×1 — #99's pre-implementation sweep: `rust:1.98.0-slim-bookworm` was a floating tag in both `ci.yml` and `release.yml`, and it is the container that builds the binary a release ships.</sub>

## If you wrote a rule, a policy, or a checklist

- **State what it says about everything it will be read against, or it will be read against them anyway.** A rule that names one case leaves every other case to be argued about later, by someone who will reasonably read the omission as permission or as prohibition — whichever suits. <sub>×1 — #99: the no-hard-wrap rule named Rust source and said nothing about Python, so a reviewer correctly read PEP 8 comments in `scripts/*.py` as a violation of it. The rule was incomplete, not the code.</sub>
- **Do not require something the platform cannot do.** A checklist item that is impossible is not a high standard, it is an item everyone learns to skip — and skipping becomes the habit for the items that are possible. <sub>×1 — #99: the PR template demanded a reply "on its own thread" for outside-diff findings, which have no thread and no comment id to reply on.</sub>

## If you changed a validator, a parser, or a gate

- **Check that it rejects the *missing* case, not only the wrong one.** A validator that catches a thing in the wrong place will happily accept a file where the thing is absent entirely, and absence is usually the likelier mistake. <sub>×1 — #99: `changelog-check` rejected `[Unreleased]` below the first section but accepted a file with no `[Unreleased]` at all.</sub>
- **If two places decide the same question, make them share the decision or prove they agree.** Two definitions of one predicate drift, and the drift shows up as one gate passing what the next one fails. <sub>×1 — #99: `release-notes` accepted an undated heading that `release-check` rejected, turning "not ready yet" into a red job on `main`.</sub>
- **Prove a new check fails on the defect it exists for.** Reconstruct the pre-fix state and watch it go red. A check that has never been seen to fail is decoration, and nobody will notice when it stops working. <sub>×1 — #99, applied to all four new guards.</sub>

## If you changed a write path

- **Anything the tree calls immutable needs the write path to say so.** Documentation does not stop `write_text`. <sub>×1 — #99: the CHANGELOG rotation could overwrite an archived release's notes, which the archive's own README calls permanent.</sub>
- **Check every target before writing any of them.** A collision discovered halfway through leaves the tree in a state neither the old nor the new one. <sub>×1 — #99, same finding.</sub>
- **A guard against overwriting must let a run resume itself.** Output identical to what this run would write is not a collision — it is this run, interrupted. Refusing it makes the retry impossible and the only way out a manual delete. <sub>×1 — #99: the collision guard added for one review made a half-finished rotation unrecoverable, and the next review caught it.</sub>
- **A string from a document that becomes a filename is a path.** Validate it against the grammar it is supposed to have, before it is joined to a directory. `## [../../CLAUDE]` is a legal Markdown heading. <sub>×1 — #99. And `exists()` follows symlinks, so a dangling one reads as absent and gets written through; refuse a symlink whatever it points at.</sub>

## If you changed a `justfile` recipe with prerequisites

- **`just` stops the chain at the first failure, so order by what can fail for reasons other than "the tree is wrong".** Hardware-specific and tool-specific recipes go last, or they take unrelated checks down with them. <sub>×1 — #99: `check-cuda` sat ahead of `coverage`, so a host with no CUDA toolkit lost its coverage report to something unrelated to it.</sub>
- **`just --list` shows the *last* comment line before a recipe.** A multi-line prose comment leaves a sentence fragment as the recipe's description. Put a blank line, then a one-line summary. <sub>×1 — learned building #99; the tree already had two recipes described by half a sentence.</sub>

## If you are acting on someone else's suggested patch

- **Read the diff before applying it.** A correct diagnosis attached to a wrong fix was two of PR #99's four findings — not an edge case. Take the diagnosis seriously and the patch sceptically. <sub>×1 — #99.</sub>
- **Watch for a value interpolated into a pattern.** Version numbers are full of dots, and a dot in a regex matches anything: `0.2.0` also matches `0X2X0`. Match literally and pattern only the part that is genuinely variable. <sub>×1 — #99, in a suggested diff that was not applied as written.</sub>
- **An exit code must mean one thing.** A status that means both "your change is broken" and "this machine lacks a toolkit" cannot be acted on, and buries the first inside the second. <sub>×1 — #99: a suggestion to continue past a failed `check-cuda` and return "incomplete", rejected for this reason.</sub>

---

## Retired

A rule lands here when it graduates, with what replaced it and the PR list it earned on — the provenance is the point, and it must survive the promotion.

- **To a script, at ×2 — "a renamed heading breaks every reference to it, silently."** Now `scripts/check-doc-links.py`, run by `just doc-links` in `just ci`: every relative link and every `#anchor` across the documentation resolves, or the gate is red. <sub>×2 — #99. This is the link-shaped half of the doc-sync rule above; the half that needs judgment stayed there, because no script can tell that `README.md` saying "three rules" and `CLAUDE.md` saying "five" is a contradiction rather than two true sentences.</sub>

The shape of an entry: **to a script**, at ×2, name the check and the recipe that runs it. **To `CLAUDE.md`**, at ×5, quote the rule as it was written there, so anyone wondering why that line exists in the project instructions can find the five findings that put it there.
