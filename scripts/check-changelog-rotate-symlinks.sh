#!/usr/bin/env bash
#
# rotate-changelog.py writes into docs/changelog/ from two places: the
# ordinary rotation path and the over<=0 recovery branch that rebuilds a
# stale index after an interrupted run. Both must refuse a symlinked
# docs/changelog/ directory and a symlinked docs/changelog/README.md — a
# guard added to one write path and not the other is exactly the defect
# PR #99 found twice (LESSONS.md, "guard the whole path, not the leaf",
# missed x2). This reconstructs both escapes against both call paths and
# asserts the script refuses every one of them.

set -euo pipefail

readonly ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "check-changelog-rotate-symlinks: $1" >&2
    exit 1
}

# Five released sections plus [Unreleased]: over<=0, so a run here always
# takes the recovery branch rather than the ordinary rotation path.
write_changelog_at_limit() {
    cat >"$1/CHANGELOG.md" <<'EOF'
# Changelog

## [Unreleased]

## [1.0.5] - 2026-01-05
five

## [1.0.4] - 2026-01-04
four

## [1.0.3] - 2026-01-03
three

## [1.0.2] - 2026-01-02
two

## [1.0.1] - 2026-01-01
one
EOF
}

# Six released sections: over>0, so a run here takes the ordinary rotation
# path instead of the recovery branch.
write_changelog_over_limit() {
    cat >"$1/CHANGELOG.md" <<'EOF'
# Changelog

## [Unreleased]

## [1.0.6] - 2026-01-06
six

## [1.0.5] - 2026-01-05
five

## [1.0.4] - 2026-01-04
four

## [1.0.3] - 2026-01-03
three

## [1.0.2] - 2026-01-02
two

## [1.0.1] - 2026-01-01
one
EOF
}

new_sandbox() {
    local dir="$TMP/$1"
    mkdir -p "$dir/scripts" "$dir/docs"
    cp "$ROOT/scripts/rotate-changelog.py" "$dir/scripts/rotate-changelog.py"
    echo "$dir"
}

# docs/changelog/ is a symlink pointing outside the repository, on the
# recovery branch (over<=0).
dir1="$(new_sandbox dir-symlink-recovery)"
write_changelog_at_limit "$dir1"
outside="$TMP/outside-1"
mkdir -p "$outside"
printf '# 1.0.0\n\nReleased 2025-12-01.\n\nzero\n' >"$outside/1.0.0.md"
ln -s "$outside" "$dir1/docs/changelog"
if (cd "$dir1" && python3 scripts/rotate-changelog.py) >/dev/null 2>&1; then
    fail "a symlinked docs/changelog/ was not refused on the recovery branch"
fi

# docs/changelog/ is a symlink pointing outside the repository, on the
# ordinary rotation branch (over>0).
dir2="$(new_sandbox dir-symlink-rotation)"
write_changelog_over_limit "$dir2"
outside2="$TMP/outside-2"
mkdir -p "$outside2"
ln -s "$outside2" "$dir2/docs/changelog"
if (cd "$dir2" && python3 scripts/rotate-changelog.py) >/dev/null 2>&1; then
    fail "a symlinked docs/changelog/ was not refused on the rotation branch"
fi

# docs/changelog/README.md itself is a symlink, on the recovery branch.
dir3="$(new_sandbox leaf-symlink-recovery)"
write_changelog_at_limit "$dir3"
mkdir -p "$dir3/docs/changelog"
printf '# 1.0.0\n\nReleased 2025-12-01.\n\nzero\n' >"$dir3/docs/changelog/1.0.0.md"
evil="$TMP/evil.md"
echo "pwned" >"$evil"
ln -s "$evil" "$dir3/docs/changelog/README.md"
if (cd "$dir3" && python3 scripts/rotate-changelog.py) >/dev/null 2>&1; then
    fail "a symlinked archive README.md was not refused on the recovery branch"
fi
if grep -q "Changelog archive" "$evil"; then
    fail "the symlinked archive README.md target was written through"
fi

echo "ok: rotate-changelog.py refuses a symlinked archive directory and a symlinked index, on both write paths"
