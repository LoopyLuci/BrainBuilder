use async_trait::async_trait;
use crate::runtime::scheduler::ExecutionPlan;
use crate::data::source::DataIterator;
use crate::Result;

/// Generic trainer interface. Implementations for standard, GAN, RL, etc.
#[async_trait]
pub trait Trainer: Send + Sync {
    async fn fit(&self, plan: ExecutionPlan, data: &mut dyn DataIterator) -> Result<()>;
}

/// Holds all available trainer implementations, keyed by type string.
pub struct TrainerController {
    trainers: std::collections::HashMap<String, Box<dyn Trainer>>,
}

impl TrainerController {
    pub fn new(context: std::sync::Arc<crate::AppContext>) -> Result<Self> {
        let mut map: std::collections::HashMap<String, Box<dyn Trainer>> = std::collections::HashMap::new();
        map.insert(
            "standard".into(),
            Box::new(super::standard_trainer::StandardTrainer::new(context)?),
        );
        Ok(Self { trainers: map })
    }

    pub fn get_trainer(&self, config: &Option<crate::bbir::TrainingConfig>) -> Result<&Box<dyn Trainer>> {
        match config {
            Some(cfg) => self.trainers.get(&cfg.trainer_type).ok_or_else(|| {
                crate::interop::protocol::BrainBuilderError::ConfigError(format!(
                    "unknown trainer {}",
                    cfg.trainer_type
                ))
            }),
            None => Err(crate::interop::protocol::BrainBuilderError::ConfigError(
                "missing training config".into(),
            )),
        }
    }
}
