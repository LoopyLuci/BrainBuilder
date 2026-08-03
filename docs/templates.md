# Templates

`gui/src/templates/` is the "build agents easily" feature: a small library of starter
graphs a user can drop onto an empty canvas in one click, instead of assembling every
node by hand.

## Registry (`gui/src/templates/registry.ts`)

`Template` = `{id, name, category, description, nodes[], edges[], training?}`.
`instantiateTemplate()` converts declarative node/edge specs into real reactflow
`Node`/`Edge` objects, seeding hyperparameters from the live
[component descriptor](component-system.md) registry's defaults and then overriding
with the template's own values — the same shape `graphStore.addNode` produces.
`missingComponents()` flags any referenced component not present in the current
registry, so the panel can disable "Use" and warn rather than placing a broken node —
this is what protects a template against drift if a component is ever renamed or
removed.

## The five built-in templates

| Template | Category | Graph | Notes |
|---|---|---|---|
| **MLP Classifier** | Starter | `linear(4→16) → gelu → linear(16→3)`, cross_entropy/adam | The simplest end-to-end trainable graph |
| **Attention Stack** | Transformer | `embedding(vocab=5000, features=64) → attention(num_heads=4) → layernorm(features=64)`, cross_entropy/adam | Core transformer block, "ready to extend" |
| **Text Sentiment Classifier** | Text | bag-of-words `linear(200→32) → gelu → linear(32→2)`, cross_entropy/adam | Description instructs the user to pick a CSV in the [Data panel](gui-panels.md#data-panel), enable the text_column toggle, name the text/label columns, then adjust `in_features` to match the reported vocabulary width. References `gui/examples/sentiment.bbir.edn` as a proven example |
| **Next-Word Predictor** | Transformer | `embedding(vocab=60, embedding_dim=32) → attention(features=32, num_heads=4) → select_last → linear(32→60)` | Predicts the next word of a `text_sequence` dataset by attending over the full context window. References `gui/examples/story.bbir.edn` |
| **DSpark Speculative Drafter** | Speculative decoding | Three parallel heads with no inter-edges: `parallel_intern(features=256, vocab=1000, draft_len=8)`, `low_rank_markov_head(features=256, rank=32, vocab=1000)`, `confidence_head(features=256, proj=128)` | A DeepSeek-style drafter stack — the user wires the shared backbone hidden state in manually. See [DSpark](dspark-system.md) for the serving-side counterpart |

## `TemplatesPanel.tsx` and `useApplyTemplate.ts`

`TemplatesPanel.tsx` groups `TEMPLATES` by category and shows a "Use" button per
entry, disabled with a warning if `missingComponents()` finds a gap. `useApplyTemplate()`
is the shared hook (used by both the Templates panel and the
[canvas empty-state prompt](gui-architecture.md#the-canvas-gui-srccanvas)) — it checks
`missingComponents`, calls `instantiateTemplate`, then `setGraph`/`setTraining`,
logging success or failure to the [Console panel](gui-panels.md#console-panel).

## Related

- [Component Library](component-library.md) — what every component in these graphs
  actually computes.
- [DSpark Speculative Decoding](dspark-system.md) — the standalone serving engine the
  DSpark template's components are trained to feed into.
