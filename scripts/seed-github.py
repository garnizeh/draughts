#!/usr/bin/env python3
"""Create the GitHub milestones, labels and issues described by docs/ROADMAP.md.

The roadmap is the source of truth: this script parses it rather than carrying
its own copy of the plan, so a roadmap amendment and a `--sync` run cannot mean
two different things.  Issues already present (matched on the `Mn-k` prefix of
the title) are left alone, which makes the script safe to re-run after the
roadmap gains an issue.

    scripts/seed-github.py --dry-run     # print what would happen
    scripts/seed-github.py               # create it
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROADMAP = ROOT / "docs" / "ROADMAP.md"
REPO = "garnizeh/draughts"

# Label colours group by prefix so the issue list is readable at a glance.
LABEL_COLOURS = {
    "area": ("0e8a16", "The part of the tree this touches"),
    "type": ("1d76db", "The kind of work"),
    "gate": ("b60205", "Touches one of the five rules in CLAUDE.md — needs an architecture review"),
    "prio": ("5319e7", "How close to the MVP path this sits"),
}


# GitHub throttles content creation well below the documented core rate limit —
# roughly 80 creations a minute, and it answers with "rate limit already
# exceeded" rather than a Retry-After.  Seeding 94 issues therefore has to pace
# itself and back off when it is told to, or it stops two thirds of the way in.
CREATE_INTERVAL_S = 3.0
BACKOFF_S = 90.0


def gh(*args: str, check: bool = True, retries: int = 0) -> str:
    for attempt in range(retries + 1):
        r = subprocess.run(["gh", *args], capture_output=True, text=True)
        if r.returncode == 0:
            return r.stdout.strip()
        if attempt < retries and "rate limit" in r.stderr.lower():
            print(f"  rate limited; waiting {BACKOFF_S:.0f}s", flush=True)
            time.sleep(BACKOFF_S)
            continue
        if check:
            raise SystemExit(f"gh {' '.join(args)} failed:\n{r.stderr.strip()}")
        return r.stdout.strip()
    return ""


def parse_roadmap(text: str):
    """Return (milestones, labels) from the roadmap's milestone sections."""
    milestones = []
    # "## M1 — Rules Core" through to the next "## " heading.
    pattern = re.compile(r"^## (M\d) — (.+?)$(.*?)(?=^## |\Z)", re.M | re.S)
    for m in pattern.finditer(text):
        key, name, body = m.group(1), m.group(2).strip(), m.group(3)

        def field(label: str) -> str:
            f = re.search(rf"\*\*{label}:\*\* (.+?)(?:\n\*\*|\n\n)", body, re.S)
            return " ".join(f.group(1).split()) if f else ""

        issues = []
        for row in re.finditer(r"^\| (M\d-\d+) \| (.+?) \| (.+?) \| (.+?) \|$", body, re.M):
            ident, title, section, labels = (g.strip() for g in row.groups())
            if title.startswith("~~"):  # a dropped issue keeps its row, not an issue
                continue
            issues.append({
                "id": ident,
                "title": re.sub(r"\*\*|`", "", title),
                "section": section,
                "labels": re.findall(r"`([^`]+)`", labels),
            })

        exit_ = re.search(r"^\*\*Exit:\*\* (.+?)$", body, re.M | re.S)
        milestones.append({
            "key": key,
            "name": f"{key} — {name}",
            "goal": field("Goal"),
            "depends": field("Depends on"),
            "exit": " ".join(exit_.group(1).split()) if exit_ else "",
            "issues": issues,
        })

    labels = sorted({l for ms in milestones for i in ms["issues"] for l in i["labels"]})
    return milestones, labels


def issue_body(ms: dict, issue: dict) -> str:
    return "\n".join([
        issue["title"] + ".",
        "",
        f"**Owning section:** {issue['section']} — the architecture is the "
        "specification; this issue only names the work.",
        f"**Milestone:** {ms['name']} — {ms['goal']}",
        "",
        "Read the § before writing anything. When the code and the document "
        "disagree, the document is right.",
        "",
        "**Done means** the [definition of done]"
        "(https://github.com/garnizeh/draughts/blob/main/docs/ROADMAP.md#definition-of-done--applies-to-every-issue) "
        "in the roadmap: `just ci` green, the property-named tests §20 requires, "
        "any non-obvious constant citing its §, and CHANGELOG.md updated.",
        "",
        f"Tracked as `{issue['id']}` in [docs/ROADMAP.md]"
        "(https://github.com/garnizeh/draughts/blob/main/docs/ROADMAP.md).",
    ])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    milestones, labels = parse_roadmap(ROADMAP.read_text())
    total = sum(len(m["issues"]) for m in milestones)
    print(f"roadmap: {len(milestones)} milestones, {total} issues, {len(labels)} labels")
    if args.dry_run:
        for ms in milestones:
            print(f"\n{ms['name']} ({len(ms['issues'])} issues)")
            for i in ms["issues"]:
                print(f"  {i['id']:6} {i['title'][:70]:70} {' '.join(i['labels'])}")
        print("\nlabels: " + " ".join(labels))
        return 0

    # Labels.
    existing = {l["name"] for l in json.loads(gh("label", "list", "-R", REPO, "--limit", "200", "--json", "name"))}
    for label in labels:
        if label in existing:
            continue
        colour, desc = LABEL_COLOURS[label.split(":")[0]]
        gh("label", "create", label, "-R", REPO, "--color", colour, "--description", desc)
        print(f"label + {label}")

    # Milestones.
    have = {m["title"]: m["number"] for m in json.loads(gh("api", f"repos/{REPO}/milestones?state=all"))}
    numbers = {}
    for ms in milestones:
        if ms["name"] in have:
            numbers[ms["key"]] = have[ms["name"]]
            continue
        desc = f"{ms['goal']} Depends on: {ms['depends'] or 'nothing'} Exit: {ms['exit']}"
        out = gh("api", f"repos/{REPO}/milestones", "-f", f"title={ms['name']}", "-f", f"description={desc[:900]}")
        numbers[ms["key"]] = json.loads(out)["number"]
        print(f"milestone + {ms['name']}")

    # Issues.  Existing ones are matched on the Mn-k prefix so a re-run is a no-op.
    seen = set()
    for it in json.loads(gh("issue", "list", "-R", REPO, "--state", "all", "--limit", "500", "--json", "title")):
        m = re.match(r"(M\d-\d+)", it["title"])
        if m:
            seen.add(m.group(1))

    created = 0
    for ms in milestones:
        for issue in ms["issues"]:
            if issue["id"] in seen:
                print(f"issue = {issue['id']} (exists)")
                continue
            cmd = ["issue", "create", "-R", REPO,
                   "--title", f"{issue['id']} {issue['title']}",
                   "--body", issue_body(ms, issue),
                   "--milestone", ms["name"]]
            for label in issue["labels"]:
                cmd += ["--label", label]
            url = gh(*cmd, retries=5)
            created += 1
            print(f"issue + {issue['id']} {url.splitlines()[-1] if url else ''}", flush=True)
            time.sleep(CREATE_INTERVAL_S)

    print(f"\ndone: {created} issues created, {total - created} already present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
