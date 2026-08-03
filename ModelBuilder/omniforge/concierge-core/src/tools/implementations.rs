//! Concrete tool execution. Platform-specific actions are abstracted behind
//! the `PlatformHost` trait; a fully functional `MockPlatformHost` is provided
//! for testing, auditing and offline development.

use async_trait::async_trait;
use tracing::info;

use super::registry::ToolResult;
use crate::error::ConciergeError;

/// Trait that the OmniForge desktop / server host must implement.
/// All canvas, model-registry and training operations go through this boundary.
#[async_trait]
pub trait PlatformHost: Send + Sync {
    async fn search_models(
        &self,
        query: &str,
        modality: Option<&str>,
        arch: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn import_model(
        &self,
        path: &str,
        format: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn list_models(&self, limit: usize) -> Result<String, ConciergeError>;

    async fn add_node(
        &self,
        node_type: &str,
        model_id: Option<&str>,
        label: Option<&str>,
        x: f64,
        y: f64,
        config: Option<serde_json::Value>,
    ) -> Result<String, ConciergeError>;

    async fn connect_nodes(
        &self,
        source_node: &str,
        source_socket: &str,
        target_node: &str,
        target_socket: &str,
    ) -> Result<String, ConciergeError>;

    async fn create_dataset(
        &self,
        name: &str,
        sources: &[String],
        labels: Option<&[String]>,
        format: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn run_training(
        &self,
        base_model: &str,
        dataset: &str,
        recipe: &str,
        output_name: Option<&str>,
        epochs: Option<i64>,
        learning_rate: Option<f64>,
    ) -> Result<String, ConciergeError>;

    async fn create_knowledge_module(
        &self,
        base_model: &str,
        adapter_path: &str,
        metadata: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn merge_models(
        &self,
        model_ids: &[String],
        strategy: &str,
        weights: Option<&[f64]>,
        output_name: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn inspect_node(&self, node_id: &str) -> Result<String, ConciergeError>;

    async fn execute_graph(
        &self,
        inputs: &serde_json::Value,
        timeout_ms: Option<u64>,
    ) -> Result<String, ConciergeError>;

    async fn compare_outputs(
        &self,
        node_a: &str,
        node_b: &str,
        input: &str,
    ) -> Result<String, ConciergeError>;

    async fn search_docs(&self, query: &str, limit: usize) -> Result<String, ConciergeError>;

    async fn write_plugin(
        &self,
        description: &str,
        language: &str,
        name: Option<&str>,
    ) -> Result<String, ConciergeError>;

    async fn modify_graph(
        &self,
        action: &str,
        target: &str,
        payload: Option<&serde_json::Value>,
    ) -> Result<String, ConciergeError>;

    async fn get_platform_status(&self) -> Result<String, ConciergeError>;
}

/// Fully functional mock host that logs every call and returns deterministic
/// success values. Sufficient for unit tests, integration tests and auditing.
pub struct MockPlatformHost;

#[async_trait]
impl PlatformHost for MockPlatformHost {
    async fn search_models(
        &self,
        query: &str,
        modality: Option<&str>,
        arch: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(query, ?modality, ?arch, "search_models");
        Ok(format!(
            "Found 0 models matching '{}'(modality={:?}, arch={:?}). Mock mode – replace with real registry.",
            query, modality, arch
        ))
    }

    async fn import_model(
        &self,
        path: &str,
        format: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(path, ?format, ?name, "import_model");
        Ok(format!(
            "Model imported (mock) from {} as {:?} (name={:?})",
            path, format, name
        ))
    }

    async fn list_models(&self, limit: usize) -> Result<String, ConciergeError> {
        info!(limit, "list_models");
        Ok(format!("[] (mock – limit {})", limit))
    }

    async fn add_node(
        &self,
        node_type: &str,
        model_id: Option<&str>,
        label: Option<&str>,
        x: f64,
        y: f64,
        config: Option<serde_json::Value>,
    ) -> Result<String, ConciergeError> {
        info!(node_type, ?model_id, ?label, x, y, ?config, "add_node");
        Ok(format!("node-{}", uuid::Uuid::new_v4()))
    }

    async fn connect_nodes(
        &self,
        source_node: &str,
        source_socket: &str,
        target_node: &str,
        target_socket: &str,
    ) -> Result<String, ConciergeError> {
        info!(
            source_node,
            source_socket, target_node, target_socket, "connect_nodes"
        );
        Ok(format!(
            "connection established: {}:{} → {}:{}",
            source_node, source_socket, target_node, target_socket
        ))
    }

    async fn create_dataset(
        &self,
        name: &str,
        sources: &[String],
        labels: Option<&[String]>,
        format: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(name, ?sources, ?labels, ?format, "create_dataset");
        Ok(format!("Dataset '{}' created (mock)", name))
    }

    async fn run_training(
        &self,
        base_model: &str,
        dataset: &str,
        recipe: &str,
        output_name: Option<&str>,
        epochs: Option<i64>,
        learning_rate: Option<f64>,
    ) -> Result<String, ConciergeError> {
        info!(
            base_model,
            dataset, recipe, ?output_name, ?epochs, ?learning_rate, "run_training"
        );
        Ok(format!("training-job-{}", uuid::Uuid::new_v4()))
    }

    async fn create_knowledge_module(
        &self,
        base_model: &str,
        adapter_path: &str,
        metadata: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(base_model, adapter_path, ?metadata, ?name, "create_knowledge_module");
        Ok(format!("km-{}", uuid::Uuid::new_v4()))
    }

    async fn merge_models(
        &self,
        model_ids: &[String],
        strategy: &str,
        weights: Option<&[f64]>,
        output_name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(?model_ids, strategy, ?weights, ?output_name, "merge_models");
        Ok(format!("merged-model-{}", uuid::Uuid::new_v4()))
    }

    async fn inspect_node(&self, node_id: &str) -> Result<String, ConciergeError> {
        info!(node_id, "inspect_node");
        Ok(format!(
            r#"{{"id":"{}","type":"model","status":"idle","sockets":{{"in":["input"],"out":["output"]}}}}"#,
            node_id
        ))
    }

    async fn execute_graph(
        &self,
        inputs: &serde_json::Value,
        timeout_ms: Option<u64>,
    ) -> Result<String, ConciergeError> {
        info!(%inputs, ?timeout_ms, "execute_graph");
        Ok(r#"{"status":"ok","outputs":{"result":"mock output"}}"#.to_string())
    }

    async fn compare_outputs(
        &self,
        node_a: &str,
        node_b: &str,
        input: &str,
    ) -> Result<String, ConciergeError> {
        info!(node_a, node_b, input, "compare_outputs");
        Ok(r#"{"similarity":0.87,"note":"mock comparison"}"#.to_string())
    }

    async fn search_docs(&self, query: &str, limit: usize) -> Result<String, ConciergeError> {
        info!(query, limit, "search_docs");
        Ok(format!(
            "No documentation entries found for '{}' (mock, limit={})",
            query, limit
        ))
    }

    async fn write_plugin(
        &self,
        description: &str,
        language: &str,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(description, language, ?name, "write_plugin");
        Ok(format!(
            "Plugin scaffold generated (lang={}, name={:?}). Description: {}",
            language, name, description
        ))
    }

    async fn modify_graph(
        &self,
        action: &str,
        target: &str,
        payload: Option<&serde_json::Value>,
    ) -> Result<String, ConciergeError> {
        info!(action, target, ?payload, "modify_graph");
        Ok(format!("Graph modified: action={}, target={}", action, target))
    }

    async fn get_platform_status(&self) -> Result<String, ConciergeError> {
        info!("get_platform_status");
        Ok(r#"{"status":"healthy","models":0,"nodes":0,"jobs":0,"mode":"mock"}"#.to_string())
    }
}

/// Dispatches tool calls to the underlying PlatformHost (and a few local helpers).
pub struct ToolExecutor {
    host: Box<dyn PlatformHost>,
}

impl ToolExecutor {
    pub fn new(host: Box<dyn PlatformHost>) -> Self {
        Self { host }
    }

    /// Execute a tool by name with a JSON argument object.
    pub async fn execute(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolResult, ConciergeError> {
        match tool_name {
            "search_models" => {
                let query = arguments["query"].as_str().unwrap_or("");
                let modality = arguments["modality"].as_str();
                let arch = arguments["arch"].as_str();
                let result = self.host.search_models(query, modality, arch).await?;
                Ok(ToolResult::ok(result))
            }
            "import_model" => {
                let path = arguments["path"].as_str().unwrap_or("");
                let format = arguments["format"].as_str();
                let name = arguments["name"].as_str();
                let result = self.host.import_model(path, format, name).await?;
                Ok(ToolResult::ok(result))
            }
            "list_models" => {
                let limit = arguments["limit"].as_u64().unwrap_or(50) as usize;
                let result = self.host.list_models(limit).await?;
                Ok(ToolResult::ok(result))
            }
            "add_node" => {
                let node_type = arguments["node_type"].as_str().unwrap_or("model");
                let model_id = arguments["model_id"].as_str();
                let label = arguments["label"].as_str();
                let x = arguments
                    .get("position")
                    .and_then(|p| p.get("x"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let y = arguments
                    .get("position")
                    .and_then(|p| p.get("y"))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let config = arguments.get("config").cloned();
                let result = self
                    .host
                    .add_node(node_type, model_id, label, x, y, config)
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "connect_nodes" => {
                let source_node = arguments["source_node"].as_str().unwrap_or("");
                let source_socket = arguments["source_socket"].as_str().unwrap_or("output");
                let target_node = arguments["target_node"].as_str().unwrap_or("");
                let target_socket = arguments["target_socket"].as_str().unwrap_or("input");
                let result = self
                    .host
                    .connect_nodes(source_node, source_socket, target_node, target_socket)
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "create_dataset" => {
                let name = arguments["name"].as_str().unwrap_or("unnamed");
                let sources: Vec<String> = arguments["sources"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let labels: Option<Vec<String>> = arguments["labels"].as_array().map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                });
                let format = arguments["format"].as_str();
                let result = self
                    .host
                    .create_dataset(
                        name,
                        &sources,
                        labels.as_deref(),
                        format,
                    )
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "run_training" => {
                let base_model = arguments["base_model"].as_str().unwrap_or("");
                let dataset = arguments["dataset"].as_str().unwrap_or("");
                let recipe = arguments["recipe"].as_str().unwrap_or("lora");
                let output_name = arguments["output_name"].as_str();
                let epochs = arguments["epochs"].as_i64();
                let learning_rate = arguments["learning_rate"].as_f64();
                let result = self
                    .host
                    .run_training(
                        base_model,
                        dataset,
                        recipe,
                        output_name,
                        epochs,
                        learning_rate,
                    )
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "create_knowledge_module" => {
                let base_model = arguments["base_model"].as_str().unwrap_or("");
                let adapter_path = arguments["adapter_path"].as_str().unwrap_or("");
                let metadata = arguments["metadata"]
                    .as_object()
                    .map(|o| serde_json::to_string(o).unwrap_or_default());
                let name = arguments["name"].as_str();
                let result = self
                    .host
                    .create_knowledge_module(
                        base_model,
                        adapter_path,
                        metadata.as_deref(),
                        name,
                    )
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "merge_models" => {
                let model_ids: Vec<String> = arguments["model_ids"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                let strategy = arguments["strategy"].as_str().unwrap_or("linear");
                let weights: Option<Vec<f64>> = arguments["weights"].as_array().map(|a| {
                    a.iter().filter_map(|v| v.as_f64()).collect()
                });
                let output_name = arguments["output_name"].as_str();
                let result = self
                    .host
                    .merge_models(
                        &model_ids,
                        strategy,
                        weights.as_deref(),
                        output_name,
                    )
                    .await?;
                Ok(ToolResult::ok(result))
            }
            "inspect_node" => {
                let node_id = arguments["node_id"].as_str().unwrap_or("");
                let result = self.host.inspect_node(node_id).await?;
                Ok(ToolResult::ok(result))
            }
            "execute_graph" => {
                let inputs = arguments
                    .get("inputs")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));
                let timeout_ms = arguments["timeout_ms"].as_u64();
                let result = self.host.execute_graph(&inputs, timeout_ms).await?;
                Ok(ToolResult::ok(result))
            }
            "compare_outputs" => {
                let node_a = arguments["node_a"].as_str().unwrap_or("");
                let node_b = arguments["node_b"].as_str().unwrap_or("");
                let input = arguments["input"].as_str().unwrap_or("");
                let result = self.host.compare_outputs(node_a, node_b, input).await?;
                Ok(ToolResult::ok(result))
            }
            "search_docs" => {
                let query = arguments["query"].as_str().unwrap_or("");
                let limit = arguments["limit"].as_u64().unwrap_or(5) as usize;
                let result = self.host.search_docs(query, limit).await?;
                Ok(ToolResult::ok(result))
            }
            "write_plugin" => {
                let description = arguments["description"].as_str().unwrap_or("");
                let language = arguments["language"].as_str().unwrap_or("python");
                let name = arguments["name"].as_str();
                let result = self.host.write_plugin(description, language, name).await?;
                Ok(ToolResult::ok(result))
            }
            "modify_graph" => {
                let action = arguments["action"].as_str().unwrap_or("");
                let target = arguments["target"].as_str().unwrap_or("");
                let payload = arguments.get("payload");
                let result = self.host.modify_graph(action, target, payload).await?;
                Ok(ToolResult::ok(result))
            }
            "get_platform_status" => {
                let result = self.host.get_platform_status().await?;
                Ok(ToolResult::ok(result))
            }
            // These two are handled at the agent level (they need the CodeAgent)
            "set_code_agent_mode" | "execute_python" => Err(ConciergeError::ToolExecution(
                format!(
                    "Tool '{}' is handled by the agent layer, not the ToolExecutor",
                    tool_name
                ),
            )),
            _ => Err(ConciergeError::ToolExecution(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    }
}
