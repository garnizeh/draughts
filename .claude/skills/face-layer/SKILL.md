---
name: face-layer
description: The Face/LLM commentary layer in draughts — device selection, CUDA vs CPU profiles, GGUF and tokenizer loading, the circuit breaker, canned commentary, prompt construction, output sanitization, and the constraint that the LLM never plays. Use when touching src/face, anything CUDA or candle, commentary latency, VRAM budgets, or the cuda cargo feature.
---

# The Face layer

Commentary. It runs in-process — no daemon, no REST hop, no second failure
domain — and **it is never permitted to touch the game**.

The governing principle: *the engine is authoritative and everything else is
optional*. A game played entirely against canned commentary, with no model file
present, is a fully valid game. Every decision below follows from that.

## The two hard constraints

**The LLM never plays** (§2.3). It cannot choose, validate, or influence a move.
This is enforced by types: `CommentaryContext` is the whole of what the Face
layer is given, and there is no move in it. Adding a field to that struct is an
architectural change, not a convenience — if you are reaching for one, the
design intends you to reach for something else.

**`candle_core::Device` is constructed in exactly one function** —
`face::device::select_device` (§19.6.5 property 1). A second `Device::Cpu` or
`Device::new_cuda` anywhere in `src/`, `tests/` or `benches/` fails
`just device-check`. The device is resolved once at startup and passed as a
parameter thereafter. This is what keeps the next device change a one-line edit
instead of a search-and-replace.

## Device selection — §7.4.1

CUDA is the *default* device and is *required by nothing*. On the target host
two DDR4 channels give two cores ~15 GB/s against the card's ~168, and the CPU
path cannot meet its own 2.5 s deadline at any model worth listening to. But:

- `cargo build --release` with no features produces a binary with **no CUDA
  dependency** that plays the same draughts on a machine with no driver. CI
  builds it in a driver-less container, runs it, and greps `objdump` to prove it.
- A card that is absent, busy, or out of memory **falls back** to the CPU
  profile rather than failing. `select_device` never returns an error;
  `DeviceKind` records what was actually resolved, and `/health` reports both
  what was requested and what was resolved.

## Two profiles, not one model path — §7.5.4

Two `.gguf` files ship, one per device profile: Qwen2.5-1.5B-Instruct on CUDA,
Qwen2.5-0.5B-Instruct on the CPU fallback (both Apache 2.0). The resolved device
can change between one boot and the next without anyone editing configuration,
and a single `model_path` would then put a 4.3-second model against a 2.5-second
deadline — a correct degradation that produces a silent outage.

§23.1 validation therefore checks deadline feasibility for **both** profiles:
the active one failing refuses startup, the inactive one failing warns.

## The circuit breaker — §7.8

Three failures amputate the model for five minutes. A degraded model must never
degrade gameplay. What counts as a failure is §7.8.3 — not everything that goes
wrong is a breaker trip, and classifying too broadly makes the breaker fire on
load. The breaker is tested against an *injected clock* (`SystemClock` is the
production one); a test that sleeps is a test that will flake.

When the breaker is open, the canned provider answers (§7.9). Canned commentary
covers every `(CommentaryEvent, Tone)` pair — that totality is what makes the
fallback unconditional. Adding a variant to either enum means adding canned
lines for it.

Note the deliberate absence of a `face_unavailable` error status in §9.1: the
API has no way to say "commentary is down", because from the client's side it
never is.

## Latency and the critical path — §10.3

Commentary arrives **separately**, whenever the model finishes. It is never on
the move-response path: a *successful* 2.5-second inference on the critical path
is still 2.5 seconds. The board comes back at engine speed.

## Guardrails — §7.7, §18.2

Model output is sanitized before it reaches a page. Prompt construction is
§18.2 — the position is data, and nothing derived from it is permitted to act
as an instruction.

## Budgets

- Host residency: §16.4. Peak RSS under a full-density batch is a **gate**, not
  a metric: < 56 GB.
- VRAM: §16.6. Peak VRAM for commentary under lab load < 4.5 GB, also a gate.
- What CUDA is *not* for: §19.6.6. Read it before proposing any other use of the
  device. The engine never touches it.

## Working on the GPU path

```bash
CUDA_COMPUTE_CAP=86 just check-cuda   # compiles without a device — a CI gate
CUDA_COMPUTE_CAP=86 just build-cuda   # needs a toolkit
```

**A change that can only be tested on a GPU has broken the CPU path.** The whole
suite runs on `face.device = "cpu"`, on the default build, on a runner with no
driver. That is the gate (§20.10). Face tests are §20.7.
