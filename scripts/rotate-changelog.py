#!/usr/bin/env python3
"""Keep CHANGELOG.md short by archiving old releases.

A changelog nobody can read is a changelog nobody reads. `CHANGELOG.md` holds
`[Unreleased]` and the five most recent releases; everything older moves to
`docs/changelog/<version>.md`, one file per release, and the index there lists
them.

One release per archive file rather than five, deliberately: a file's name never
changes once written, so a link to it never rots and `git log` follows it. The
logrotate habit of renaming `.1` to `.2` would rewrite every archive on every
release and break both.

    rotate-changelog.py --check     exit 1 if CHANGELOG.md is over the limit
    rotate-changelog.py             archive whatever is over it

Run by `just changelog-check` (in the merge gate) and `just changelog-rotate`.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# Five is not a derived number; it is "enough that the recent past is on one
# screen". Change it here and the gate follows.
KEEP = 5

ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = ROOT / "CHANGELOG.md"
ARCHIVE = ROOT / "docs" / "changelog"
REPO = "https://github.com/garnizeh/draughts"

HEADING = re.compile(r"^## \[(?P<version>[^\]]+)\](?: - (?P<date>\d{4}-\d{2}-\d{2}))?\s*$")
REFERENCE = re.compile(r"^\[(?P<version>[^\]]+)\]:\s*\S+\s*$")


class Section:
    def __init__(self, version: str, date: str | None, body: list[str]) -> None:
        self.version = version
        self.date = date
        self.body = body

    @property
    def released(self) -> bool:
        return self.version.lower() != "unreleased"

    def text(self) -> str:
        heading = f"## [{self.version}]"
        if self.date:
            heading += f" - {self.date}"
        return "\n".join([heading, *self.body]).rstrip() + "\n"


def parse(text: str) -> tuple[list[str], list[Section], list[str]]:
    """Split the file into (preamble, sections, reference definitions)."""
    lines = text.splitlines()

    preamble: list[str] = []
    sections: list[Section] = []
    current: Section | None = None

    for line in lines:
        match = HEADING.match(line)
        if match:
            current = Section(match["version"], match["date"], [])
            sections.append(current)
            continue
        if current is None:
            preamble.append(line)
        else:
            current.body.append(line)

    # The reference definitions live at the end of the last section's body.
    # They belong to the file, not to that release.
    references: list[str] = []
    if sections:
        body = sections[-1].body
        while body and (REFERENCE.match(body[-1]) or not body[-1].strip()):
            if REFERENCE.match(body[-1]):
                references.insert(0, body[-1])
            body.pop()
    else:
        while preamble and (REFERENCE.match(preamble[-1]) or not preamble[-1].strip()):
            if REFERENCE.match(preamble[-1]):
                references.insert(0, preamble[-1])
            preamble.pop()

    return preamble, sections, references


def render(preamble: list[str], sections: list[Section], references: list[str]) -> str:
    parts = ["\n".join(preamble).rstrip() + "\n"]
    parts.extend(section.text() for section in sections)
    body = "\n".join(parts)
    if references:
        body += "\n" + "\n".join(references) + "\n"
    return body


def archive_page(section: Section) -> str:
    released = f"Released {section.date}." if section.date else "Date unrecorded."
    return (
        f"# draughts {section.version}\n"
        "\n"
        f"{released} Archived from [CHANGELOG.md](../../CHANGELOG.md), which keeps\n"
        f"the {KEEP} most recent releases and nothing else. The index is\n"
        "[here](README.md).\n"
        "\n"
        + "\n".join(section.body).strip()
        + "\n"
        "\n"
        f"[{section.version}]: {REPO}/releases/tag/v{section.version}\n"
    )


def ordering_problem(released: list[Section]) -> str | None:
    """Newest first, and say so out loud rather than assuming it.

    Everything downstream depends on the order: the tail of the list is what
    ages out, and a release appended at the bottom would archive the newest
    entry and publish the oldest one's notes. Keep a Changelog puts the newest
    first; this is what makes that a check rather than a habit.
    """
    for older, newer in zip(released[1:], released):
        newer_rank = (version_key(newer.version), newer.date or "")
        older_rank = (version_key(older.version), older.date or "")
        if newer_rank < older_rank:
            return (
                f"[{newer.version}] appears above [{older.version}] but is older. "
                "CHANGELOG.md is newest-first"
            )
    return None


def version_key(version: str) -> tuple[int, ...]:
    core = version.split("-", 1)[0]
    try:
        return tuple(int(part) for part in core.split("."))
    except ValueError:
        return (0,)


def write_index(pages: list[tuple[str, str | None]]) -> None:
    pages = sorted(pages, key=lambda page: version_key(page[0]), reverse=True)
    rows = "\n".join(
        f"| [{version}]({version}.md) | {date or '—'} |" for version, date in pages
    )
    (ARCHIVE / "README.md").write_text(
        "# Changelog archive\n"
        "\n"
        f"[CHANGELOG.md](../../CHANGELOG.md) keeps `[Unreleased]` and the {KEEP} most\n"
        "recent releases. Everything older lives here, one file per release, and a\n"
        "file's name never changes once it is written — a link to it does not rot and\n"
        "`git log` follows it.\n"
        "\n"
        "Written by `just changelog-rotate`; `just changelog-check` is what notices\n"
        "the main file has grown past the limit. Do not edit an archived section: the\n"
        "notes on its GitHub release were rendered from it at publish time, and\n"
        "editing one makes the two disagree with no way to tell which is right.\n"
        "\n"
        + (
            "| Version | Released |\n|---|---|\n" + rows + "\n"
            if rows
            else f"Nothing archived yet — the first {KEEP} releases still live in\n"
            "[CHANGELOG.md](../../CHANGELOG.md).\n"
        ),
        encoding="utf-8",
    )


def main() -> int:
    check_only = "--check" in sys.argv[1:]

    text = CHANGELOG.read_text(encoding="utf-8")
    preamble, sections, references = parse(text)

    released = [section for section in sections if section.released]

    problem = ordering_problem(released)
    if problem is not None:
        print(f"changelog: {problem}", file=sys.stderr)
        return 1

    # `[Unreleased]` is where the next change goes, so it belongs at the top or
    # it belongs nowhere.
    if any(not section.released for section in sections[1:]):
        print("changelog: [Unreleased] must be the first section", file=sys.stderr)
        return 1

    over = len(released) - KEEP

    if over <= 0:
        if check_only:
            print(
                f"changelog-check: {len(released)} released section(s), newest first, "
                f"limit {KEEP}"
            )
        else:
            print(f"changelog-rotate: nothing to archive ({len(released)}/{KEEP})")
        return 0

    if check_only:
        names = ", ".join(section.version for section in released[KEEP:])
        print(
            f"changelog-check: CHANGELOG.md holds {len(released)} released sections, "
            f"limit is {KEEP}. Run 'just changelog-rotate' to archive: {names}",
            file=sys.stderr,
        )
        return 1

    ARCHIVE.mkdir(parents=True, exist_ok=True)

    # Sections appear newest first, so the tail is what ages out.
    retiring = released[KEEP:]
    for section in retiring:
        page = ARCHIVE / f"{section.version}.md"
        page.write_text(archive_page(section), encoding="utf-8")
        print(f"archived {section.version} -> {page.relative_to(ROOT)}")

    retired = {section.version for section in retiring}
    kept = [section for section in sections if section.version not in retired]
    references = [
        line for line in references if REFERENCE.match(line)["version"] not in retired
    ]

    CHANGELOG.write_text(render(preamble, kept, references), encoding="utf-8")

    pages = []
    for page in ARCHIVE.glob("*.md"):
        if page.name == "README.md":
            continue
        match = re.search(r"^Released (\d{4}-\d{2}-\d{2})\.", page.read_text(encoding="utf-8"), re.M)
        pages.append((page.stem, match.group(1) if match else None))
    write_index(pages)

    print(f"changelog-rotate: CHANGELOG.md now holds {KEEP} released section(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
