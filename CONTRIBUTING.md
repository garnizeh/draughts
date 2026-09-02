# Contributing

The architecture is settled at version 1.4; the implementation is not finished.
Start with [docs/architecture/](docs/architecture/README.md) — the code is
written against it, and every non-obvious decision in the tree cites the section
that justifies it.

## Getting set up

```bash
just setup            # rustfmt, clippy, cargo-deny
just setup-frontend   # vendor htmx and Alpine into static/vendor/
cp draughts.example.toml draughts.toml
just check-config     # §23.1 validation against this host
just ci               # the full merge gate
```

`just` is the only entry point you need. Every recipe is listed by `just` on its
own, and CI runs the same recipes — `just ci` locally and a green pipeline mean
the same thing by construction.

## The gate

```bash
just ci
```

which is `fmt-check`, `lint`, `test`, `device-check`, `format-version-check` and
`docs`. Two of those are static checks that no compiler performs:

- `device-check` asserts `candle_core::Device` is constructed in exactly one
  function ([§19.6.5](docs/architecture/19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve)).
- `format-version-check` asserts every insert path names `format_version`
  ([§20.8](docs/architecture/20-testing-strategy.md#208-format-version-tests)).

The heavy suites are deliberately outside the gate and run nightly:
`just test-tt-off`, `just test-load`, `just bench`.

## The three rules

These are not style preferences. The design rests on them, and each one is a
review blocker on its own.

1. **Reading a persisted BLOB without dispatching on its `format_version` is a
   defect.**
   ([§13.7](docs/architecture/13-data-dictionary.md#137-format_version--new-in-11))
   The column's `DEFAULT` exists for the v1.0 migration and for nothing else.

2. **The transposition table may change how long a search takes and must never
   change what it returns.**
   ([§20.5](docs/architecture/20-testing-strategy.md#205-transposition-table-tests))
   Anything touching `probe`, `store`, or `EvaluatorIdentity` needs the
   differential search test to pass at 1, 2, 8 and 10 threads.

3. **`candle_core::Device` is constructed in exactly one function.**
   ([§19.6.5](docs/architecture/19-extensibility-roadmap.md#1965-what-the-mvp-must-preserve))
   A second construction anywhere is what turns the next device change from a
   one-line edit into a search-and-replace.

## Two more that are easy to break by accident

- **The LLM never plays.** It cannot choose, validate, or influence a move, and
  the constraint is enforced by types: `CommentaryContext` is the whole of what
  the Face layer is given, and there is no move in it
  ([§2.3](docs/architecture/02-scope-and-constraints.md#23-explicit-constraint-llm-does-not-play-draughts)).
  Adding a field to that struct is an architectural change, not a convenience.

- **A change that can only be tested on a GPU has broken the CPU path.** The
  whole suite runs on `face.device = "cpu"`, on the default build, on a runner
  with no driver — that is the gate
  ([§20.10](docs/architecture/20-testing-strategy.md#2010-device-parity-and-cuda-tests)).

## Working on the GPU path

```bash
CUDA_COMPUTE_CAP=86 just check-cuda   # compiles without a device
CUDA_COMPUTE_CAP=86 just build-cuda   # needs a toolkit
```

CI compiles the `cuda` feature on every PR and runs nothing on a device. The
device-requiring half of §20.10 runs on the target host only, and making it a
merge gate would reintroduce the GPU dependency the section exists to prevent.

If `cudarc`'s supported toolkit range lags the installed driver's CUDA version,
install a toolkit in range and point the build at it. That is the fix, not a
driver downgrade
([§22.1](docs/architecture/22-deployment-model.md#221-mvp-single-machine-deployment)).

## Conventions

- Comments explain *why*, and cite the section that decided it. The codebase is
  full of numbers that look arbitrary and are not; a bare constant with no
  citation is the thing most likely to be "cleaned up" into a bug.
- Tests are named as the property they assert, not as the function they call.
- New performance numbers go in
  [Appendix B](docs/architecture/appendix-b-performance-targets.md), not in a
  comment.
- Changing a persisted encoding — including regenerating the Zobrist key table —
  is a `format_version` bump. `just test` will tell you; do not update the
  expected constant to make it stop.

## Responding to CodeRabbit

Every CodeRabbit finding gets a reply on its own thread before the PR merges —
never silence, and never only a summary comment elsewhere on the PR. Verify the
finding against the current code first, since it can be stale by the time you
read it. Then:

- **Fixing it:** reply with the commit (and line, if it moved) that fixes it.
- **Not fixing it:** say why, plainly — a documented `todo!()` seam, a design
  question that needs a human decision rather than a unilateral edit, or a
  finding that no longer applies. CodeRabbit reads the reply and either
  resolves the thread or pushes back on it; that loop is the point.

A PR with unanswered CodeRabbit threads is not done, the same way a PR with a
red `just ci` is not done.

## Reporting a design problem

An issue against the architecture is as valuable as a patch. The eight defects
in [§0.2.2](docs/architecture/00-revision-history.md#022-corrections) were all
found by reading, and the two revisions since were each caused by a single
number nobody had derived.

## Working with Claude Code

The repository carries a committed harness: [CLAUDE.md](CLAUDE.md) for the rules
above, and [.claude/](.claude/README.md) for the skills, the review subagent, and
a post-edit hook that runs the two static checks while the change is still in
hand. It is reviewed like any other file — if it describes a tree that no longer
exists, that is a defect.
