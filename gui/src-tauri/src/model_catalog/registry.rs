use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

#[async_trait::async_trait]
pub trait Model: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn execute(&self, _params: HashMap<String, Value>) -> Result<Value, String>;
}

pub struct ModelRegistry {
    models: HashMap<String, Arc<dyn Model>>,
}

impl ModelRegistry {
    pub fn new() -> Self { Self { models: HashMap::new() } }
    pub fn register(&mut self, model: Arc<dyn Model>) { self.models.insert(model.id().to_string(), model); }
    pub async fn execute(&self, id: &str, _params: HashMap<String, Value>) -> Result<Value, String> {
        match self.models.get(id) { Some(model) => model.execute(_params).await, None => Err(format!("Model '{}' not found", id)) }
    }
    pub fn list(&self) -> Vec<(&str, &str)> { self.models.values().map(|m| (m.id(), m.name())).collect() }
}
