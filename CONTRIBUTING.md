# Contributing

The architecture is settled at version 1.4; the implementation is not finished.
Start with [docs/architecture/](docs/architecture/README.md) — the code is
written against it, and every non-obvious decision in the tree cites the section
that justifies it.

## Getting set up

```bash
just setup            # rustfmt, clippy, cargo-deny
just setup-frontend   # vendor htmx and Alpine into static/vendor/
cp draughts.example.toml draughts.toml
just check-config     # §23.1 validation against this host
just pre-pr           # every CI job, locally — see The gate below
```

`just` is the only entry point you need. Every recipe is listed by `just` on its
own, and CI runs the same recipes — `just ci` locally and a green pipeline mean
the same thing by construction.

## The gate

```bash
just pre-pr
```

is the one to run before opening a pull request: every job `ci.yml` runs, here,
in CI's order. Finding a broken CUDA build or a bad workflow expression locally
costs a minute; finding it on a pushed branch costs a round trip and a red PR.

Two things it cannot fully reproduce, and says so rather than pretending
otherwise: `portable-check` builds outside a driverless container, so CI stays
the authority on §22.1; and `check-cuda`/`build-cuda` need a CUDA toolkit on
this host. If a recipe fails for want of a tool rather than for want of correct
code, `just setup` — and if it still cannot run, name it. A check that did not
run is not a check that passed.

```bash
just ci
```

is one of those six jobs — `fmt-check`, `lint`, `test`, `device-check`,
`format-version-check`, `changelog-check`, `doc-links` and `docs`. Four of those
are static checks that no compiler performs:

- `device-check` asserts `candle_core::Device` is constructed in exactly one
  function ([§19.6.5](docs/architecture/19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve)).
