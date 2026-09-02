---
name: file-issue
description: Filing a GitHub issue against draughts, or transcribing a docs/ROADMAP.md row into one — a defect, an invariant violation, an architecture finding, a seam, a performance regression, or a post-MVP proposal. Use whenever asked to open, file, report, or raise an issue, whenever `gh issue create` is about to run, and whenever the roadmap is being transcribed into GitHub milestones and issues.
---

# Filing an issue

Six forms live in `.github/ISSUE_TEMPLATE/`. They exist because this project
answers questions with a `§`, and a form that does not ask for one produces
issues nobody can act on without re-deriving the whole context first.

| Form | For |
|---|---|
| `1-defect.yml` | The code crashes, returns the wrong answer, or contradicts a § |
| `2-invariant-violation.yml` | One of the five rules in `CLAUDE.md` is broken |
| `3-architecture-defect.yml` | The **document** is wrong, inconsistent, or rests on an underived number |
| `4-implementation.yml` | A roadmap row, or a `todo!()` seam |
| `5-performance.yml` | A figure moved against Appendix B, or a target has no measurement |
| `6-post-mvp.yml` | §19 and §2.2 — out of scope for the MVP, not rejected |

Pick by *what the reader has to do about it*, not by what it feels like. A wrong
answer that traces to a broken rule is `2`, not `1`, because it routes to an
`architecture-reviewer` pass. A figure that misses one of the two budget gates —
peak RSS and peak VRAM — is `2` as well, not `5`; §16.1 and §16.6 are gates.

## Detail is the whole point

**Give every detail you have. All of it.** An issue is read once by someone who
was not there, and everything they cannot reconstruct is lost the moment it is
left out. Costing a reader one round trip for a version string is worse than a
long issue, every time.

Concretely, on any issue that reports something observed:

- **Verbatim, not paraphrased.** The whole panic with its backtrace, the whole
  failing assertion with both sides of it, the whole response body, the whole
  `just ci` tail. Never "it panics in the writer" when the panic text exists.
  Never trim a backtrace to the frames that look relevant — the judgement about
  which frames matter is the reader's, and it is made from the ones you dropped.
- **The exact commands**, in order, from a clean checkout, including the ones
  that worked. A reproduction that starts at step four is not a reproduction.
- **The configuration.** Paste the relevant part of `draughts.toml`. Most
  behaviour in this tree is a function of `time_budget_ms`, worker count, cache
  sizes, or `face.device`, and an issue without them is a story about a machine
  nobody can identify.
- **The environment**: commit SHA, `rustc --version`, OS, and — for anything
  touching the Face layer — device, driver, and toolkit versions. §20.10 makes a
  CPU defect and a CUDA defect two different defects; without the device the
  issue cannot be classified at all.
- **Numbers with their units, their spread, and their n.** A p99 with no sample
  count is a rumour. Say what the host was and whether it was otherwise idle
  (§2.4).
- **What you already ruled out**, and how. This is the field people skip and the
  one that saves the most time — it stops the reader repeating an experiment you
  have already run.

**The one thing not to expand is the specification.** Do not restate what §6.4
says about tree search; cite §6.4. The architecture is approved at v1.4, it is
already written, and a paraphrase of it inside an issue is a second source of
truth that will drift. Maximum detail means maximum detail *about your
situation* — the part that exists nowhere else. Cite the section, quote it only
where the exact wording is the thing in dispute.

If a field does not apply, say why in one line rather than leaving it blank. A
blank field and a deliberately empty one look identical to the reader, and only
one of them means anything.

## Creating an issue from the command line

`gh issue create` does not fill in a form. `--template` is starting body text
for the interactive editor; a scripted `gh issue create --title ... --body ...`
bypasses `.github/ISSUE_TEMPLATE/` entirely. So an agent-created issue only
matches a web-created one if it is written to match.

GitHub renders a submitted form as `###` headings carrying the field's **label**,
each followed by the value. Reproduce that exactly — same labels, same order as
the form's `body:` — so that both routes produce one shape:

```markdown
### What happens

The writer actor panics on the second batch commit.

### What should happen, and which §

§11.2.2 — the actor loop drains the channel and commits in 50 000-row
transactions; a full channel is backpressure (§11.2.5), not a panic.
```

Read the form's YAML before writing the body. Its field labels and dropdown
option strings are the vocabulary; inventing a near-miss variant is what makes
issues unsearchable six months later.

## Labels

An issue form can only apply a fixed label set, so the `area:`, `gate:` and
`prio:` labels are captured as dropdowns in the body and **must be applied by
hand** after filing:

```
area:rules area:engine area:db area:api area:ui area:lab area:face area:config area:ci
type:seam type:test type:perf type:docs type:infra
gate:determinism gate:format-version gate:memory gate:cpu-only
prio:mvp-blocker prio:mvp prio:post-mvp
```

A `gate:` label is not decoration — it means the issue touches one of the five
rules and needs an `architecture-reviewer` pass before anything merges against
it. Apply it when in doubt; a false positive costs one review, a false negative
costs the invariant.

## Transcribing the roadmap

`docs/ROADMAP.md` is the plan; GitHub is the state. Every milestone becomes a
GitHub milestone, every issue row becomes a GitHub issue, through
`4-implementation.yml`.

- **The ID is permanent and never reused.** `M3-7` means one thing forever, in
  the roadmap and in every commit that cites it. Put it in the Roadmap ID field
  and in the title.
- **Do not mirror status back into the file.** The one exception is the
  *Milestone status* table.
- **Work that is not in the roadmap is a roadmap change first** — add the row in
  the same PR, bump the minor version, and add the revision-history line. An
  issue that exists only in GitHub is a plan that is already wrong.
- Carry the row's § and labels across unchanged. Where the row is terse, the
  issue may add detail about *what lands* and *what the tests must assert* —
  that is expansion of the situation, which is wanted, not restatement of the
  section, which is not.
- Post-MVP work is unmilestoned and goes through `6-post-mvp.yml`.

## Before filing anything

1. **Verify the finding against the current tree.** A `todo!()` seam is not a
   defect; it is `4-implementation.yml`. `grep -rn 'todo!' src` is authoritative.
2. **Read the § you are about to cite**, in full, rather than citing it from
   memory. A wrong citation in an issue propagates into the fix.
3. **Search for the duplicate.** `gh issue list --search` before `gh issue
   create`, always.
