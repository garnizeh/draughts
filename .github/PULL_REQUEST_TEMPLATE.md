<!--
Everything that lands in this repository is written in English — including this
description. See CONTRIBUTING.md.
-->

## What this changes

<!-- One paragraph. What the tree does after this that it did not do before. -->

## Why, and which § decided it

<!--
Cite the section that owns the behaviour. A constant with no citation is the
thing most likely to be "cleaned up" into a bug later.

Closes #
-->

## The gate

<!--
Paste the actual output. Do not describe a run you did not make. `just pre-pr` is
every job CI runs, not the `gate` one alone; if a recipe could not run here — the
CUDA path needs a toolkit — name it rather than rounding it up to green.
-->

```
$ just pre-pr
```

## The five rules

Tick what applies; delete what does not. An unticked box that should have been
ticked is the review finding this section exists to prevent.

- [ ] **`format_version`** — this change reads or writes a persisted BLOB, and
      every read dispatches on `format_version`; every insert names it
      explicitly from `CURRENT_FORMAT_VERSION`
      ([§13.7](../docs/architecture/13-data-dictionary.md)).
- [ ] **Transposition determinism** — this change touches `probe`, `store` or
      `EvaluatorIdentity`, and the differential search test is green at 1, 2, 8
      and 10 threads
      ([§20.5](../docs/architecture/20-testing-strategy.md#205-transposition-table-tests)).
- [ ] **One device seam** — `candle_core::Device` is still constructed only in
      `face::device::select_device`; `just device-check` is green
      ([§19.6.5](../docs/architecture/19-extensibility-roadmap.md)).
- [ ] **The LLM never plays** — `CommentaryContext` gained no field that could
      influence a move ([§2.3](../docs/architecture/02-scope-and-constraints.md)).
- [ ] **The CPU path still works** — every new test runs on
      `face.device = "cpu"`, on the default build, on a runner with no driver
      ([§20.10](../docs/architecture/20-testing-strategy.md#2010-device-parity-and-cuda-tests)).

## Housekeeping

- [ ] [CHANGELOG.md](../CHANGELOG.md) records this under `[Unreleased]`, or this
      change is not worth recording and that was a decision.
- [ ] New performance numbers went to
      [Appendix B](../docs/architecture/appendix-b-performance-targets.md), not
      into a comment.
- [ ] Tests are named as the property they assert, not as the function they
      call.
- [ ] Every CodeRabbit comment has a reply on its own thread. The "Outside
      diff range" findings inside the review body never appear in the comments
      endpoint and have no thread to reply on — those are answered together in
      one PR comment naming each file and line.

## Anything a reviewer should push back on

<!--
An assumption you made, a § you think is wrong, a seam you left `todo!()` on
purpose. Naming it here is faster than being asked.
-->
