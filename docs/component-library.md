# Component Library

The catalog of every shipped component under `components/`. Each is a `.edn`
descriptor (see [Component System](component-system.md) for the format) paired with a
Python implementation under `components/python/`. All 16 descriptors currently declare
`:language "python"` — `conv2d.edn` additionally lists an unimplemented `"rust"` entry
(no matching `conv2d.rs` exists), so Rust-language dispatch is aspirational, not real,
for any shipped component today.

## Status summary

Of the 16 leaf components, **15 have complete, non-stub Python implementations**. The
one exception, `image_classifier_pipeline`, is a `:meta-type "pipeline"` composite by
design (not a leaf module), so it has no single-entry-point `.py` file to begin with —
this is expected, not a gap. One quality caveat: `conv2d.py`'s forward pass ignores the
`stride`/`padding` hyperparameters its own descriptor declares, always doing a
default-stride "valid" convolution.

## Catalog

| Component | Meta-type | Ports | What it computes |
|---|---|---|---|
| **adam** | optimizer | `params`, `grads` in → `updated_params` out (all `Data` role — no `Parameter` ports) | One real, bias-corrected (t=1 only) Adam step. Standalone/demo only — the real trainer uses `torch.optim.Adam` directly for true multi-step momentum/variance state, which this stateless interface can't represent |
| **add** | pure-function | `a`, `b` → `output` | `a + b`, elementwise residual/skip connection |
| **attention** | stateful-module | `input` + parameters `q_weight`, `k_weight`, `v_weight`, `out_weight` → `output` | Real multi-head scaled-dot-product self-attention (Vaswani et al.), splits into `num_heads` heads, validates `features % num_heads == 0` |
| **confidence_head** | stateful-module, tags `dspark/speculative/drafter` | `input` + `hidden_weight`, `score_weight` → `output` | `sigmoid(linear(silu(linear(input, hidden_weight)), score_weight))` — scores the probability a drafted token should be accepted. See [DSpark](dspark-system.md) |
| **conv2d** | stateful-module, cnn/vision | `x`, `weight` (parameter) → `y` | `torch.nn.functional.conv2d(x, weight)`, no bias. **Ignores `stride`/`padding` hyperparameters** — always default-stride valid convolution |
| **dropout** | pure-function, regularization | `input` → `output` | `torch.nn.functional.dropout(input, p, training)`. `training` defaults `True`; the inference path (`ExecutionPlan::forward`) explicitly passes `training=False` |
| **embedding** | stateful-module, transformer/nlp | `ids`, `weight` (parameter) → `output` | `torch.nn.functional.embedding(ids.long(), weight)` — `ids` arrives as float32 (BrainBuilder's tensor exchange is float32 end-to-end) and is cast to long internally |
| **gelu** | pure-function/activation | `input` → `output` | `torch.nn.functional.gelu(input)` |
| **image_classifier_pipeline** | pipeline (composite) | `image` (batch,3,224,224) → `logits` (batch,10) | No `.py` — composes other leaf components at the graph level; not a standalone module by design |
| **layernorm** | stateful-module, normalization | `input`, `weight`/`bias` (parameter) → `output` | `torch.nn.functional.layer_norm`, driven off the actual `weight` tensor's shape rather than the `features` hyperparameter so it self-corrects if they diverge |
| **linear** | stateful-module | `input`, `weight` (parameter) → `output` | `torch.nn.functional.linear(input, weight)` — no bias port at all |
| **lora_linear** | stateful-module, fine-tuning/lora | `input`; `weight` (parameter, `trainable: false`); `lora_a` (parameter); `lora_b` (parameter, `zero_init: true`) → `output` | Real LoRA (Hu et al.): `base + (lora_b @ lora_a @ input) * (alpha/rank)`, `lora_b` zero-initialized so training starts as an exact no-op on the frozen base |
| **low_rank_markov_head** | stateful-module, tags `dspark/speculative/drafter` | `input`, `compress_weight`, `expand_weight` (parameters) → `output` (a logit bias) | Compress→expand low-rank bottleneck that biases next-token logits from the preceding hidden state, curing "suffix decay" in a parallel drafter. See [DSpark](dspark-system.md) |
| **parallel_intern** | stateful-module, tags `dspark/speculative/drafter` | `input`, `heads_weight` (parameter, one projection head per future position) → `output` | `einsum("bf,dvf->bdv", ...)` — drafts `draft_len` future tokens in one parallel pass ("the Intern"). See [DSpark](dspark-system.md) |
| **relu** | function | `x` → `y` | `torch.relu(x)` |
| **scale** | function | `x` (1-D), `w` (parameter, shape `[1]`) → `y` | `y = x * w`. Exists specifically because the Arrow data bridge produces one 1-D per-column tensor, whereas `linear`/`conv2d` need a proper 2-D `(batch, features)` matrix nothing currently assembles — `scale` is the honest building block for today's simple tabular pipeline. CPU-only compatibility declared |
| **select_last** | pure-function, sequence | `input` (batch,seq,features) → `output` (batch,features) | `input[:, -1, :]`, a real differentiable slice for next-token-style readout |

## Notes on parameter roles in practice

- `lora_linear` is the clearest real example of `Parameter` role fields in use:
  `weight` is a frozen (`trainable: false`) parameter, `lora_a`/`lora_b` are trainable,
  and `lora_b` is `zero_init: true`. See [Component System](component-system.md#ports-and-roles)
  for what these fields mean to the trainer.
- The three DSpark-tagged components (`parallel_intern`, `low_rank_markov_head`,
  `confidence_head`) are trainable, first-class BrainBuilder components — they can be
  wired into the main graph editor and trained end-to-end like any other component.
  `dspark_system/` (the standalone serving engine) independently reimplements the same
  architectures as plain `torch.nn.Module` classes so the serving engine has no runtime
  dependency on the graph editor — see [DSpark](dspark-system.md#relationship-to-the-trainable-graph-components)
  for the important caveat that there's no code path today that loads editor-trained
  weights into the serving engine.

## Templates that use this library

The [built-in templates](templates.md) are the fastest way to see these components
wired into real graphs — e.g. the "Attention Stack" template chains `embedding →
attention → layernorm`, and the "DSpark Speculative Drafter" template places all three
DSpark-tagged components on the canvas at once.
