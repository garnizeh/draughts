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

# Everything the `gate` job checks, in the order it checks it. See `pre-pr`.
ci: fmt-check lint test device-check format-version-check changelog-check doc-links docs
    @echo "ci: green"

# Every job CI runs, not only the `gate` one. This is the pre-PR check.
#
# `just ci` is one job of six. The other five have caught real breakage that
# `ci.yml`'s gate cannot see — a CUDA path that stopped compiling, a licence a
# dependency changed under us, a workflow expression that parses and means
# nothing. Finding those here costs a minute; finding them on a pushed branch
# costs a round trip and a red PR.
#
# Order matters, and it is not the order `ci.yml` lists the jobs in. `just` stops
# the prerequisite chain at the first failure, so the list is sorted by *what a
# failure would mean*, in three tiers:
#
#   1. `ci`, `portable-check`     — need only the pinned Rust toolchain. If one
#                                   of these fails, the tree is wrong.
#   2. `audit`, `coverage`,       — need a tool `just setup` installs
#      `workflows-check`            (cargo-deny, cargo-llvm-cov, actionlint). A
#                                   failure here may mean the host, not the tree.
#   3. `check-cuda`, `build-cuda` — need a CUDA toolkit on this host.
#
# Everything answerable on any machine is answered before anything that can fail
# for a reason unrelated to the change. Sorting the other way is how a missing
# actionlint costs you the coverage report.
#
# Two caveats, stated rather than hidden. `portable-check` builds outside a
# driverless container, so it is weaker than CI's version of the same job — CI
# stays the authority on §22.1. And a red `pre-pr` whose only failure is a
# missing tool is not a red tree: say which recipe could not run, run `just
# setup` if that is the fix, and let CI answer the rest.

# Every CI job, locally. Run this before opening a PR.
pre-pr: ci portable-check audit coverage workflows-check check-cuda build-cuda
    @echo "pre-pr: every CI job is green here"

# The `workflows` CI job: actionlint over .github/workflows. See `just setup`.
workflows-check:
    actionlint

# The `portable-build` CI job, minus the container: §22.1 and §19.6.5's third
# property, asserted rather than assumed.

# Build the default binary, run it, and prove it needs no CUDA library.
portable-check: build-release
    ./target/release/draughts --version
    ./target/release/draughts --config draughts.example.toml --check-config
    ./scripts/check-no-cuda-linkage.sh

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

# Clippy over every target, default features. `--all-features` would pull in
# `cuda` and its `cudarc`/nvcc dependency, breaking the CPU-only gate — the
# feature-gated path is linted separately by `check-cuda` (§20.10).
lint:
    cargo clippy --locked --all-targets -- -D warnings

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

# This tree cites itself constantly — several hundred cross-references across
# sixty-odd files — and that density is worth nothing once the references stop
# being true. Renaming one heading tells you nothing about the twenty links you
# just broke.

# Every relative link and § anchor in the documentation resolves.
doc-links:
    ./scripts/check-doc-links.py

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
# Lints too, not just compiles — `lint` deliberately excludes `--features cuda`
# (it would pull in cudarc/nvcc and break the CPU-only gate), so this is the
# only place the cuda-gated path gets `-D warnings` coverage.

# Compile and lint the CUDA path without a device present.
check-cuda:
    cargo check --locked --all-targets --features cuda
    cargo clippy --locked --all-targets --features cuda -- -D warnings

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
# Releasing
# ---------------------------------------------------------------------------
#
# The version in Cargo.toml is the source of truth and the CHANGELOG is the
# gate. `release.yml` pushes a tag for `just version` only once
# `just release-notes` finds a *closed* section for it — a dated
# `## [x.y.z] - YYYY-MM-DD` heading, not `[Unreleased]`. Bumping the version
# without writing that section is therefore not a release, which is the point:
# nothing ships whose notes nobody wrote.

# The crate version. Cargo.toml is the one place it is written down.
version:
    @sed -n '/^\[package\]/,/^\[[a-z]/ s/^version = "\(.*\)"/\1/p' Cargo.toml | head -1

# Print the CHANGELOG section for VERSION. Non-zero if it is not closed yet.
#
# "Closed" means dated. `release.yml` asks this recipe whether a commit is a
# release at all, and a `## [x.y.z]` with no date is a draft — matching it here
# would send an unfinished section down a path that `release-check` then fails,
# turning "not ready yet" into a red job on main. The version is matched
# literally and only the date is a pattern: a version is full of dots, and dots
# in a regex match anything.
release-notes VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    notes="$(awk -v want="## [{{VERSION}}] - " '
        index($0, want) == 1 && $0 ~ /- [0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]$/ {
            inside = 1
            next
        }
        inside && index($0, "## [") == 1 { exit }
        inside { print }
    ' CHANGELOG.md | sed -e '/./,$!d' -e :a -e '/^\n*$/{$d;N;ba' -e '}')"
    if [[ -z "$notes" ]]; then
        echo "CHANGELOG.md has no closed section for {{VERSION}}" >&2
        exit 1
    fi
    printf '%s\n' "$notes"

