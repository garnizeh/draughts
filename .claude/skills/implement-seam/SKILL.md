---
name: implement-seam
description: Turn one of the todo!() seams in draughts into a real implementation — move generation, move application, tree search, random rollout, transposition store, the writer actor loop, the lab worker pool, batch recovery, or GGUF/tokenizer loading. Use whenever asked to implement, finish, fill in, or make something work in src/, and whenever a todo!() is hit at runtime or in a test.
---

# Implementing a seam

The tree is scaffolding by design: each unfinished part is a `todo!()` placed at
the seam the architecture defines for it, naming the section that owns it. That
citation is the specification. Implementing against a guess instead of against
the section is the failure mode this skill exists to prevent.

## The seams

```
src/rules/moves.rs:100      move generation             §5.3, perft baselines in §20.1
src/rules/moves.rs:111      move application            §5.3
src/engine/mcts.rs:142      tree search                 §6.4; safety net is §20.5
src/engine/evaluator.rs:164 random rollout playout      §6.3
src/engine/transposition.rs:288 store, merge, capacity  §6.7.5
src/db/writer.rs:156        writer actor loop           §11.2.2, durability §20.6
src/lab/runner.rs:123       worker pool, batch lifecycle §5.5, §15.3
src/lab/runner.rs:132       interrupted-batch recovery  §11.4
src/face/candle_adapter.rs:87  GGUF load + tokenizer    §7.4
src/face/candle_adapter.rs:121 host residency accounting §16.4
src/face/candle_adapter.rs:129 CUDA memory accounting   §16.6
```

Line numbers drift; `grep -rn 'todo!' src` is authoritative.

## Procedure

1. **Read the cited section in full.** Not the grep hit — the section. Follow
   the `architecture-map` skill if you need to locate more.
2. **Read the surrounding module.** The types around the seam were written
   against the same section and already encode half the decisions: the seam is
   usually smaller than it looks because the signature is already correct.
   Do not change a public signature to make an implementation easier without
   saying why the architecture allows it.
3. **Read the testing subsection that owns it** (§20.1 rules, §20.2 engine,
   §20.5 transposition, §20.6 writer, §20.7 Face). It states the property the
   implementation must have. That property is the test you write.
4. **Implement.** Cite the § for every constant and every non-obvious branch.
   Match the comment density and voice of the file — these are prose comments
   that explain *why*, not labels that restate the code.
5. **Write the tests as properties.** Named for the claim, not the function:
   `a_capture_is_mandatory_when_one_exists`, not `test_gen_moves`. Put
   cross-cutting claims no single module owns in `tests/startup_invariants.rs`.
6. **Run `just ci`.** Not `cargo test` alone — the gate includes two static
   checks and the doc build, and a seam that passes the suite while breaking
   `device-check` is not finished.
7. **Update `CHANGELOG.md`**: move the item out of "Not yet implemented" into
   "Added", with the § link. That file is how the project reports its own state.

## Constraints that bite at these seams

- **`rules::moves`** — the perft baselines in §20.1 are the specification of
  correctness; a move generator that passes its own tests and fails perft is
  wrong. Captures are mandatory in English draughts, and multi-jumps are one
  move, not several.
- **`engine::mcts`** — determinism is §6.8. The differential search test
  (§20.5) must produce identical output at 1, 2, 8 and 10 threads with the
  table on and off. If your search consults wall-clock time or thread order to
  decide *what* it returns, it is wrong.
- **`engine::transposition::store`** — §6.7.5. `Estimate` entries are readable
  only in `TtMode::Throughput`; storing one in `Deterministic` mode is a defect,
  not an optimisation. A truncated move list is an illegal-move generator:
  `SmallMoveList::from_moves` returning `None` means cache the value *without*
  the list.
- **`db::writer`** — §11.2. One writer thread, batching into one transaction.
  Backpressure is `WriteQueueSaturated` (§11.2.5), which is the system working,
  not failing. Every insert names `format_version` from
  `CURRENT_FORMAT_VERSION` — see the `persisted-format` skill.
- **`lab::runner`** — §15.3. Workers share one transposition table and funnel
  every write through the one actor. A worker that opens its own connection has
  broken the design.
- **`face::candle_adapter`** — the device arrives as a parameter. Do not
  construct one (`face-layer` skill). Loading must fail into the circuit
  breaker and the canned provider, never into a startup failure: a game against
  canned commentary with no model file present is a fully valid game.

## What not to do

- Do not delete a `todo!()` and return a plausible default. A panic that says
  "unimplemented, see §6.4" is more honest than a wrong answer, and something
  downstream is asserting on it.
- Do not implement two seams at once unless one cannot compile without the
  other. Each is reviewable on its own against one section.
- Do not add a dependency to make a seam easier. The dependency list in
  `Cargo.toml` is annotated per section; adding to it is an architectural change.
