//! Auto-tuning: the "just make it work" path for a zero-knowledge user. Instead
//! of guessing a learning rate or optimizer, the user states a goal and the
//! system searches a small space of [`TrainingConfig`]s, runs each as a real
//! (short) training trial, and keeps the one that actually trains best.
//!
//! This module owns the **search strategy** — space generation, the
//! successive-halving schedule, and ranking — as pure, fully-tested logic. The
//! trials themselves (which need torch + the orchestrator) are driven by the
//! caller through a simple evaluator, so the reusable core is verifiable
//! without a training runtime, and a synthetic objective stands in for real
//! training in the unit tests.
use crate::bbir::{BBIRGraph, TrainingConfig};
use serde::Serialize;

/// The knobs auto-tuning sweeps. Kept deliberately small — these are the levers
/// that most often decide whether a model trains at all, and a compact grid
/// keeps the search cheap enough to run on a laptop.
///
/// `width_scales` is the light **architecture** dimension: each scale multiplies
/// the hidden-width hyperparameters of the graph's layers (see
/// [`apply_width_scale`]), so the search can find not just a good training
/// config but a better-sized model. `1.0` leaves the architecture untouched.
#[derive(Debug, Clone)]
pub struct SearchSpace {
    pub learning_rates: Vec<f64>,
    pub batch_sizes: Vec<usize>,
    pub optimizers: Vec<String>,
    pub width_scales: Vec<f64>,
}

impl SearchSpace {
    /// A sensible default grid centered on the base config's current choices,
    /// spanning the learning rates and optimizers that matter most in practice.
    /// Architecture is left alone by default (`width_scales = [1.0]`); call
    /// [`SearchSpace::with_architecture_search`] to also sweep model width.
    pub fn default_around(base: &TrainingConfig) -> Self {
        let base_batch = base.data_source.batch_size.max(1);
        Self {
            learning_rates: vec![0.1, 0.01, 0.001, 0.0001],
            batch_sizes: vec![base_batch, (base_batch * 2).max(2)],
            optimizers: vec!["adam".to_string(), "sgd".to_string()],
            width_scales: vec![1.0],
        }
    }

    /// Add a light architecture search: also try a narrower and a wider model
    /// (half / double the hidden widths) alongside the given base. Opt-in
    /// because it multiplies the grid and an ill-fitting scale simply fails its
    /// trial (and ranks last), rather than being silently accepted.
    pub fn with_architecture_search(mut self) -> Self {
        self.width_scales = vec![0.5, 1.0, 2.0];
        self
    }

    /// Total number of candidate configs this space expands to.
    pub fn size(&self) -> usize {
        self.learning_rates.len() * self.batch_sizes.len() * self.optimizers.len() * self.width_scales.len().max(1)
    }
}

/// One point in the search space: a training config plus the architecture width
/// scale to apply to the graph before training it.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub config: TrainingConfig,
    pub width_scale: f64,
}

/// Expand a search space into concrete candidates, each a clone of `base` with
/// one grid point applied (and the width scale to apply to the graph). Capped at
/// `max_candidates` (a budget) so a large grid can't blow up the number of real
/// training runs.
pub fn candidate_configs(base: &TrainingConfig, space: &SearchSpace, max_candidates: usize) -> Vec<Candidate> {
    let mut out = Vec::new();
    let width_scales = if space.width_scales.is_empty() { &[1.0][..] } else { &space.width_scales[..] };
    for &lr in &space.learning_rates {
        for &batch in &space.batch_sizes {
            for opt in &space.optimizers {
                for &width_scale in width_scales {
                    if out.len() >= max_candidates {
                        return out;
                    }
                    let mut cfg = base.clone();
                    cfg.optimizer = opt.clone();
                    cfg.data_source.batch_size = batch;
                    set_lr(&mut cfg, lr);
                    out.push(Candidate { config: cfg, width_scale });
                }
            }
        }
    }
    out
}

/// Hidden-width hyperparameter names the architecture search is allowed to
/// scale. Deliberately a curated allow-list: scaling something data-tied (an
/// embedding's `vocab_size`, an output dimension) would break the graph, so we
/// touch only the interior widths that are safe to grow/shrink.
const SCALABLE_WIDTH_KEYS: &[&str] = &["hidden", "hidden_size", "units", "features", "out_features", "dim", "d_model"];