# Everything that must hold before VERSION is packaged or tagged.
release-check VERSION:
    #!/usr/bin/env bash
    set -euo pipefail
    want="{{VERSION}}"
    if [[ ! "$want" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
        echo "release-check: '$want' is not a semantic version" >&2
        exit 1
    fi
    have="$(just version)"
    if [[ "$have" != "$want" ]]; then
        echo "release-check: Cargo.toml says $have, release says $want" >&2
        exit 1
    fi
    # A stale lockfile means the tarball is built from a dependency graph
    # nobody recorded. `--locked` everywhere else makes this cheap to assert.
    locked="$(awk '/^name = "draughts"$/ { f = 1 } f && /^version = / { print; exit }' Cargo.lock)"
    if [[ "$locked" != "version = \"$want\"" ]]; then
        echo "release-check: Cargo.lock does not record draughts $want — run 'cargo update -p draughts'" >&2
        exit 1
    fi
    # Keep a Changelog: the heading carries a release date, and an undated
    # section is a draft rather than a release.
    if ! grep -qE "^## \[${want//./\\.}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" CHANGELOG.md; then
        echo "release-check: CHANGELOG.md has no dated '## [$want] - YYYY-MM-DD' heading" >&2
        exit 1
    fi
    just release-notes "$want" >/dev/null
    echo "release-check: $want is ready"

# CHANGELOG.md keeps `[Unreleased]` and the five most recent releases, newest
# first; everything older is archived under docs/changelog/, one file per
# release. In the gate, because a release PR is exactly when it is cheapest to
# fix and the only time it comes up.

# Newest first, and no more than five released sections in CHANGELOG.md.
changelog-check:
    ./scripts/rotate-changelog.py --check

# Archive whatever is over the limit into docs/changelog/.
changelog-rotate:
    ./scripts/rotate-changelog.py

# §22.1: two builds, and both must work. `portable` is the one the deployment
# model promises runs on a machine with no driver — its linkage is asserted,
# not assumed. `cuda` adds a device and therefore adds a host requirement, so
# it ships under its own name and says so in the tarball.

# Build and archive a release tarball into dist/. FLAVOUR is portable or cuda.
package VERSION FLAVOUR:
    #!/usr/bin/env bash
    set -euo pipefail
    just release-check "{{VERSION}}"
    case "{{FLAVOUR}}" in
        portable) just build-release; suffix="x86_64-unknown-linux-gnu" ;;
        cuda)     just build-cuda;    suffix="x86_64-unknown-linux-gnu-cuda" ;;
        *) echo "package: unknown flavour '{{FLAVOUR}}' (portable|cuda)" >&2; exit 1 ;;
    esac
    name="draughts-{{VERSION}}-${suffix}"
    stage="dist/${name}"
    rm -rf "$stage"
    mkdir -p "$stage"
    cp target/release/draughts "$stage/"
    cp draughts.example.toml README.md CHANGELOG.md LICENSE "$stage/"
    if [[ "{{FLAVOUR}}" == "portable" ]]; then
        ./scripts/check-no-cuda-linkage.sh "$stage/draughts"
    else
        cat > "$stage/CUDA.md" <<'NOTE'
    # This build carries the `cuda` feature

    It needs an NVIDIA driver and the CUDA 12.x runtime libraries on the host.
    The feature adds a *device*, never a requirement of the engine: this binary
    still starts, plays and comments with no GPU present, by falling back to the
    CPU profile (§7.4.1). But it will not load at all without the CUDA runtime
    it is linked against — that is a host requirement of the executable, not of
    the design.

    If the host has no driver, take the portable tarball instead. It is the same
    revision with the feature off, and §22.1 is written around it.
    NOTE
    fi
    tar -C dist -czf "dist/${name}.tar.gz" "$name"
    ( cd dist && sha256sum "${name}.tar.gz" > "${name}.tar.gz.sha256" )
    rm -rf "$stage"
    echo "packaged: dist/${name}.tar.gz"

# ---------------------------------------------------------------------------
# Coverage
# ---------------------------------------------------------------------------

# §20: reported, never gated — a percentage threshold against a tree full of
# `todo!()` seams would measure the seams.

# An lcov report and a summary table.
coverage:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --locked --all-targets --no-report
    cargo llvm-cov report --lcov --output-path lcov.info
    cargo llvm-cov report | tee coverage-summary.txt

# ---------------------------------------------------------------------------
# Housekeeping
# ---------------------------------------------------------------------------

# `|| true` on each install: a tool that is already present, or a network that
# is not, should not stop the rest from being set up. The recipe that needs a
# missing tool will say so by name when it runs.

# Install the toolchain components and CLI tools these recipes assume.
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    rustup component add rustfmt clippy llvm-tools-preview
    cargo install --locked cargo-deny || true
    cargo install --locked cargo-llvm-cov || true
    # actionlint has no cargo package. Go if it is here, a release binary if it
    # is not — v1.7.7 is what the CI action's pinned image carries.
    if command -v actionlint >/dev/null 2>&1; then
        actionlint --version | head -1
    elif command -v go >/dev/null 2>&1; then
        go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.7
    else
        echo "actionlint is missing and there is no Go toolchain to build it with." >&2
        echo "See https://github.com/rhysd/actionlint/releases — 'just workflows-check' needs it." >&2
    fi

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

# Remove build artefacts, release tarballs and the coverage report.
clean:
    cargo clean
    rm -rf dist/ lcov.info coverage-summary.txt

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
