#!/usr/bin/env python3
"""Every relative link and every § anchor in the documentation resolves.

This tree is unusually citation-dense on purpose: a constant with no § is the
thing most likely to be "cleaned up" into a bug, so the documents point at each
other constantly — 62 markdown files, several hundred cross-references. That
density is only worth anything while the references are true, and nothing about
renaming a heading tells you which twenty links you just broke.

Checks two things, both silent failures otherwise:

  * a relative link whose target file does not exist
  * a `#anchor` with no matching heading in the file it points into

External links are not checked: a network call does not belong in the gate, and
a 404 on someone else's site is not this repository's build failing.

Run by `just doc-links`, which is part of `just ci`.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SKIP_DIRS = {"target", ".git", "node_modules"}

# Markdown link, excluding images. The optional trailing "title" is rare here
# but legal, and swallowing it keeps the target clean.
LINK = re.compile(r'(?<!\!)\[([^\]]*)\]\(([^)\s]+)(?:\s+"[^"]*")?\)')

HEADING = re.compile(r"^#{1,6}\s+(.*)$")


def slug(text: str) -> str:
    """GitHub's heading-to-anchor rule, as far as this tree exercises it.

    Two details cost an afternoon each if guessed. Underscores survive — the
    anchor for `format_version` keeps it — and *each* space becomes a hyphen
    rather than each run of them, which is why an em-dash heading like
    "7.8 Circuit Breaker — New in 1.1" ends up with the double hyphen in
    `#78-circuit-breaker--new-in-11`: the dash is dropped and the two spaces
    around it are not.
    """
    t = text.strip().lower()
    t = re.sub(r"[`*\[\]()]", "", t)
    t = re.sub(r"[^\w\s-]", "", t, flags=re.UNICODE)
    return t.replace(" ", "-")


def markdown_files() -> list[Path]:
    return sorted(
        p
        for p in ROOT.rglob("*.md")
        if not SKIP_DIRS.intersection(p.relative_to(ROOT).parts)
    )


def main() -> int:
    files = markdown_files()
    anchors: dict[Path, set[str]] = {}
    for path in files:
        found = set()
        for line in path.read_text(encoding="utf-8").splitlines():
            match = HEADING.match(line)
            if match:
                found.add(slug(match.group(1)))
        anchors[path] = found

    problems: list[str] = []
    for path in files:
        text = path.read_text(encoding="utf-8")
        here = path.relative_to(ROOT)
        for match in LINK.finditer(text):
            target = match.group(2)
            if target.startswith(("http://", "https://", "mailto:", "#L")):
                continue
            line_no = text.count("\n", 0, match.start()) + 1
            path_part, _, anchor = target.partition("#")

            if path_part:
                dest = (path.parent / path_part).resolve()
                if not dest.exists():
                    problems.append(f"{here}:{line_no}: no such file: {target}")
                    continue
            else:
                dest = path

            # A link into source cites a line, not a heading; nothing to check.
            if anchor.startswith("L") and anchor[1:].isdigit():
                continue
            if anchor and dest.suffix == ".md" and anchor not in anchors.get(dest, set()):
                problems.append(f"{here}:{line_no}: no such anchor: {target}")

    if problems:
        print(f"check-doc-links: {len(problems)} broken reference(s)", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(f"ok: every relative link and anchor resolves, across {len(files)} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