/// Apply an architecture width scale to a graph in place: multiply every
/// scalable integer hidden-width hyperparameter by `scale`, rounded, min 1. A
/// `scale` of 1.0 is a no-op. An unlucky scale that produces an inconsistent
/// graph just fails its trial downstream (scored `None`), so this can only
/// *propose* architectures, never silently corrupt one.
pub fn apply_width_scale(graph: &mut BBIRGraph, scale: f64) {
    if (scale - 1.0).abs() < f64::EPSILON {
        return;
    }
    for node in &mut graph.nodes {
        let Some(map) = node.hyperparams.as_object_mut() else { continue };
        for key in SCALABLE_WIDTH_KEYS {
            if let Some(v) = map.get_mut(*key) {
                if let Some(n) = v.as_u64() {
                    let scaled = ((n as f64) * scale).round().max(1.0) as u64;
                    *v = serde_json::json!(scaled);
                }
            }
        }
    }
}

/// Write a learning rate into a config's `hyperparams` JSON object, preserving
/// any other hyperparameters already there.
fn set_lr(cfg: &mut TrainingConfig, lr: f64) {
    let obj = cfg.hyperparams.as_object_mut();
    match obj {
        Some(map) => {
            map.insert("lr".to_string(), serde_json::json!(lr));
        }
        None => {
            cfg.hyperparams = serde_json::json!({ "lr": lr });
        }
    }
}

/// Read the learning rate back out of a config (for reporting/tests).
pub fn lr_of(cfg: &TrainingConfig) -> Option<f64> {
    cfg.hyperparams.get("lr").and_then(|v| v.as_f64())
}

/// The successive-halving schedule: how many candidates survive each round,
/// starting from `n` and keeping the better half (rounded up) until one
/// remains. Cheap trials eliminate most candidates early; only the promising
/// few get the expensive later rounds. Pure + deterministic.
pub fn halving_schedule(n: usize) -> Vec<usize> {
    if n == 0 {
        return vec![];
    }
    let mut schedule = vec![n];
    let mut cur = n;
    while cur > 1 {
        cur = cur.div_ceil(2);
        schedule.push(cur);
    }
    schedule
}

/// Given trial scores (lower is better, e.g. final loss), return the indices of
/// the `keep` best, preserving best-first order. The core selection step of
/// successive halving.
pub fn select_survivors(scores: &[(usize, f32)], keep: usize) -> Vec<usize> {
    let mut ranked: Vec<(usize, f32)> = scores.to_vec();
    // Map NaN to +inf so a diverged trial deterministically sorts last (a plain
    // `partial_cmp` on NaN yields an inconsistent order and an unspecified sort).
    let key = |x: f32| if x.is_nan() { f32::INFINITY } else { x };
    ranked.sort_by(|a, b| key(a.1).partial_cmp(&key(b.1)).unwrap());
    ranked.into_iter().take(keep).map(|(i, _)| i).collect()
}

/// One trial's outcome.
#[derive(Debug, Clone, Serialize)]
pub struct TrialResult {
    pub index: usize,
    pub learning_rate: Option<f64>,
    pub batch_size: usize,
    pub optimizer: String,
    /// Architecture width scale applied to the graph for this trial (1.0 = as
    /// authored). Reported so the UI/leaderboard can show which size won.
    pub width_scale: f64,
    /// Final loss (lower is better); `None` if the trial errored/diverged.
    pub score: Option<f32>,
}

