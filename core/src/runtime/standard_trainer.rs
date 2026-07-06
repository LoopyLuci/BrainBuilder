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
        let loss_name = training_cfg.loss.clone();
        let optimizer_name = training_cfg.optimizer.clone();

        // Learned parameters persist across the whole training loop (every
        // epoch, every batch) — a fresh optimizer/weight init per batch
        // would never converge.
        let mut weights: HashMap<String, Tensor> = HashMap::new();
        let mut step = 0usize;

        let epochs = plan.epochs;
        for epoch in 0..epochs {
            // Real bug fix: without rewinding, every epoch after the first
            // silently ran zero batches (see `DataIterator::reset`'s doc
            // comment) — `epochs` had no effect beyond the first pass over
            // the data no matter what a user configured.
            data.reset()?;
            while let Some(batch) = data.next()? {
                let inputs = plan.prepare_inputs(batch)?;
                let loss = plan.train_step(
                    step,
                    inputs,
                    &mut weights,
                    &loss_name,
                    &optimizer_name,
                    lr,
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

                metrics::publish_metric(metrics::MetricPoint {
                    epoch,
                    step: loss.step,
                    loss: loss.value,
                });
            }
        }

        if !weights.is_empty() {
            let checkpoint_path = self.context.checkpoint_path(&plan.graph.graph_id);
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
