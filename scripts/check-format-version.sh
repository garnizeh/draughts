#!/usr/bin/env bash
#
# §20.8: "A static check over the insert statements asserts every write path
# sets format_version explicitly from CURRENT_FORMAT_VERSION."
#
# A DEFAULT in the schema is what makes a version column stop meaning anything:
# it lets a write path that has never heard of versioning produce rows claiming
# to be version 1. The column keeps its default for the benefit of the v1.0
# migration (§12.1) and for nothing else.

set -euo pipefail

status=0

# Every INSERT into a table carrying a format_version must name that column.
for table in games positions; do
    while IFS= read -r hit; do
        [[ -z "$hit" ]] && continue

        file="${hit%%:*}"
        line="${hit#*:}"
        line="${line%%:*}"

        # Read the statement: from the INSERT to the first closing paren of the
        # column list, which is enough to see whether the column is named.
        statement="$(sed -n "${line},$((line + 12))p" "$file")"

        if ! grep -qi 'format_version' <<<"$statement"; then
            echo "error: ${file}:${line} inserts into ${table} without naming format_version." >&2
            echo "       See §13.7 — the column's DEFAULT exists for the v1.0" >&2
            echo "       migration, not for new write paths." >&2
            status=1
        fi
    done < <(grep -rEni --include='*.rs' "INSERT +INTO +${table}\b" src 2>/dev/null || true)
done

# Every decode of a versioned BLOB must have dispatched on the version. The
# check is deliberately coarse: it asserts that the constant is referenced in
# the module that decodes, which is a floor, not a proof.
if grep -rqE 'fn decode_' --include='*.rs' src/db; then
    if ! grep -rq 'CURRENT_FORMAT_VERSION' --include='*.rs' src/db; then
        echo "error: src/db decodes BLOBs without referencing CURRENT_FORMAT_VERSION." >&2
        status=1
    fi
fi

if [[ $status -eq 0 ]]; then
    echo "ok: every insert path names format_version"
fi

exit $status
