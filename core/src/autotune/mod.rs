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
use crate::bbir::TrainingConfig;
use serde::Serialize;

/// The knobs auto-tuning sweeps. Kept deliberately small — these are the levers
/// that most often decide whether a model trains at all, and a compact grid
/// keeps the search cheap enough to run on a laptop.
#[derive(Debug, Clone)]
pub struct SearchSpace {
    pub learning_rates: Vec<f64>,
    pub batch_sizes: Vec<usize>,
    pub optimizers: Vec<String>,
}

impl SearchSpace {
    /// A sensible default grid centered on the base config's current choices,
    /// spanning the learning rates and optimizers that matter most in practice.
    pub fn default_around(base: &TrainingConfig) -> Self {
        let base_batch = base.data_source.batch_size.max(1);
        Self {
            learning_rates: vec![0.1, 0.01, 0.001, 0.0001],
            batch_sizes: vec![base_batch, (base_batch * 2).max(2)],
            optimizers: vec!["adam".to_string(), "sgd".to_string()],
        }
    }

    /// Total number of candidate configs this space expands to.
    pub fn size(&self) -> usize {
        self.learning_rates.len() * self.batch_sizes.len() * self.optimizers.len()
    }
}

/// Expand a search space into concrete candidate configs, each a clone of
/// `base` with one grid point applied. Capped at `max_candidates` (a budget) so
/// a large grid can't blow up the number of real training runs.
pub fn candidate_configs(base: &TrainingConfig, space: &SearchSpace, max_candidates: usize) -> Vec<TrainingConfig> {
    let mut out = Vec::new();
    for &lr in &space.learning_rates {
        for &batch in &space.batch_sizes {
            for opt in &space.optimizers {
                if out.len() >= max_candidates {
                    return out;
                }
                let mut cfg = base.clone();
                cfg.optimizer = opt.clone();
                cfg.data_source.batch_size = batch;
                set_lr(&mut cfg, lr);
                out.push(cfg);
            }
        }
    }
    out
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
        assert!(all.iter().all(|c| lr_of(c).is_some()));
        assert!(all.iter().all(|c| c.hyperparams.get("epochs").is_some()));
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
            TrialResult { index: 0, learning_rate: Some(0.1), batch_size: 16, optimizer: "sgd".into(), score: Some(0.5) },
            TrialResult { index: 1, learning_rate: Some(0.01), batch_size: 16, optimizer: "adam".into(), score: None },
            TrialResult { index: 2, learning_rate: Some(0.001), batch_size: 16, optimizer: "adam".into(), score: Some(0.2) },
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
            let scores: Vec<(usize, f32)> = alive.iter().map(|&i| (i, objective(&candidates[i]))).collect();
            alive = select_survivors(&scores, survivors);
        }
        assert_eq!(alive.len(), 1);
        let winner = &candidates[alive[0]];
        assert_eq!(lr_of(winner), Some(0.001));
    }
}
