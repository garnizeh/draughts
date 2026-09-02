#!/usr/bin/env bash
#
# §19.6.5 property 3, §20.10 and §22.1: the `cuda` feature adds a device, never
# a requirement. The default build must therefore link against nothing from the
# CUDA runtime, so that the binary the deployment model promises — "runs on a
# machine with no driver" — actually does.
#
# The failure this catches is invisible at link time on a CI runner that happens
# to have the toolkit installed: the binary links, ships, and then fails to load
# on the target host with a missing shared object. Reading NEEDED is the cheap
# way to ask the question on any host, with or without a GPU.
#
# Usage: check-no-cuda-linkage.sh [BINARY]   (default: target/release/draughts)

set -euo pipefail

binary="${1:-target/release/draughts}"

if [[ ! -f "$binary" ]]; then
    echo "check-no-cuda-linkage: no such binary: $binary" >&2
    exit 1
fi

# binutils is a build dependency of Rust itself, so this is present on any host
# that can produce the binary being checked. Refuse rather than skip: a check
# that silently passes when it could not run is worse than no check.
if command -v objdump >/dev/null 2>&1; then
    needed="$(objdump -p "$binary" | grep -E '^\s*NEEDED' || true)"
elif command -v readelf >/dev/null 2>&1; then
    needed="$(readelf -d "$binary" | grep -E 'NEEDED' || true)"
else
    echo "check-no-cuda-linkage: neither objdump nor readelf is installed; cannot verify $binary" >&2
    exit 1
fi

if printf '%s\n' "$needed" | grep -iE '(libcuda|libcudart|libcublas|libcurand|libnvrtc|libnvidia)'; then
    echo >&2
    echo "check-no-cuda-linkage: $binary has a CUDA dependency in NEEDED." >&2
    echo "The default build must run on a machine with no driver (§22.1)." >&2
    exit 1
fi

echo "check-no-cuda-linkage: $binary needs no CUDA library"
