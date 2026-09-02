#!/usr/bin/env bash
#
# §19.6.5 property 1, §20.10: `candle_core::Device` is constructed in exactly
# one function — `select_device`, in src/face/device.rs.
#
# A second `Device::Cpu` or `Device::new_cuda` anywhere else in the tree is a
# review-blocking defect, because it is what turns the next device change from a
# one-line edit into a search-and-replace.
#
# §20.10 says a grep in CI is sufficient and honest for this. It is this grep.

set -euo pipefail

readonly ALLOWED_FILE="src/face/device.rs"
# Every spelling that constructs a `candle_core::Device`: the two unit-like
# forms this file actually uses (`Device::Cpu`, `Device::new_cuda`), the
# public `Cuda`/`Metal` tuple variants (constructible directly, bypassing any
# `new_*` function), and the other constructor methods on the type
# (`cuda_if_available`, `new_cuda_with_stream`, `new_metal`). A pattern that
# only matched today's call sites would stop enforcing the rule the moment a
# later change reached for one of these instead.
readonly PATTERN='\bDevice::(Cpu|Cuda|Metal|new_cuda_with_stream|new_cuda|new_metal|cuda_if_available)\b'

offenders="$(
    grep -rEn --include='*.rs' "$PATTERN" src tests benches 2>/dev/null \
        | grep -v "^${ALLOWED_FILE}:" \
        || true
)"

# `use candle_core::Device as D;` (bare) and `use candle_core::{..., Device as
# D, ...};` (grouped, possibly spread across several lines) both let a caller
# build `D::Cpu` without the literal string `Device::` ever appearing —
# invisible to PATTERN above. Flag any aliased import of the type outside the
# allowed file, whatever the alias is later used for — cheaper than trying to
# track what the alias then constructs. Each file is flattened to one line
# first (`tr '\n' ' '`) so a brace list broken across lines can't hide the
# alias from a single-line regex, then matched up to the statement's `;`,
# which covers the bare and grouped forms with one pattern.
alias_offenders=""
while IFS= read -r -d '' file; do
    [[ "$file" == "$ALLOWED_FILE" ]] && continue
    if tr '\n' ' ' <"$file" | grep -qE '\buse\s+candle_core::[^;]*\bDevice\s+as\s+\w+\b'; then
        alias_offenders+="${file}: aliased Device import ('use ... as <name>')"$'\n'
    fi
done < <(find src tests benches -name '*.rs' -print0 2>/dev/null)

if [[ -n "$offenders" || -n "$alias_offenders" ]]; then
    echo "error: candle_core::Device is constructed outside ${ALLOWED_FILE}." >&2
    echo "       See §19.6.5 property 1 — the device must be resolved once," >&2
    echo "       in select_device(), and passed as a parameter thereafter." >&2
    echo >&2
    echo "$offenders" >&2
    echo "$alias_offenders" >&2
    exit 1
fi

# The allowed file must actually contain the construction. A check that passes
# because the function was deleted or renamed is not a check.
if ! grep -qE "$PATTERN" "$ALLOWED_FILE"; then
    echo "error: no Device construction found in ${ALLOWED_FILE}." >&2
    echo "       Either select_device() moved, or this check has gone stale." >&2
    exit 1
fi

echo "ok: Device is constructed only in ${ALLOWED_FILE}"
