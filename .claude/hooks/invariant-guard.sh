#!/usr/bin/env bash
#
# PostToolUse advisory guard.
#
# The three architectural invariants in CLAUDE.md are checked by CI, which means
# they are found minutes after the edit that broke them. Two of them are greps;
# running those greps at edit time costs nothing and closes the loop while the
# change is still in hand.
#
# Advisory by design: exit 2 feeds the message back to Claude without undoing
# the edit, because a half-finished refactor legitimately trips these checks.
#
# It is a convenience, never a boundary. PostToolUse fires for the file-editing
# tools and for nothing else, so an edit made through the shell — `sed -i`, a
# heredoc, a Python one-liner — never reaches this script. `just ci` is the
# authority on all three invariants and this hook only shortens the loop for the
# common case.

set -uo pipefail

root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
[[ -d "$root" ]] || exit 0
cd "$root" || exit 0

payload="$(cat)"

# A guard that cannot read its input is a guard that is not running, and the
# silent version of that is the exact failure the gate exists to prevent: an
# unrun check is worse than a red one, because nothing says it did not run.
if ! command -v jq >/dev/null 2>&1; then
    {
        echo "invariant-guard: jq is not installed, so this hook checked nothing."
        echo "Both invariants are still gates — run 'just device-check' and"
        echo "'just format-version-check' by hand, or install jq."
    } >&2
    exit 2
fi

if ! file="$(jq -r '.tool_input.file_path // .tool_input.filePath // empty' <<<"$payload")"; then
    echo "invariant-guard: the hook payload did not parse; nothing was checked." >&2
    exit 2
fi

[[ -n "$file" ]] || exit 0

# Only Rust sources under the crate can break these.
case "$file" in
    *.rs) ;;
    *) exit 0 ;;
esac

relative="${file#"$root"/}"
case "$relative" in
    src/*|tests/*|benches/*) ;;
    *) exit 0 ;;
esac

findings=""

# §19.6.5 property 3 — Device is constructed in exactly one function.
if ! device_out="$(just device-check 2>&1)"; then
    findings+="${device_out}"$'\n'
fi

# §20.8 — every insert path names format_version.
if ! format_out="$(just format-version-check 2>&1)"; then
    findings+="${format_out}"$'\n'
fi

if [[ -n "$findings" ]]; then
    {
        echo "invariant-guard: an architectural invariant is currently violated."
        echo "These are CI gates (just device-check, just format-version-check)."
        echo "Fix before finishing; ignore only if the tree is mid-refactor."
        echo
        printf '%s' "$findings"
    } >&2
    exit 2
fi

exit 0
