# Autotune

`core/src/autotune/mod.rs` implements a "just make it work" search: instead of a user
guessing learning rate, batch size, and optimizer, BrainBuilder sweeps a small grid,
runs real short trials, and keeps the best. Search-strategy logic (space generation,
successive halving, ranking) is pure and fully unit-tested here; trial *execution*
needs a live training run and lives in the Tauri command.

## Search space

`SearchSpace::default_around(base)` centers the grid on the current config:
`learning_rates = [0.1, 0.01, 0.001, 0.0001]`, `batch_sizes = [base_batch,
base_batch*2]`, `optimizers = ["adam", "sgd"]`, `width_scales = [1.0]` (architecture
untouched by default). `with_architecture_search()` widens `width_scales` to `[0.5,
1.0, 2.0]`.

## Candidate expansion

`candidate_configs(base, space, max_candidates)` nested-loops lr × batch × optimizer ×
width_scale, cloning `base` and applying each combination, capped at
`max_candidates` — the "budget" parameter from the Tauri command.

## Architecture width scaling

`apply_width_scale(graph, scale)` multiplies every hyperparameter matching a curated
allow-list (`SCALABLE_WIDTH_KEYS`: `hidden`, `hidden_size`, `units`, `features`,
`out_features`, `dim`, `d_model`) by `scale`, rounded, floored at 1. Deliberately
excludes data-tied dimensions like `vocab_size` — verified by a dedicated test — so
scaling can only *propose* an architecture; an inconsistent result simply fails its
trial downstream (scored `None`), never silently corrupts the graph.

## Successive halving

`halving_schedule(n)` returns `[n, ceil(n/2), ceil(n/4), ..., 1]`. `select_survivors(scores, keep)`
sorts ascending (lower loss wins), mapping `NaN` to `+inf` so a diverged trial
deterministically sorts last, and returns the top `keep` indices.

## Ranking

`rank(results)` sorts `TrialResult`s ascending by score, with `None` (errored or
diverged trials) always ranked last regardless of comparison — explicit `Ordering`
arms handle the `(Some, None)`/`(None, Some)` cases so this can't accidentally invert.

## Tauri command: `autotune(graph_json, budget, search_arch, state)`

Pulls `graph.training` as the base config (errors if absent), builds a `SearchSpace`
(plus architecture search if requested), and calls `candidate_configs`. For each
candidate, sequentially: clones the graph, applies `apply_width_scale`, sets the trial
config, then captures the trial's best (lowest) loss by **concurrently** draining
`subscribe_metrics()` (a bounded `tokio::sync::broadcast` channel) while
`orchestrator.execute_graph(trial_graph).await` runs. A `tokio::spawn`'d collector task
uses `tokio::select!` with metrics-draining prioritized over a stop signal — this
specific ordering exists to avoid `RecvError::Lagged` from the bounded channel causing
a false "trial failed" score of `None`, a subtlety documented directly in the code
comment. On an `execute_graph` error, the trial scores `None` regardless of any points
collected before the failure. Results are ranked via `autotune::rank` and returned as
JSON.

## GUI

The [Auto-tune button in the Data panel](gui-panels.md#data-panel) serializes the
current canvas to BBIR, calls `autotune` with a budget of 8 or 16 trials (depending on
whether architecture search is enabled), and displays results ranked with the winner
marked. Applying the result sets `optimizer`/`learning_rate`/`batch_size` on the
training config and, if `width_scale != 1`, resizes every node's scalable-width
hyperparameters to match — using the same `SCALABLE_WIDTH_KEYS` list the backend uses,
kept in sync by convention rather than a shared constant.

## Verifying this end to end

See [section 4 of the Live Verification Checklist](VERIFICATION.md#4-auto-tune-phase-d)
for the manual pass criteria (trials stream live, rank correctly, and the winning
config applies with one click).