- `format-version-check` asserts every insert path names `format_version`
  ([§20.8](docs/architecture/20-testing-strategy.md#208-format-version-tests)).
- `changelog-check` asserts `CHANGELOG.md` is newest-first and holds no more
  than five released sections. See *Releasing* below.
- `doc-links` asserts every relative link and every `#anchor` in the
  documentation resolves. This tree cites itself several hundred times across
  sixty-odd files, and renaming one heading tells you nothing about the twenty
  references you just broke.

The heavy suites are deliberately outside the gate and run nightly:
`just test-tt-off` and `just bench`. `just test-load` runs neither — its CI
job is commented out in `nightly.yml` until `tests/load.rs`'s `todo!()` bodies
are implemented.

The other five jobs on a pull request are `portable-build` (the default binary,
built *and run* in a container with no driver and no toolkit), `cuda-compile`
(`just check-cuda` and `just build-cuda`), `supply-chain` (`just audit`),
`workflows` (actionlint over `.github/workflows/`), and `coverage`
(`just coverage`). `just pre-pr` is all six.

Coverage is reported and never gated. There is no percentage threshold, because
a threshold against a tree whose unimplemented seams are `todo!()` would measure
the seams — and would be met by deleting them rather than by filling them in.
The lcov file is a run artifact; the summary is in the run's step summary.

## The three rules

These are not style preferences. The design rests on them, and each one is a
review blocker on its own.

1. **Reading a persisted BLOB without dispatching on its `format_version` is a
   defect.**
   ([§13.7](docs/architecture/13-data-dictionary.md#137-format_version--new-in-11))
   The column's `DEFAULT` exists for the v1.0 migration and for nothing else.

2. **The transposition table may change how long a search takes and must never
   change what it returns.**
   ([§20.5](docs/architecture/20-testing-strategy.md#205-transposition-table-tests))
   Anything touching `probe`, `store`, or `EvaluatorIdentity` needs the
   differential search test to pass at 1, 2, 8 and 10 threads.

3. **`candle_core::Device` is constructed in exactly one function.**
   ([§19.6.5](docs/architecture/19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve))
   A second construction anywhere is what turns the next device change from a
   one-line edit into a search-and-replace.

## Two more that are easy to break by accident

- **The LLM never plays.** It cannot choose, validate, or influence a move, and
  the constraint is enforced by types: `CommentaryContext` is the whole of what
  the Face layer is given, and there is no move in it
  ([§2.3](docs/architecture/02-scope-and-constraints.md#23-explicit-constraint-llm-does-not-play-draughts)).
  Adding a field to that struct is an architectural change, not a convenience.

- **A change that can only be tested on a GPU has broken the CPU path.** The
  whole suite runs on `face.device = "cpu"`, on the default build, on a runner
  with no driver — that is the gate
  ([§20.10](docs/architecture/20-testing-strategy.md#2010-device-parity-and-cuda-tests)).

## Working on the GPU path

```bash
CUDA_COMPUTE_CAP=86 just check-cuda   # compiles without a device
CUDA_COMPUTE_CAP=86 just build-cuda   # needs a toolkit
```

CI compiles the `cuda` feature on every PR and runs nothing on a device. The
device-requiring half of §20.10 runs on the target host only, and making it a
merge gate would reintroduce the GPU dependency the section exists to prevent.

If `cudarc`'s supported toolkit range lags the installed driver's CUDA version,
install a toolkit in range and point the build at it. That is the fix, not a
driver downgrade
([§22.1](docs/architecture/22-deployment-model.md#221-mvp-single-machine-deployment)).

## Conventions

- Comments explain *why*, and cite the section that decided it. The codebase is
  full of numbers that look arbitrary and are not; a bare constant with no
  citation is the thing most likely to be "cleaned up" into a bug.
- Tests are named as the property they assert, not as the function they call.
- New performance numbers go in
  [Appendix B](docs/architecture/appendix-b-performance-targets.md), not in a
  comment.
- Changing a persisted encoding — including regenerating the Zobrist key table —
  is a `format_version` bump. `just test` will tell you; do not update the
  expected constant to make it stop.

## Line width

**Prose is not hard-wrapped at a column count.** Not at 80, not at any number. Markdown, commit-message bodies, pull request and issue text, YAML comments: one paragraph is one line, and a long paragraph breaks at a sentence boundary if it breaks at all.

Every renderer that shows this text reflows it. Monitors are wide. And a fixed wrap makes a one-word edit rewrite an entire paragraph in the diff, which buries the change that mattered under the reflow that did not.

Two exceptions, and they are the only two:

- **Source files** follow their own language's convention — Rust `rustfmt.toml`'s `max_width = 100`, enforced by `just fmt-check`; Python PEP 8. Comments inside them match the code they sit in: source does not reflow, and a comment wider than the code it annotates reads badly in a split editor. This is why `scripts/*.py` prose is wrapped and `CHANGELOG.md` prose is not.
- **A commit subject line** stays under about 72 characters, because `git log --oneline`, `git shortlog` and GitHub's UI all truncate it. The commit *body* has no such limit and should not be wrapped.

The tree still contains a lot of prose wrapped at 80 from before this rule. Unwrap what you are editing anyway; do not mass-rewrap a file you are not otherwise touching.

## Releasing

**The version in `Cargo.toml` is the source of truth, `CHANGELOG.md` is the
gate, and nobody runs `git tag`.**

`release.yml` runs on every push to `main` and asks two questions. Does
`v$(just version)` already have a tag? Does `CHANGELOG.md` have a *closed*
section for that version — a dated `## [x.y.z] - YYYY-MM-DD` heading, not
`[Unreleased]`? Almost every commit answers "already tagged" or "not closed",
the job says so and exits green, and nothing happens. When the version is new
*and* its notes are written, it cuts an annotated tag, re-runs the gate at that
tag, builds, and publishes.

That ordering is deliberate: nothing ships whose notes nobody wrote, and no bot
rewrites the CHANGELOG. It stays prose, in one voice, like the rest of the tree.

A release is an ordinary pull request containing only these:

1. `version` bumped in `Cargo.toml`.
2. `cargo update -p draughts`, so `Cargo.lock` records it. A stale lock means
   the tarball is built from a dependency graph nobody wrote down.
3. `## [Unreleased]` renamed to `## [x.y.z] - YYYY-MM-DD`, a fresh empty
   `## [Unreleased]` opened above it, and the link references updated.
4. `just changelog-rotate` if that pushed the file past five released sections.
5. `just release-check x.y.z` printing `x.y.z is ready`.

Merge it; the tag and the release appear by themselves. A release commit that
also changes behaviour is a release whose notes are wrong.

Two tarballs ship, Linux x86-64 only, each with a `.sha256` that CI verifies
before publishing:

- **portable** — default features, built inside a container with no driver and
  no toolkit and then *run* there. Its linkage is asserted by
  `scripts/check-no-cuda-linkage.sh`, the same check `ci.yml` makes, because a
  missing CUDA library shows up at load time rather than at link time.
- **cuda** — `--features cuda`, toolkit 12.6.0, built but never run. It carries
  a `CUDA.md` saying plainly that it needs the CUDA runtime on the host: the
  feature adds a device to the *engine*, but it adds a requirement to the
  *executable*. §22.1 is written around the portable one.

To rebuild an existing tag — a lost artifact, not a new version — use the
workflow's `workflow_dispatch` input. It never creates a tag.

### The CHANGELOG does not grow without bound

`CHANGELOG.md` holds `[Unreleased]` and the **five most recent releases, newest
first**. Anything older is archived by `just changelog-rotate` into
`docs/changelog/`, one file per release, indexed in
[docs/changelog/README.md](docs/changelog/README.md). `just changelog-check` is
in the gate, so the file cannot quietly become the thing nobody opens.

One release per archive file rather than five to a file, and the name never
changes once written: a link to it does not rot, and `git log` follows it. The
logrotate habit of shuffling `.1` to `.2` would rewrite every archive on every
release and break both.

Do not edit an archived section. The notes on its GitHub release were rendered
from it at publish time; editing the file afterwards makes the two disagree with
no way to tell which is right.

## Reporting a vulnerability

Not as an issue. [SECURITY.md](SECURITY.md) has the private-reporting link, what
is in scope, and what is deliberately not — the absence of authentication is
documented in [§18](docs/architecture/18-security-and-safety.md), not an
oversight.

## Responding to CodeRabbit

Every CodeRabbit finding gets a reply on its own thread before the PR merges —
never silence, and never only a summary comment elsewhere on the PR. Verify the
finding against the current code first, since it can be stale by the time you
read it — and read the proposed patch sceptically even when the diagnosis is
right, because a correct diagnosis attached to a wrong fix is the ordinary case
rather than the exceptional one. Then:

- **Fixing it:** reply with the commit (and line, if it moved) that fixes it.
- **Not fixing it:** say why, plainly — a documented `todo!()` seam, a design
  question that needs a human decision rather than a unilateral edit, or a
  finding that no longer applies. CodeRabbit reads the reply and either
  resolves the thread or pushes back on it; that loop is the point.

A PR with unanswered CodeRabbit threads is not done, the same way a PR with a
red `just ci` is not done.

**"Every thread" means more than the per-line comments.** CodeRabbit puts
findings it cannot anchor to a changed line inside the review's own body text
— a collapsible **"⚠️ Outside diff range comments"** section — rather than as
individual comments with their own ids. Listing per-line comments (`gh api
--paginate repos/OWNER/REPO/pulls/PR/comments`) never surfaces these; they
exist only in `gh api --paginate repos/OWNER/REPO/pulls/PR/reviews` → each
review's `body`. `--paginate` matters on both calls, not just one: a PR with
enough reviews or comments to span more than one page silently drops the rest
without it, which looks exactly like a clean pass. A pass that only reads the
flat comment list will look complete and still leave findings answered by
nobody. Before calling a CodeRabbit review done: list every review on the PR,
read each `body` in full, and treat an outside-diff finding exactly like an
inline one — verify it, fix or decline it, and reply where a reader can find
the answer (inline if CodeRabbit later promotes it to its own thread,
otherwise one PR comment naming every file and line it covered).

A review that changes nothing about how the next mistake is caught was a review
half-read. When a finding names a *class* of mistake rather than a single
instance, it should leave something behind:
[.claude/skills/review-response/LESSONS.md](.claude/skills/review-response/LESSONS.md)
is that record — a short conditional checklist, *if you changed X, check Y*,
where every line cites the pull requests it was earned from. Read it against
your own diff before opening a PR; it is the cheapest place left to catch the
things no script catches yet.

Each rule carries an **origin** and two counters, all of them lists of dated evidence rather than numbers, so the count and the proof cannot drift apart. `origin` is the finding that created the rule — nothing could have prevented it. **missed** is the times the rule existed, was not applied, and a reviewer caught the mistake anyway. **saved** is the times it was read before a PR and caught the mistake first. Last-useful is the latest date in either counter, and is not written down — a third number is a third thing to forget.

Separating the origin from a miss is the point of the shape. An origin is the world being surprising; a miss is a sentence in this file failing to teach anyone. Only the second criticises the writing, and `saved / (saved + missed)` is the rule's hit rate — of the times it was relevant, how often it actually fired.

A rule with a high `missed` and a zero `saved` is not a stubborn class: **the text is not working**, and rewriting it beats bumping it. A rule with a high `saved` and no misses is working and should be left alone. A rule cold in both since the previous release is a candidate for deletion — it has either been internalised or it is about code nobody touches — and that question, asked at each release, is what stops the file growing from the far end.

Promotion keys on **missed**, and the first miss buys a rewrite of the sentence rather than a promotion: a rule that failed once may simply have been worded badly, and mechanizing a property nobody has managed to state clearly produces a badly-aimed check. At two, anything a script can decide becomes a check in `just ci` — a check costs a second of CI and nothing at all to remember. At five, a rule that only judgment can decide earns a line in `CLAUDE.md`, which is loaded into every session and is therefore the most expensive place in the tree to put a sentence. Either way the line leaves the file, so it stays bounded by graduation rather than by pruning. A check may also be written before its counter asks for one, when the risk is obvious and the property is cheap to assert — say so, and leave the counters at zero.

Rules with a miss are hoisted into a *Failed before* block at the top of the file, and demoted out of it after a release with no new miss. Attention is as scarce a budget as context: if everything is at the top, nothing is.

A rule that is prone to false positives carries a `⚠ Looks like a violation but is not` note inline, listing what will look wrong and is not. That note is part of the rule: someone reading the file against a diff is about to go looking, and a rule that sends them chasing ghosts three times gets ignored the fourth — on the occasion that counted. It is also the cheapest test of whether a rule could ever be a script: exceptions you cannot state crisply for a person will not separate cleanly for a checker either.

One caveat stated in the file itself: `saved` is self-reported and has no review thread behind it. Record one only when you can name what it caught, as concretely as a miss names its finding. Since the hit rate divides by the sum of both, an inflated `saved` corrupts the only number that ranks the rules.

## Filing an issue

Six forms in [.github/ISSUE_TEMPLATE/](.github/ISSUE_TEMPLATE/), chosen by what
the reader has to do about it rather than by what it feels like:

| Form | For |
|---|---|
| Defect | The code crashes, returns the wrong answer, or contradicts a § |
| Invariant violation | One of the five rules above is broken — routes to an architecture review |
| Defect in the architecture or the docs | The **document** is wrong, inconsistent, or rests on an underived number |
| Implementation work | A row from [docs/ROADMAP.md](docs/ROADMAP.md), or a `todo!()` seam |
| Performance regression | A figure moved against [Appendix B](docs/architecture/appendix-b-performance-targets.md) |
| Post-MVP proposal | [§19](docs/architecture/19-extensibility-roadmap.md) and §2.2 — out of scope, not rejected |

Two edges are easy to get wrong. A wrong answer that traces to a broken rule is
an *invariant violation*, not a defect, because it needs an architecture pass
before anything merges against it. And the two budget gates — peak RSS under a
full-density batch, peak VRAM under lab load — are invariant violations too, not
performance issues; §16.1 and §16.6 are gates rather than metrics.

**Fill the forms out fully.** Every field you leave thin is one the reader
cannot reconstruct: paste the panic and the backtrace whole rather than the
frames that look relevant, give the commands that worked as well as the one that
did not, include the part of `draughts.toml` that was in force, the commit, the
`rustc` version, and the device. §20.10 makes a CPU defect and a CUDA defect two
different defects, so an issue without the device cannot even be classified. Say
what you already ruled out, and how — it is the field people skip and the one
that saves the most time. If a field does not apply, say so in a line; a blank
field and a deliberately empty one look identical.

The exception, and it is the only one: **do not restate the specification.** It
is approved at v1.4 and already written. Cite the § and quote it only where the
exact wording is what the issue turns on. Maximum detail means maximum detail
about *your situation* — the part that exists nowhere else.

`gh issue create` does not fill in a form; `--template` is only starting body
text for the interactive editor. A scripted issue should reproduce the form's
field labels as `###` headings, in order, so both routes produce one shape. The
`area:`, `gate:` and `prio:` labels are captured as dropdowns in the body and
have to be applied by hand after filing — a form can only apply a fixed set.

### Reporting a design problem

An issue against the architecture is as valuable as a patch. The eight defects
in [§0.2.2](docs/architecture/00-revision-history.md#022-corrections) were all
found by reading, and the two revisions since were each caused by a single
number nobody had derived. Quote generously when you file one: where two
sections disagree, the clause that settles it is usually the one a paraphrase
drops.

## Working with Claude Code

The repository carries a committed harness: [CLAUDE.md](CLAUDE.md) for the rules
above, and [.claude/](.claude/README.md) for the skills, the review subagent, and
a post-edit hook that runs the two static checks while the change is still in
hand. It is reviewed like any other file — if it describes a tree that no longer
exists, that is a defect.