/// Rank completed trials best-first (lowest score wins; failed trials last).
pub fn rank(results: &[TrialResult]) -> Vec<TrialResult> {
    let mut r = results.to_vec();
    r.sort_by(|a, b| match (a.score, b.score) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Greater),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbir::DataSourceConfig;

    fn base_config() -> TrainingConfig {
        TrainingConfig {
            loss: "mse".into(),
            optimizer: "sgd".into(),
            trainer_type: "standard".into(),
            hyperparams: serde_json::json!({ "epochs": 5 }),
            data_source: DataSourceConfig {
                source_type: "file".into(),
                path_or_uri: "data.csv".into(),
                batch_size: 16,
                preprocessing: vec![],
                sequence_length: None,
                vocab_size: None,
                image_size: None,
                grayscale: None,
                text_column: None,
                label_column: None,
            },
            reproducibility: None,
        }
    }

    #[test]
    fn candidate_generation_covers_the_grid_and_respects_the_budget() {
        let base = base_config();
        let space = SearchSpace::default_around(&base);
        let all = candidate_configs(&base, &space, 1000);
        assert_eq!(all.len(), space.size());
        // Learning rate is actually applied and other hyperparams preserved.
        assert!(all.iter().all(|c| lr_of(&c.config).is_some()));
        assert!(all.iter().all(|c| c.config.hyperparams.get("epochs").is_some()));
        // Budget cap is honored.
        let capped = candidate_configs(&base, &space, 3);
        assert_eq!(capped.len(), 3);
    }

    #[test]
    fn halving_schedule_shrinks_to_one() {
        assert_eq!(halving_schedule(8), vec![8, 4, 2, 1]);
        assert_eq!(halving_schedule(5), vec![5, 3, 2, 1]);
        assert_eq!(halving_schedule(1), vec![1]);
        assert_eq!(halving_schedule(0), Vec::<usize>::new());
    }

    #[test]
    fn survivors_keep_the_lowest_scores() {
        let scores = vec![(0, 0.9f32), (1, 0.1), (2, 0.5), (3, 0.3)];
        assert_eq!(select_survivors(&scores, 2), vec![1, 3]);
    }

    #[test]
    fn survivors_push_nan_scores_to_the_back() {
        let scores = vec![(0, f32::NAN), (1, 0.2), (2, 0.4)];
        // The two real scores win over the diverged (NaN) trial.
        let s = select_survivors(&scores, 2);
        assert!(s.contains(&1) && s.contains(&2));
        assert!(!s.contains(&0));
    }

    #[test]
    fn ranking_puts_the_best_first_and_failures_last() {
        let results = vec![
            TrialResult { index: 0, learning_rate: Some(0.1), batch_size: 16, optimizer: "sgd".into(), width_scale: 1.0, score: Some(0.5) },
            TrialResult { index: 1, learning_rate: Some(0.01), batch_size: 16, optimizer: "adam".into(), width_scale: 1.0, score: None },
            TrialResult { index: 2, learning_rate: Some(0.001), batch_size: 16, optimizer: "adam".into(), width_scale: 2.0, score: Some(0.2) },
        ];
        let ranked = rank(&results);
        assert_eq!(ranked[0].index, 2); // lowest loss
        assert_eq!(ranked[1].index, 0);
        assert_eq!(ranked[2].index, 1); // failed trial last
    }

    // Simulates a full successive-halving run against a synthetic objective
    // whose optimum is a known learning rate, proving the strategy converges on
    // it without any training runtime.
    #[test]
    fn successive_halving_finds_the_synthetic_optimum() {
        let base = base_config();
        let space = SearchSpace::default_around(&base);
        let candidates = candidate_configs(&base, &space, 1000);

        // Objective: loss is distance of log10(lr) from the ideal (1e-3), so
        // lr = 0.001 is best. batch/optimizer don't matter here.
        let objective = |cfg: &TrainingConfig| -> f32 {
            let lr = lr_of(cfg).unwrap_or(1.0);
            ((lr.log10() - (0.001f64).log10()).abs()) as f32
        };

        let mut alive: Vec<usize> = (0..candidates.len()).collect();
        for &survivors in halving_schedule(candidates.len()).iter().skip(1) {
            let scores: Vec<(usize, f32)> = alive.iter().map(|&i| (i, objective(&candidates[i].config))).collect();
            alive = select_survivors(&scores, survivors);
        }
        assert_eq!(alive.len(), 1);
        let winner = &candidates[alive[0]];
        assert_eq!(lr_of(&winner.config), Some(0.001));
    }

    #[test]
    fn architecture_search_multiplies_the_grid_and_carries_scales() {
        let base = base_config();
        let space = SearchSpace::default_around(&base).with_architecture_search();
        let all = candidate_configs(&base, &space, 10_000);
        // Three width scales widen the grid threefold vs. the config-only space.
        assert_eq!(all.len(), space.size());
        assert!(all.iter().any(|c| (c.width_scale - 0.5).abs() < 1e-9));
        assert!(all.iter().any(|c| (c.width_scale - 2.0).abs() < 1e-9));
    }

    #[test]
    fn width_scale_grows_only_curated_hidden_keys() {
        use crate::bbir::{BBIRNode, PortInfo};
        let node = |hp: serde_json::Value| BBIRNode {
            id: "n".into(),
            component: "linear".into(),
            label: None,
            hyperparams: hp,
            ports: PortInfo { input_ports: vec![], output_ports: vec![] },
            position: None,
        };
        let mut graph = BBIRGraph {
            schema_version: 1,
            graph_id: "g".into(),
            name: "t".into(),
            nodes: vec![node(serde_json::json!({ "hidden": 32, "vocab_size": 5000 }))],
            edges: vec![],
            training: None,
        };
        apply_width_scale(&mut graph, 2.0);
        let hp = graph.nodes[0].hyperparams.as_object().unwrap();
        assert_eq!(hp["hidden"].as_u64(), Some(64), "hidden width doubles");
        assert_eq!(hp["vocab_size"].as_u64(), Some(5000), "data-tied dims are left alone");

        // A scale of 1.0 is a no-op.
        let mut g2 = graph.clone();
        apply_width_scale(&mut g2, 1.0);
        assert_eq!(g2.nodes[0].hyperparams["hidden"].as_u64(), Some(64));
    }
}
