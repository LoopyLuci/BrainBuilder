use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use crate::runtime::scheduler::ExecutionPlan;
use crate::data::source::DataIterator;
use crate::data::metrics;
use crate::interop::python::PythonBridge;
use crate::interop::protocol::BrainBuilderError;
use crate::AppContext;
use crate::Result;
use crate::Tensor;

pub struct StandardTrainer {
    context: Arc<AppContext>,
    python: PythonBridge,
}

impl StandardTrainer {
    pub fn new(context: Arc<AppContext>) -> crate::Result<Self> {
        let python = PythonBridge::new(context.arena.clone())?;
        Ok(Self { context, python })
    }
}

#[async_trait]
impl super::trainer::Trainer for StandardTrainer {
    async fn fit(&self, plan: ExecutionPlan, data: &mut dyn DataIterator) -> Result<()> {
        let training_cfg = plan.graph.training.as_ref().ok_or_else(|| {
            BrainBuilderError::ConfigError("graph has no training config".into())
        })?;
        let lr = training_cfg
            .hyperparams
            .get("lr")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.01);
        // 0.0 (the default) means "off" — the optimizer behaves exactly as
        // it did before this hyperparameter existed. A positive value pulls
        // every weight a little toward zero on each step (L2 regularization,
        // passed straight through to torch.optim's own `weight_decay`
        // kwarg), which discourages any single weight from growing large
        // enough to memorize noise in the training data instead of the real
        // pattern — a different lever from `dropout` (a per-component node
        // on the canvas), not a replacement for it.
        let weight_decay = training_cfg
            .hyperparams
            .get("weight_decay")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // 0.0 (the default) means "off" — SGD updates using only the
        // current step's gradient, same as before this hyperparameter
        // existed. A value in (0, 1) (0.9 is the classic default
        // elsewhere) carries forward a fraction of the previous step's
        // update direction (torch's own SGD `momentum` kwarg — real
        // physical-momentum-style velocity, not an approximation),
        // smoothing out noisy per-batch gradients and often reaching a
        // good loss faster. Meaningless for `adam`, which has its own
        // built-in adaptive momentum and doesn't accept this kwarg at all —
        // silently ignored there rather than erroring.
        let momentum = training_cfg
            .hyperparams
            .get("momentum")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // 0.0 (the default) means "off" — gradients pass through unmodified,
        // exactly as before this hyperparameter existed. A positive value
        // caps the total L2 norm of the trainable parameters' gradients at
        // that value (via torch's own `clip_grad_norm_`) right before the
        // optimizer step, so a single unusually large batch or a step early
        // in training can't blow the weights up in one move — a stability
        // lever, distinct from `weight_decay` (which shrinks weights every
        // step regardless of the gradient) and `patience` (which stops the
        // run entirely instead of tempering individual steps).
        let grad_clip = training_cfg
            .hyperparams
            .get("grad_clip")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // 0.0 (the default) means "off" — cross_entropy trains against
        // exact one-hot targets, same as before this hyperparameter
        // existed. A value in (0, 1) blends each target a little toward a
        // uniform distribution over every class (torch's own
        // `label_smoothing` kwarg), discouraging the model from becoming
        // fully, overconfidently certain about training labels it may have
        // gotten from noisy data — a different lever from `weight_decay`
        // (shrinks weights) or `dropout` (drops signal): this one softens
        // what "correct" means. Meaningless (and silently ignored) for
        // `mse`, which has no notion of a class distribution.
        let label_smoothing = training_cfg
            .hyperparams
            .get("label_smoothing")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // 0 (the default — see gui's TRAINING_SCHEMA) means "off": every
        // epoch runs, exactly like before this feature existed. A positive
        // patience stops training once `patience` epochs in a row fail to
        // improve the epoch's average loss by more than EARLY_STOP_MIN_DELTA
        // — the noise floor for float32 loss values, so trailing-digit jitter
        // alone can't look like "no improvement" forever.
        let patience = training_cfg
            .hyperparams
            .get("patience")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        // 0 (the default) means "off" — the learning rate stays exactly
        // `lr` for the whole run, same as before this feature existed. A
        // positive value halves the current learning rate every time that
        // many epochs complete, so later epochs — once the model is
        // already roughly in the right place — take smaller, more careful
        // steps instead of the same large ones the first epoch used.
        let lr_decay_epochs = training_cfg
            .hyperparams
            .get("lr_decay_epochs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let mut current_lr = lr;
        const EARLY_STOP_MIN_DELTA: f32 = 1e-4;
        let mut best_epoch_loss = f32::INFINITY;
        let mut epochs_without_improvement = 0usize;
        let loss_name = training_cfg.loss.clone();
        let optimizer_name = training_cfg.optimizer.clone();

        // Learned parameters persist across the whole training loop (every
        // epoch, every batch) — a fresh optimizer/weight init per batch
        // would never converge. Seeded from any pretrained weights the plan
        // loaded (transfer learning): those ports start from the real
        // pretrained tensor instead of `torch.randn`, and frozen ones (a
        // `lora_linear` base `weight`) are never handed to the optimizer, so
        // they stay fixed while the adapters/head learn.
        let mut weights: HashMap<String, Tensor> = plan.preset_weights.clone();
        let mut step = 0usize;

        let epochs = plan.epochs;
        for epoch in 0..epochs {
            // Halve the learning rate every `lr_decay_epochs` completed
            // epochs (epoch 0 always uses the full starting `lr`).
            if lr_decay_epochs > 0 && epoch > 0 && epoch % lr_decay_epochs == 0 {
                current_lr *= 0.5;
            }
            // Real bug fix: without rewinding, every epoch after the first
            // silently ran zero batches (see `DataIterator::reset`'s doc
            // comment) — `epochs` had no effect beyond the first pass over
            // the data no matter what a user configured.
            data.reset()?;
            let mut epoch_loss_sum = 0f32;
            let mut epoch_steps = 0usize;
            let mut last_point: Option<metrics::MetricPoint> = None;
            while let Some(batch) = data.next()? {
                let inputs = plan.prepare_inputs(batch)?;
                let loss = plan.train_step(
                    step,
                    inputs,
                    &mut weights,
                    &loss_name,
                    &optimizer_name,
                    current_lr,
                    weight_decay,
                    grad_clip,
                    label_smoothing,
                    momentum,
                    &self.python,
                )?;
                self.context.provenance.log(crate::utils::provenance::ProvenanceRecord {
                    graph_version: plan.graph.graph_id.clone(),
                    node_id: plan
                        .operations
                        .last()
                        .map(|op| op.node_id.clone())
                        .unwrap_or_default(),
                    sample_id: step as u64,
                    step,
                })?;
                step += 1;
                epoch_loss_sum += loss.value;
                epoch_steps += 1;

                let point = metrics::MetricPoint {
                    epoch,
                    step: loss.step,
                    loss: loss.value,
                    stopped_early: false,
                    current_lr: current_lr as f32,
                };
                metrics::publish_metric(point.clone());
                last_point = Some(point);
            }

            if patience > 0 && epoch_steps > 0 {
                let epoch_avg = epoch_loss_sum / epoch_steps as f32;
                if epoch_avg < best_epoch_loss - EARLY_STOP_MIN_DELTA {
                    best_epoch_loss = epoch_avg;
                    epochs_without_improvement = 0;
                } else {
                    epochs_without_improvement += 1;
                    if epochs_without_improvement >= patience {
                        log::info!(
                            "early stopping graph `{}`: no improvement in {patience} epoch(s) (best avg loss \
                             {best_epoch_loss}) — stopping after epoch {epoch} of {epochs}",
                            plan.graph.name
                        );
                        if let Some(mut point) = last_point {
                            point.stopped_early = true;
                            metrics::publish_metric(point);
                        }
                        break;
                    }
                }
            }
        }

        if !weights.is_empty() {
            let checkpoint_path = self.context.checkpoint_path(&plan.graph.graph_id);
            // Archive whatever checkpoint is about to be overwritten — training
            // over a working model used to be a one-way door; now the previous
            // state is always recoverable (see utils::checkpoint_versions).
            crate::utils::checkpoint_versions::archive_current(&self.context.checkpoints_dir, &plan.graph.graph_id)?;
            self.python.save_state_dict(&weights, &checkpoint_path)?;
            log::info!(
                "saved checkpoint for graph `{}` to {}",
                plan.graph.name,
                checkpoint_path.display()
            );
        }
        Ok(())
    }
}
