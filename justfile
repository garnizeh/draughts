# draughts — task automation
#
# The same recipes run locally and in CI. That is the point: `just ci` is
# exactly what .github/workflows/ci.yml invokes, so a green machine and a green
# pipeline cannot mean two different things.
#
#     just              list every recipe
#     just ci           the full merge gate, as CI runs it
#     just test         the suite
#     just run          start the server against draughts.toml

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := false

# Ampere GA10x — the RTX 3050 in §2.4. Override for a different card.
export CUDA_COMPUTE_CAP := env_var_or_default("CUDA_COMPUTE_CAP", "86")

config := env_var_or_default("DRAUGHTS_CONFIG", "draughts.toml")

# List the available recipes.
default:
    @just --list --unsorted

# ---------------------------------------------------------------------------
# The gate
# ---------------------------------------------------------------------------

# Everything CI checks, in the order CI checks it. Run before opening a PR.
ci: fmt-check lint test device-check format-version-check docs
    @echo "ci: green"

# The half of §20.10 that runs everywhere, including a runner with no GPU:
# `cargo build` with no features must produce a binary with no CUDA dependency.

# Device-parity checks that need no GPU.
device-parity: device-check
    cargo build --locked
    cargo test --locked --lib face::

# ---------------------------------------------------------------------------
# Formatting and lints
# ---------------------------------------------------------------------------

# Format the tree.
fmt:
    cargo fmt --all

# Fail if the tree is not formatted.
fmt-check:
    cargo fmt --all -- --check

# Clippy over every target, warnings denied.
lint:
    cargo clippy --locked --all-targets --all-features -- -D warnings

# Type-check without producing a binary.
check:
    cargo check --locked --all-targets

# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

# The suite.
test *ARGS:
    cargo test --locked --all-targets {{ARGS}}

# One test or module by name.
test-one NAME:
    cargo test --locked {{NAME}} -- --nocapture

# §5.4: prove the transposition table is not load-bearing for correctness by
# running the search suite with it disabled.

# The search suite with the transposition table off. Nightly in CI.
test-tt-off:
    DRAUGHTS_TT_DISABLED=1 cargo test --locked --all-targets

# §20.4: load and volume tests. Minutes, not seconds — excluded from `just ci`.
test-load:
    cargo test --locked --release --test load -- --ignored --nocapture

# Doc examples.
test-docs:
    cargo test --locked --doc

# ---------------------------------------------------------------------------
# Static checks that no compiler performs
# ---------------------------------------------------------------------------

# §19.6.5: `candle_core::Device` is constructed in exactly one function.
device-check:
    ./scripts/check-device-construction.sh

# §20.8: every insert path sets format_version explicitly.
format-version-check:
    ./scripts/check-format-version.sh

# Documentation builds without a broken intra-doc link.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --document-private-items

# Known advisories in the dependency graph. Needs `cargo-deny`; see `just setup`.
audit:
    cargo deny check advisories bans licenses sources

# ---------------------------------------------------------------------------
# Builds
# ---------------------------------------------------------------------------

# Debug build.
build:
    cargo build --locked

# Portable release build: no CUDA dependency, runs on a machine with no driver.
build-release:
    cargo build --locked --release

# Target-host release build: adds the CUDA path. Requires a CUDA toolkit.
build-cuda:
    cargo build --locked --release --features cuda

# The compile-only CI job in §20.10, so the feature-gated path cannot rot.

# Compile the CUDA path without a device present.
check-cuda:
    cargo check --locked --all-targets --features cuda

# ---------------------------------------------------------------------------
# Running
# ---------------------------------------------------------------------------

# Start the server against the configured file (default draughts.toml).
run: _ensure-config
    cargo run --locked -- --config {{config}}

# Start the release build against the configured file.
run-release: _ensure-config build-release
    ./target/release/draughts --config {{config}}

# No database is opened, no table allocated, no port bound.

# §23.1: validate the configuration against this host and exit.
check-config: _ensure-config
    cargo run --locked -- --config {{config}} --check-config

# Validate the committed example rather than the local file.
check-config-example:
    cargo run --locked -- --config draughts.example.toml --check-config

_ensure-config:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ ! -f "{{config}}" ]]; then
        echo "{{config}} not found; copying draughts.example.toml"
        cp draughts.example.toml "{{config}}"
    fi

# ---------------------------------------------------------------------------
# Benchmarks and baselines
# ---------------------------------------------------------------------------

# §20.9: performance baselines. Tracked as committed numbers, not pass/fail.
bench *ARGS:
    cargo bench --locked {{ARGS}}

# ---------------------------------------------------------------------------
# Housekeeping
# ---------------------------------------------------------------------------

# Install the toolchain components and CLI tools these recipes assume.
setup:
    rustup component add rustfmt clippy
    cargo install --locked cargo-deny || true

# §22.1 promises a deployment that needs no internet. Fetch htmx and Alpine into
# static/vendor/ rather than loading them from a CDN at page load.

# Download the vendored frontend libraries.
setup-frontend:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p static/vendor
    curl -fsSLo static/vendor/htmx.min.js   https://unpkg.com/htmx.org@2/dist/htmx.min.js
    curl -fsSLo static/vendor/alpine.min.js https://unpkg.com/alpinejs@3/dist/cdn.min.js
    echo "vendored: $(ls -1 static/vendor/*.js | wc -l) files"

# Remove build artefacts.
clean:
    cargo clean

# Remove the local database and its WAL. Does not touch models or config.
clean-data:
    rm -rf data/

# Update the lockfile within the declared semver ranges.
update:
    cargo update

# What this tree would build against.
versions:
    @rustc --version
    @cargo --version
    @just --version
