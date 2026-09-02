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
  agents/architecture-reviewer  reviews a diff against the five rules
  commands/                     /gate /seam /arch /review
.github/ISSUE_TEMPLATE/         the forms themselves; the skill writes to their shape
```

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

## Keeping it honest

- If a `todo!()` seam is implemented, update the seam list in
  `skills/implement-seam/SKILL.md` and `CHANGELOG.md`.
- If a section is renumbered, the `architecture-map` table moves with it.
- If a recipe is added to `just ci`, the triage table in `skills/merge-gate/`
  gains a row.
- If a field or a dropdown option changes in `.github/ISSUE_TEMPLATE/`, the
  heading vocabulary in `skills/file-issue/` changes with it. `gh issue create`
  bypasses the forms, so that skill is the only thing keeping a scripted issue
  and a web-filed one the same shape.
- Every CodeRabbit review comment gets a reply on its own thread before a PR is
  done — pointing at the fix, or stating why it stands as is. See
  `CONTRIBUTING.md`.

A harness that describes a tree that no longer exists is worse than none.
