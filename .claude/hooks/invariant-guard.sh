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

set -uo pipefail

root="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null)}"
[[ -d "$root" ]] || exit 0
cd "$root" || exit 0

payload="$(cat)"
file="$(jq -r '.tool_input.file_path // .tool_input.filePath // empty' <<<"$payload" 2>/dev/null)"
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
if ! device_out="$(./scripts/check-device-construction.sh 2>&1)"; then
    findings+="${device_out}"$'\n'
fi

# §20.8 — every insert path names format_version.
if ! format_out="$(./scripts/check-format-version.sh 2>&1)"; then
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
