# The Claude Code harness

Committed, and reviewed like any other file in the tree. It exists so that an
agent working here reaches the same conclusions a contributor would after
reading `CONTRIBUTING.md` and `docs/architecture/` — and reaches them before
writing code rather than after CI rejects it.

```text
CLAUDE.md                       project instructions, loaded every session
.claude/
  settings.json                 permissions, env, hooks
  hooks/invariant-guard.sh      runs the two static checks after every edit
  skills/                       loaded on demand, by description match
    architecture-map/           finding the authoritative § for a question
    implement-seam/             turning a todo!() into an implementation
    merge-gate/                 just ci, its failures, and what is nightly
    persisted-format/           the format_version discipline (§13.7, §20.8)
    face-layer/                 device, profiles, breaker, "the LLM never plays"
    transposition-safety/       the determinism contract (§6.7, §20.5)
    file-issue/                 the six forms, and the detail an issue owes a reader
    releasing/                  the version, the CHANGELOG gate, what release.yml does
  agents/architecture-reviewer  reviews a diff against the five rules
  commands/                     /gate /seam /arch /review /release
scripts/
  check-device-construction.sh  §19.6.5 — one Device constructor
  check-format-version.sh       §20.8 — every insert names format_version
  check-no-cuda-linkage.sh      §22.1 — the default build needs no CUDA library
  rotate-changelog.py           CHANGELOG.md stays newest-first and five deep
docs/changelog/                 archived releases, one file per version
.github/
  ISSUE_TEMPLATE/               the forms; the file-issue skill writes to their shape
  PULL_REQUEST_TEMPLATE.md      the five rules as a checklist, plus the gate output
  CODEOWNERS                    the paths where a local-looking change is not
  workflows/                    ci, nightly, release, scorecard, auto-author-assign
SECURITY.md                     scope, private reporting, what is deliberately not a bug
```

Two things the harness deliberately does not automate. `cargo publish` is denied
outright — the crate is `publish = false` and its releases are GitHub releases,
not crates.io. And **nothing here runs `git tag`**: a release is cut by
`release.yml` when a version bump lands on `main` with its CHANGELOG section
closed, so a hand-cut tag is a way to skip the only gate that guarantees the
notes exist. The `releasing` skill says so at length.

`.claude/settings.local.json` is git-ignored for per-developer overrides. It
can widen what is allowed, but it cannot narrow what `settings.json` asks about:
permission rules resolve by type, `deny` > `ask` > `allow`, regardless of which
file they came from. An `allow` in the local file will not silence a matching
`ask` in the committed one — the rule has to leave the `ask` list. That is why
`git commit` and `git push` are not in it: an unmatched command still prompts in
the default mode, so listing them bought a second prompt rather than a first one.

## What it encodes

The five rules in `CLAUDE.md` are the ones from `CONTRIBUTING.md`. Three are
enforced mechanically — two by `just ci` greps, one by the differential search
test — and the harness front-loads them: the skills explain *why* each holds, so
that a change which trips one gets fixed rather than worked around.

It also encodes how the text itself is written. Prose is never hard-wrapped at a column count — the two exceptions are Rust source, which follows `rustfmt.toml`'s `max_width = 100` under the gate, and a commit subject line, which tooling truncates. Everything else reflows in whatever renders it, so a fixed wrap buys nothing and costs a paragraph-wide diff on every one-word edit.

It also encodes *where* checking happens. `/gate` and `/review` both run
**`just pre-pr`**, not `just ci`: `just ci` is one job of six, and the other
five have caught things it structurally cannot see. Everything found locally is
something a pushed branch would have found a round trip later. Where a recipe
cannot run on this host — the CUDA path needs a toolkit — the instruction is to
say so by name, never to round it up to green.

## Keeping it honest

- If a `todo!()` seam is implemented, update the seam list in
  `skills/implement-seam/SKILL.md` and `CHANGELOG.md`.
- If a section is renumbered, the `architecture-map` table moves with it.
- If a recipe is added to `just ci`, or a job is added to `ci.yml`, the triage
  table in `skills/merge-gate/` gains a row **and** `just pre-pr` gains the
  recipe. A CI job with no local equivalent is a job that only ever fails after
  a push.
- If the release procedure changes — a new artifact, a different gate, another
  target — `skills/releasing/` and `CONTRIBUTING.md`'s *Releasing* section move
  with it. They describe a workflow that runs unattended, which is exactly the
  kind of documentation that rots without anyone noticing.
- Every `uses:` in `.github/workflows/` names a commit, with its tag in a
  trailing comment. Dependabot maintains the pair; a hand-edited pin that loses
  the comment stops being maintained.
- If a field or a dropdown option changes in `.github/ISSUE_TEMPLATE/`, the
  heading vocabulary in `skills/file-issue/` changes with it. `gh issue create`
  bypasses the forms, so that skill is the only thing keeping a scripted issue
  and a web-filed one the same shape.
- Every CodeRabbit review comment gets a reply on its own thread before a PR is
  done — pointing at the fix, or stating why it stands as is. See
  `CONTRIBUTING.md`.

A harness that describes a tree that no longer exists is worse than none.
