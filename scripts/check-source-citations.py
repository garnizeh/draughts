#!/usr/bin/env python3
"""Every document that points into `src/` points at something that is there.

The tree keeps two lists of its unimplemented seams — `docs/ROADMAP.md`, which
is the plan, and `.claude/skills/implement-seam/SKILL.md`, which is what an
agent reads before implementing one. Both restate something `src/` already
knows, and a restatement drifts: the seam lists spent a while missing a seam
entirely while both files read as complete, and before that they cited line
numbers, five of eleven of which were wrong before a seam had been touched.

Checks two things, both silent failures otherwise:

  * a `todo!()` in `src/` that neither seam list mentions
  * a Markdown file citing a Rust source line number

Neither is an architecture rule with a § behind it. They belong to the harness:
CLAUDE.md says the unfinished parts are `todo!()` at named seams, and
`.claude/README.md` says the seam list moves when one of them does.

A seam is matched by the opening words of its `todo!()` message, because that is
what a list can quote and keep true — the message carries its own § and the
compiler will not let it drift from the code it sits in. Line numbers are not
matched at all; they are rejected, which is the point.

`path.rs#L42` links are deliberately left alone: `scripts/check-doc-links.py`
already decides what to do with those and permits them, and two scripts
answering the same question differently is how a gate starts contradicting
itself.

Run by `just source-citations`, which is part of `just ci`.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "src"

SKIP_DIRS = {"target", ".git", "node_modules"}

SEAM_LISTS = (
    Path("docs/ROADMAP.md"),
    Path(".claude/skills/implement-seam/SKILL.md"),
)

# The first string literal of a `todo!(...)`. `\s*` crosses the newline of the
# multi-line form; a bare `todo!()` has no quote and is skipped, which is how
# the doc comment in `transposition.rs` that talks *about* the seams stays out
# of the list.
TODO = re.compile(r'todo!\(\s*"((?:[^"\\]|\\.)*)', re.DOTALL)

# What a document can quote: an inline-code span.
SPAN = re.compile(r"`([^`\n]+)`")

# `moves.rs:112` and friends. The `#L42` form is check-doc-links.py's business.
LINE_CITATION = re.compile(r"\b[\w./-]*\.rs:\d+")


def seam_key(message: str) -> str:
    """The opening words of a `todo!()` message, normalised for matching.

    A line continuation inside a Rust string keeps the backslash and the
    following indentation in the raw capture; neither survives compilation and
    neither matters here, because a list quotes the beginning of the message.
    """
    text = message.replace("\\\n", " ")
    return " ".join(text.split())


def markdown_files() -> list[Path]:
    return sorted(
        p
        for p in ROOT.rglob("*.md")
        if not SKIP_DIRS.intersection(p.relative_to(ROOT).parts)
    )


def main() -> int:
    problems: list[str] = []

    seams: list[tuple[Path, int, str]] = []
    for path in sorted(SRC.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        for match in TODO.finditer(text):
            key = seam_key(match.group(1))
            if key:
                line_no = text.count("\n", 0, match.start()) + 1
                seams.append((path.relative_to(ROOT), line_no, key))

    if not seams:
        print("check-source-citations: no todo!() seams found in src/", file=sys.stderr)
        return 1

    for list_path in SEAM_LISTS:
        full = ROOT / list_path
        if not full.exists():
            problems.append(f"{list_path}: seam list is missing entirely")
            continue
        quoted = {s.strip() for s in SPAN.findall(full.read_text(encoding="utf-8"))}
        for src_path, line_no, key in seams:
            if not any(key.startswith(span) for span in quoted if span):
                problems.append(
                    f"{list_path}: no row quotes the seam at {src_path} "
                    f"(line {line_no} today): {key[:60]!r}"
                )

    for path in markdown_files():
        text = path.read_text(encoding="utf-8")
        for match in LINE_CITATION.finditer(text):
            line_no = text.count("\n", 0, match.start()) + 1
            problems.append(
                f"{path.relative_to(ROOT)}:{line_no}: cites a source line number "
                f"({match.group(0)}). Quote the `todo!()` message, a function "
                f"name or a heading — a line number is wrong by the next commit"
            )

    if problems:
        print(
            f"check-source-citations: {len(problems)} problem(s)", file=sys.stderr
        )
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(
        f"ok: {len(seams)} seams, each named by both lists; "
        f"no document cites a source line number"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
