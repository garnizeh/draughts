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

# The same grammar `just release-check` enforces. It is repeated here rather
# than shared because the two run in different languages, and the cost of them
# disagreeing is asymmetric: there a bad version fails a release, here it
# becomes a *filename*. `## [../../CLAUDE]` is a legal heading and would put an
# archive page in the repository root.
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$")
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


def duplicate_problem(released: list[Section]) -> str | None:
    """One section per version, because the rotation matches on version strings.

    `retired` is a set of versions, so a version appearing both inside and
    outside the keep window would filter the kept copy out along with the
    archived one — and the archive page for the second would overwrite the
    first. Either way a release's notes are gone. Refuse instead.
    """
    seen: set[str] = set()
    for section in released:
        if section.version in seen:
            return f"[{section.version}] appears more than once"
        seen.add(section.version)
    return None


def version_key(version: str) -> tuple[int, ...]:
    core = version.split("-", 1)[0]
    try:
        return tuple(int(part) for part in core.split("."))
    except ValueError:
        return (0,)


def scan_archive_pages() -> list[tuple[str, str | None]]:
    pages = []
    for page in ARCHIVE.glob("*.md"):
        if page.name == "README.md":
            continue
        match = re.search(r"^Released (\d{4}-\d{2}-\d{2})\.", page.read_text(encoding="utf-8"), re.M)
        pages.append((page.stem, match.group(1) if match else None))
    return pages


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

    malformed = [s.version for s in released if not SEMVER.match(s.version)]
    if malformed:
        print(
            "changelog: not a version: " + ", ".join(malformed),
            file=sys.stderr,
        )
        return 1

    for problem in (ordering_problem(released), duplicate_problem(released)):
        if problem is not None:
            print(f"changelog: {problem}", file=sys.stderr)
            return 1

    # `[Unreleased]` is where the next change goes. Without one, the next change
    # has nowhere to land and ends up appended to a released section — which is
    # editing notes that have already been published under a tag. It must exist,
    # and it must be first.
    if not sections or sections[0].released:
        print(
            "changelog: CHANGELOG.md must open with an '## [Unreleased]' section",
            file=sys.stderr,
        )
        return 1
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
            # A run interrupted between CHANGELOG.write_text() and write_index()
            # leaves `over <= 0` on retry with a stale or missing archive index.
            # Rebuild it whenever the archive directory exists, so a retry
            # finishes what the interrupted run started.
            if ARCHIVE.is_dir():
                write_index(scan_archive_pages())
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

    # Checking the page for a symlink is not enough: if `docs/` or
    # `docs/changelog/` is itself a link to somewhere else, a fresh
    # `<version>.md` inside it is not a symlink and `write_text` lands outside
    # the repository entirely. Resolve the directory and require it to stay
    # under ROOT, before it is created and before anything is written into it.
    if not ARCHIVE.resolve().is_relative_to(ROOT.resolve()):
        print(
            f"changelog-rotate: {ARCHIVE} resolves outside the repository "
            f"({ARCHIVE.resolve()}) — refusing to write",
            file=sys.stderr,
        )
        return 1

    ARCHIVE.mkdir(parents=True, exist_ok=True)

    # Sections appear newest first, so the tail is what ages out.
    retiring = released[KEEP:]

    # An archived page is immutable: its GitHub release notes were rendered from
    # it at publish time. Check every target before writing any of them, so a
    # collision halfway through cannot leave the archive half-rotated.
    #
    # A page whose content is byte-identical to what this run would write is not
    # a collision — it is this run, interrupted after writing that page and
    # before rewriting CHANGELOG.md. Treating it as fatal is what made the first
    # version of this guard unrecoverable: the retry saw its own output and
    # refused, and the only way out was to delete a file by hand. Identical
    # content is idempotent by definition, so resuming preserves immutability
    # exactly — nothing already archived is ever *changed*.
    #
    # A symlink is refused whatever it points at, dangling included: `exists()`
    # follows the link, so a dangling one reads as absent and would be written
    # through to wherever it aims.
    pages = {section.version: (ARCHIVE / f"{section.version}.md") for section in retiring}
    collisions = []
    for section in retiring:
        page = pages[section.version]
        if page.is_symlink():
            collisions.append(f"{section.version} (symlink)")
        elif page.exists() and page.read_text(encoding="utf-8") != archive_page(section):
            collisions.append(section.version)
    if collisions:
        print(
            "changelog-rotate: refusing to overwrite archived release notes for "
            + ", ".join(collisions),
            file=sys.stderr,
        )
        return 1

    for section in retiring:
        page = pages[section.version]
        rendered = archive_page(section)
        if page.exists() and page.read_text(encoding="utf-8") == rendered:
            print(f"already archived {section.version} -> {page.relative_to(ROOT)}")
            continue
        page.write_text(rendered, encoding="utf-8")
        print(f"archived {section.version} -> {page.relative_to(ROOT)}")

    retired = {section.version for section in retiring}
    kept = [section for section in sections if section.version not in retired]
    references = [
        line for line in references if REFERENCE.match(line)["version"] not in retired
    ]

    CHANGELOG.write_text(render(preamble, kept, references), encoding="utf-8")

    write_index(scan_archive_pages())

    print(f"changelog-rotate: CHANGELOG.md now holds {KEEP} released section(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
