---
name: architecture-map
description: Find the authoritative architecture section for any question about draughts — engine, MCTS, transposition table, Face/LLM layer, device selection, persistence, schema, writer actor, API contract, concurrency, memory budgets, testing, configuration, deployment, acceptance criteria. Use before designing, implementing, or reviewing anything in this tree, and whenever a decision needs a § citation for a comment or a commit message.
---

# Architecture map

`docs/architecture/` is one document in 34 files, version 1.5, approved. It is
authoritative: the code is written against it, and every non-obvious decision in
the tree cites the section that justifies it. Read the section before writing
the code, and cite it in the comment.

## How to use this

1. Find the § in the table below.
2. Read that file — the whole section, not a grep hit. These sections derive
   their numbers from stated relations; a number pulled out of context is a
   number you cannot defend in review.
3. Cite it. `// §6.7.4: entries are evicted by ...` is the house style.

If you cannot find the section, `grep -rn '<term>' docs/architecture/` and then
read the enclosing `##` heading. The index at
`docs/architecture/README.md` also carries a "what it settles" column per file.

## The map

| § | File | Settles |
|---|---|---|
| 0 | `00-revision-history.md` | What changed in 1.1–1.5 and why; the eight corrections in §0.2.2; §0.5 the draw rules |
| 1 | `01-executive-summary.md` | The two modes; what the MVP emphasizes |
| 2 | `02-scope-and-constraints.md` | In/out of scope; §2.3 the LLM does not play; §2.4 the hardware baseline every number derives from |
| 3 | `03-system-context.md` | Every component in one process; the files on disk |
| 4 | `04-separation-of-concerns.md` | Per-layer responsibilities and *forbidden* responsibilities |
| 5 | `05-runtime-components.md` | The seven components: 5.2 HTTP, 5.3 rules, **5.3.1 the draw rules**, 5.4 MCTS, 5.5 lab, 5.6 persistence, 5.7 Face |
| 6 | `06-mcts-extensibility.md` | Domain types, `EvaluationStrategy`, 6.7 the global transposition table, 6.7.5 the two table modes, 6.8 determinism |
| 7 | `07-face-llm-layer.md` | 7.4.1 device selection, 7.5 model profiles, 7.7 guardrails, 7.8 circuit breaker, 7.9 fallback |
| 8 | `08-game-modes-and-flows.md` | Play Mode and Lab Mode end to end |
| 9 | `09-api-contract.md` | 9.1 the error model, 9.2 `/health`, the `/api/v1` surface |
| 10 | `10-frontend-architecture.md` | HTMX partials; 10.3 the interaction pattern; Alpine only for square selection |
| 11 | `11-database-architecture.md` | 11.1 SQLite pragmas, 11.2 the MPSC writer actor, 11.2.5 backpressure, 11.4 durability classes |
| 12 | `12-database-schema.md` | The MVP schema — migration 1 |
| 13 | `13-data-dictionary.md` | Every column; **13.7 `format_version`** |
| 14 | `14-sampling-strategy.md` | What the lab records, at what density, at what disk cost |
| 15 | `15-concurrency-model.md` | 15.1 HTTP/engine separation, 15.2 the write path, 15.3 SQLite concurrency, 15.4 CPU partitioning |
| 16 | `16-memory-strategy.md` | 16.2 engine budgets, 16.3 table sizing, 16.4 host residency, 16.6 the VRAM budget |
| 17 | `17-reliability.md` | 17.2 Face failures, 17.3 database failures, 17.5 in-process inference risk |
| 18 | `18-security-and-safety.md` | 18.2 prompt safety |
| 19 | `19-extensibility-roadmap.md` | 19.6 GPU acceleration, **19.6.5 what the MVP must preserve**, 19.6.6 what CUDA is not for |
| 20 | `20-testing-strategy.md` | 20.1 rules, 20.2 engine, 20.4 load, **20.5 transposition**, 20.6 writer/durability, 20.7 Face, **20.8 format version**, 20.9 baselines, **20.10 device parity** |
| 21 | `21-observability.md` | What is logged, counted, and exposed |
| 22 | `22-deployment-model.md` | 22.1 single-machine deployment, 22.3 startup order, 22.4 shutdown, 22.5 the playbook |
| 23 | `23-configuration-example.md` | The config file; **23.1 startup validation** |
| 24 | `24-key-decisions.md` | The decisions and their alternatives |
| 25 | `25-acceptance-criteria.md` | What "done" means |
| 26 | `26-summary.md` | The whole thing in a page |
| A | `appendix-a-memory-budget.md` | The memory budget, derived |
| B | `appendix-b-performance-targets.md` | Every performance target — **new numbers go here** |
| C | `appendix-c-migration-from-v1-0.md` | v1.0 migration |
| D | `appendix-d-risks-and-open-questions.md` | What could go wrong |
| E | `appendix-e-change-checklist.md` | The checklist for changing the architecture |
| F | `appendix-f-v13-to-v14-checklist.md` | v1.3 → v1.4 |

## Reading paths

- **New to the system** — §1, §3, §4, §5. Forty minutes.
- **Engine work** — §5.3, §5.4, §6 in full, §15.1, §16.2, §20.1, §20.2, §20.5.
- **Persistence work** — §5.6, §11 in full, §12, §13, §15.2, §15.3, §17.3, §20.6.
- **Face work** — §2.3, §5.7, §7 in full, §10.3, §15.4, §17.2, §17.5, §18.2, §20.7.
- **GPU work** — §0.4, §7.4.1, §7.5, §16.6, §19.6, §20.10. Read §19.6.6 before
  proposing any other use of the device.
- **Operating a deployment** — §9.2, §16, §21, §22.5, §23, Appendix D.

## When the document is wrong

It has been wrong before — the eight defects in §0.2.2 were all found by reading,
two revisions were each caused by one number nobody had derived, and a third by a rule four sections required and none stated. If the
architecture does not hold together, say so and stop; changing the document is a
separate act from changing the code, and it has its own checklist
(Appendix E).
