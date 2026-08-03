# DSpark Speculative Decoding

`dspark_system/` is a standalone speculative-decoding **inference-serving** engine —
deliberately decoupled from BrainBuilder's graph-based trainer. As its own module
docstring puts it: "Training a model and serving it speculatively are distinct
concerns... a separate entry point rather than being bolted into BrainBuilder's
trainer" (`dspark_system/serve.py:4-6`).

## The problem it solves

Classic draft/verify speculative decoding: a cheap "Intern" drafter proposes multiple
future tokens per step, and an expensive "Boss" verifier model checks them all in one
parallel pass via rejection sampling — reducing the number of full forward passes
needed per generated token.

## Configuration (`dspark_system/config.py`)

`DSparkConfig`: `vocab_size=32000`, `hidden_dim=1024`, `max_draft_len=16` /
`min_draft_len=2` (the range the SPS manager interpolates draft length across),
`markov_rank=32`, `confidence_threshold=0.60`, `gpu_critical_load=0.85` /
`gpu_idle_load=0.40` (thresholds for the SPS curve).

## Dynamic hardware adaptation (`dspark_system/dynamic_hardware.py`)

`SPSManager` — **"SPS" stands for "System Performance vs Speed"** (defined explicitly
in-code, not "speculation-per-step"). It reads live GPU utilization via `pynvml` when
available, falling back to a `psutil` CPU-load proxy, and finally to a fixed 0.5
mid-point if neither library is usable — logging a one-time warning per failure mode
rather than crashing. `draft_length(active_requests)` shortens drafts to `min_len`
under heavy load (utilization above the critical threshold, or over 100 active
requests), lengthens to `max_len` when idle, and linearly interpolates in between. It
also owns real NVML resource lifecycle (`nvmlInit`/`nvmlShutdown`) to avoid handle
leaks.

## Neural heads (`dspark_system/neural_components.py`)

`ParallelIntern` (one `nn.Linear` per future position, up to `max_draft_len`),
`LowRankMarkovHead` (compress→expand bottleneck), `ConfidenceHead`
(`Linear→SiLU→Linear→Sigmoid`, scoring acceptance probability). The module docstring
states these "mirror the trainable BrainBuilder components of the same names; kept
here too so the serving engine is self-contained."

## The engine (`dspark_system/engine.py`)

`SpeculativeEngine` is constructed from an injected `intern_backbone` and `boss_model`
plus a `DSparkConfig` (dependency injection, so existing model code is never modified),
and instantiates its own `ParallelIntern`, `LowRankMarkovHead`, `ConfidenceHead`, and
`SPSManager`.

`generate(input_ids, max_new_tokens)` loops, under `@torch.no_grad()`:

1. Ask the `SPSManager` for a `draft_len` given the current batch size.
2. Get the backbone's hidden state for the last position.
3. Draft `draft_len` tokens via `ParallelIntern`, biased by `LowRankMarkovHead`'s
   suffix-decay correction, chosen via argmax.
4. Score each draft position with `ConfidenceHead` and find the first low-confidence
   cutoff (a batch-min over positions).
5. If nothing survives the cutoff, fall back to one ordinary autoregressive step.
   Otherwise verify the surviving prefix against the boss model in one parallel pass
   via rejection sampling, keeping the longest contiguous agreeing prefix (falling
   back to one ordinary step if nothing is accepted).

`boss_calls` is an instrumentation counter used by the acceptance benchmark below.

## Running it

`python -m dspark_system.serve`:

- `python -m dspark_system.serve --prompt-len 4 --new-tokens 8` — offline demo with a
  random stand-in backbone/verifier.
- `python -m dspark_system.serve --model gpt2 --prompt "Once upon a time" --new-tokens
  32` — loads a real HuggingFace `transformers` causal-LM checkpoint via adapter
  classes, with an optional `--draft-model` for a distinct smaller drafter (validated
  for matching `hidden_size`/`vocab_size`; self-speculation against the same checkpoint
  if omitted). `transformers` is an optional, lazily-imported dependency — the hard pip
  requirement (`dspark_system/requirements.txt`) is only `psutil>=5.9` (torch is
  installed separately).

Smoke tests: `python dspark_system/test_smoke.py` covers SPS fallback bounds,
confidence-cutoff bounds, rejection-sampling contiguity, and no-mutation-of-input,
against tiny stand-in modules. Both are wired into
[`scripts/verify.ps1`](scripts-and-testing.md#scriptsverifyps1).

## Relationship to the trainable graph components

`parallel_intern`, `low_rank_markov_head`, and `confidence_head` exist as first-class
BrainBuilder [components](component-library.md) (all tagged
`dspark`/`speculative`/`drafter`, real `Parameter`-role ports, real autograd) — they
can be wired into the main graph editor via the
[DSpark Speculative Drafter template](templates.md#the-five-built-in-templates) and
trained end-to-end by BrainBuilder's normal trainer/optimizer, with weights persisted
through the normal checkpoint mechanism.

`dspark_system/neural_components.py` **independently reimplements the same
architectures** as plain `torch.nn.Module` classes so the serving engine has no
runtime dependency on the graph-editor/component-descriptor machinery. This is a
self-contained duplication for deployment, not a shared-weights integration — **there
is currently no code path that loads weights trained in the graph editor into the
`dspark_system` serving engine's modules.** Bridging that gap (loading a trained
checkpoint's DSpark head weights into `SpeculativeEngine`) is a natural next step, not
something implemented today.

## Acceptance benchmark (`testing/bench/dspark_acceptance.py`)

Since the demo's random stand-in backbone/verifier have no trained weights, the
benchmark can't measure real generation quality. Instead it measures a deterministic
**structural** proxy: tokens produced per boss-model call
(`tokens_per_boss_call = tokens_produced / boss_calls`), run across 10 fixed seeds with
a fixed small `DSparkConfig`. It asserts every seed's ratio falls within `[0.5, 2.0]` —
a band with headroom around the empirically measured healthy range of 0.80–1.00 —
designed to catch structural regressions: an always-reject bug in `_verify`/the
confidence cutoff collapses the ratio toward the low end; an always-accept bug spikes
it well above 2.0; a broken `boss_calls` counter shows up as an out-of-band ratio
either way. It writes/overwrites `testing/bench/last_result.json` with per-seed detail
and a `pass` field; the checked-in result shows all 10 seeds passing with
`avg_tokens_per_boss_call ≈ 0.934`. This benchmark is one of the optional gate steps in
[`scripts/ship.ps1`](scripts-and-testing.md#scriptsshipps1).
