//! Thin facade for self-improving subsystems.
//!
//! Exposes Tauri commands and a single `register()` hook, depending on
//! sibling modules via `crate::` paths.

use std::sync::Arc;
use std::collections::HashMap;
use chrono::Utc;
use tokio::sync::Mutex;
use tauri::Manager;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use uuid::Uuid;
use crate::model_catalog::register_all;

pub struct AppStateExt {
    pub telemetry: crate::telemetry::TelemetryStore,
    pub active_learning: crate::active_learning_loop::ActiveLearningLoop,
    pub online_learning: crate::online_learning_loop::OnlineLearningLoop,
    pub fact_checking: crate::fact_checking_model::FactCheckingModel,
    pub scraping: crate::scraping_model::ScrapingModel,
    pub data_assistant: crate::data_assistant_model::DataAssistantModel,
    pub model_builder: crate::meta_model_builder::MetaModelBuilder,
    pub sandbox: crate::sandbox_executor::SandboxExecutor,
    pub meta: crate::meta_controller::MetaController,
    pub memory: crate::memory_recall::LongTermMemory,
    pub safety: crate::safety_harness::SafetyHarness,
    pub eval: crate::eval_harness::EvalHarness,
    pub model_mistress: crate::model_mistress::ModelMistressBridge,
    pub power_manager: crate::power_manager::PowerManagerBridge,
    pub compression_agent: crate::compression_agent::CompressionAgentBridge,
    pub retrieval_augmented_generation: std::sync::Arc<tokio::sync::Mutex<crate::retrieval_augmented_generation::RetrievalAugmentedGeneration>>,
    pub adaptive_reasoning: std::sync::Arc<tokio::sync::Mutex<crate::adaptive_reasoning::AdaptiveReasoning>>,
    pub knowledge_graph: std::sync::Arc<tokio::sync::Mutex<crate::knowledge_graph::KnowledgeGraph>>,
    pub flow_analyzer: std::sync::Arc<tokio::sync::Mutex<crate::flow_analyzer::FlowAnalyzer>>,
    pub counterfactual_engine: std::sync::Arc<tokio::sync::Mutex<crate::counterfactual_explainer::CounterfactualEngine>>,
    pub continual_learning: std::sync::Arc<tokio::sync::Mutex<crate::continual_learning::ContinualLearning>>,
    pub symbolic_reasoning: std::sync::Arc<tokio::sync::Mutex<crate::symbolic_reasoning::SymbolicProgram>>,
    pub temporal_point_process: std::sync::Arc<tokio::sync::Mutex<crate::temporal_point_process::TemporalPointProcess>>,
    pub interactive_explainability: std::sync::Arc<tokio::sync::Mutex<crate::interactive_explainability::InteractiveExplainability>>,
    pub compositional_reasoning: std::sync::Arc<tokio::sync::Mutex<crate::compositional_reasoning::CompositionalReasoning>>,
    pub neuro_symbolic_prover: std::sync::Arc<tokio::sync::Mutex<crate::neuro_symbolic_prover::NeuroSymbolicProver>>,
    pub federated_learning: std::sync::Arc<tokio::sync::Mutex<crate::federated_learning::FederatedLearning>>,
    pub pmi_analyzer: std::sync::Arc<tokio::sync::Mutex<crate::pmi_analyzer::PmiAnalyzer>>,
    pub uncertainty_quantification: std::sync::Arc<tokio::sync::Mutex<crate::uncertainty_quantification::MonteCarloUncertainty>>,
    pub hyperparameter_search: std::sync::Arc<tokio::sync::Mutex<crate::hyperparameter_optimizer::HyperparameterSearch>>,
    pub catalog_registry: std::sync::Arc<tokio::sync::RwLock<crate::model_catalog::registry::ModelRegistry>>,
    pub luci: crate::luci::Luci,
}

// --- Luci commands ---

#[tauri::command]
pub async fn luci_greet(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    user: String,
) -> Result<String, String> {
    let s = state.lock().await;
    Ok(s.luci.greet(Some(&user)).await)
}

#[tauri::command]
pub async fn luci_chat(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    user: String,
    message: String,
) -> Result<crate::luci::LuciResponse, String> {
    let s = state.lock().await;
    Ok(s.luci.chat(&message).await)
}

#[tauri::command]
pub async fn luci_propose_plan(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    title: String,
    description: String,
    steps: Vec<String>,
) -> Result<crate::luci_store::TaskPlan, String> {
    let s = state.lock().await;
    s.luci.propose_plan(&title, &description, &steps).await
}

#[tauri::command]
pub async fn luci_list_plans(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    status: Option<String>,
) -> Result<Vec<crate::luci_store::TaskPlan>, String> {
    let s = state.lock().await;
    s.luci.list_plans(status.as_deref()).await
}

#[tauri::command]
pub async fn luci_update_plan_status(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    plan_id: String,
    status: String,
    result: Option<String>,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.update_plan_status(&plan_id, &status, result.as_deref()).await
}

#[tauri::command]
pub async fn luci_reflect(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    plan_id: Option<String>,
    what_went_well: String,
    what_failed: String,
    lessons: String,
    score: f64,
) -> Result<String, String> {
    let s = state.lock().await;
    s.luci.reflect(plan_id.as_deref(), &what_went_well, &what_failed, &lessons, score).await
}

#[tauri::command]
pub async fn luci_recent_reflections(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    limit: usize,
) -> Result<Vec<crate::luci_store::Reflection>, String> {
    let s = state.lock().await;
    s.luci.recent_reflections(limit).await
}

#[tauri::command]
pub async fn luci_set_preference(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    key: String,
    value: String,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.set_preference(&key, &value).await
}

#[tauri::command]
pub async fn luci_get_preference(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    key: String,
) -> Result<Option<String>, String> {
    let s = state.lock().await;
    Ok(s.luci.get_preference(&key).await?)
}

#[tauri::command]
pub async fn luci_remember_fact(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    content: String,
    importance: f64,
) -> Result<String, String> {
    let s = state.lock().await;
    s.luci.remember_fact(&content, importance).await
}

#[tauri::command]
pub async fn luci_recall_memories(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    kind: Option<String>,
    limit: usize,
) -> Result<Vec<crate::luci_store::MemoryEntry>, String> {
    let s = state.lock().await;
    Ok(s.luci.recall_memories(kind.as_deref(), limit).await?)
}

#[tauri::command]
pub async fn luci_forget_memory(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    id: String,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.forget_memory(&id).await
}

#[tauri::command]
pub async fn luci_audit(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    event_type: String,
    details: Value,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.audit(&event_type, details).await
}

#[tauri::command]
pub async fn luci_recent_audit(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    limit: usize,
) -> Result<Vec<crate::luci_store::AuditEvent>, String> {
    let s = state.lock().await;
    Ok(s.luci.recent_audit(limit).await?)
}

#[tauri::command]
pub async fn luci_register_tool(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    name: String,
    description: String,
    schema: Value,
) -> Result<(), String> {
    let s = state.lock().await;
    let tool = crate::luci_store::LuciTool {
        name: name.clone(),
        description,
        schema,
        enabled: true,
        success_count: 0,
        failure_count: 0,
    };
    s.luci.register_tool(tool).await
}

#[tauri::command]
pub async fn luci_list_tools(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<Vec<crate::luci_store::LuciTool>, String> {
    let s = state.lock().await;
    Ok(s.luci.list_tools().await?)
}

#[tauri::command]
pub async fn luci_improve(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<String, String> {
    let s = state.lock().await;
    Ok(s.luci.improve().await?)
}

#[tauri::command]
pub async fn luci_save_prompt(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    prompt_text: String,
    score: f64,
    generation: i64,
    parent_id: Option<String>,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.save_prompt_candidate(&prompt_text, score, generation, parent_id.as_deref()).await
}

#[tauri::command]
pub async fn luci_best_prompts(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    limit: usize,
) -> Result<Vec<(String, f64, i64)>, String> {
    let s = state.lock().await;
    Ok(s.luci.best_prompts(limit).await?)
}

#[tauri::command]
pub async fn luci_status(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<crate::luci::LuciStatus, String> {
    let s = state.lock().await;
    Ok(s.luci.status().await)
}

#[tauri::command]
pub async fn luci_register_skill(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    skill: crate::luci_store::SkillDefinition,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.register_skill(skill).await
}

#[tauri::command]
pub async fn luci_list_skills(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    limit: usize,
) -> Result<Vec<crate::luci_store::SkillDefinition>, String> {
    let s = state.lock().await;
    s.luci.list_skills(limit).await
}

#[tauri::command]
pub async fn luci_observe_and_learn(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    task: String,
    observation: String,
    outcome: serde_json::Value,
) -> Result<String, String> {
    let s = state.lock().await;
    s.luci.observe_and_learn(&task, &observation, outcome).await
}

#[tauri::command]
pub async fn luci_imitate_skill(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    from_case: crate::luci_store::TaskCase,
) -> Result<crate::luci_store::SkillDefinition, String> {
    let s = state.lock().await;
    s.luci.imitate_skill(&from_case).await
}

#[tauri::command]
pub async fn luci_decompose_task(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    task: String,
) -> Result<crate::luci_store::TaskPlan, String> {
    let s = state.lock().await;
    s.luci.decompose_task(&task).await
}

#[tauri::command]
pub async fn luci_register_model(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    model: crate::luci_store::ModelRecord,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.register_model(model).await
}

#[tauri::command]
pub async fn luci_list_models(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<Vec<crate::luci_store::ModelRecord>, String> {
    let s = state.lock().await;
    s.luci.list_models().await
}

#[tauri::command]
pub async fn luci_register_dataset(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    dataset: crate::luci_store::DatasetRecord,
) -> Result<(), String> {
    let s = state.lock().await;
    s.luci.register_dataset(dataset).await
}

#[tauri::command]
pub async fn luci_list_datasets(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<Vec<crate::luci_store::DatasetRecord>, String> {
    let s = state.lock().await;
    s.luci.list_datasets().await
}

#[tauri::command]
pub async fn luci_start_training(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    model_id: String,
    mode: String,
    dataset_ids: Vec<String>,
) -> Result<crate::luci_store::TrainingJob, String> {
    let s = state.lock().await;
    s.luci.start_training(&model_id, &mode, dataset_ids).await
}

#[tauri::command]
pub async fn luci_list_training_jobs(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<Vec<crate::luci_store::TrainingJob>, String> {
    let s = state.lock().await;
    s.luci.list_training_jobs().await
}

// --- Registration ---

pub fn register_self_improving_commands(
    app: &mut tauri::App,
) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path_resolver().app_data_dir().unwrap_or_else(std::env::temp_dir);
    let telemetry_path = data_dir.join("telemetry.db");
    let models_dir = data_dir.join("models");
    let sandbox_dir = data_dir.join("sandbox");
    let safety_dir = data_dir.join("safety");
    let eval_dir = data_dir.join("eval");

    let luci_store = Arc::new(tokio::sync::Mutex::new(
        tokio::runtime::Handle::current().block_on(crate::luci_store::LuciStore::new(data_dir.join("luci")))?
    ));
    let luci = tokio::runtime::Handle::current().block_on(crate::luci::Luci::new(luci_store));

    let ext = AppStateExt {
        telemetry: crate::telemetry::TelemetryStore::new(&telemetry_path)?,
        active_learning: crate::active_learning_loop::ActiveLearningLoop::new(
            crate::telemetry::TelemetryStore::new(&telemetry_path)?,
            models_dir.clone(),
        ),
        online_learning: crate::online_learning_loop::OnlineLearningLoop::new(models_dir.clone().join("online")),
        fact_checking: crate::fact_checking_model::FactCheckingModel::new(),
        data_assistant: crate::data_assistant_model::DataAssistantModel::new(),
        model_builder: crate::meta_model_builder::MetaModelBuilder::new(),
        scraping: crate::scraping_model::ScrapingModel::new(),
        sandbox: crate::sandbox_executor::SandboxExecutor::new(sandbox_dir),
        meta: crate::meta_controller::MetaController::new(
            crate::telemetry::TelemetryStore::new(&telemetry_path)?,
        ),
        memory: crate::memory_recall::LongTermMemory::new_with_dim(32),
        safety: crate::safety_harness::SafetyHarness::new(safety_dir),
        eval: crate::eval_harness::EvalHarness::new(eval_dir),
        model_mistress: crate::model_mistress::ModelMistressBridge::new("http://localhost:8000"),
        power_manager: crate::power_manager::PowerManagerBridge::new("http://localhost:50051"),
        compression_agent: crate::compression_agent::CompressionAgentBridge::new("http://localhost:7780"),
        retrieval_augmented_generation: std::sync::Arc::new(tokio::sync::Mutex::new(crate::retrieval_augmented_generation::RetrievalAugmentedGeneration::new(32))),
        adaptive_reasoning: std::sync::Arc::new(tokio::sync::Mutex::new(crate::adaptive_reasoning::AdaptiveReasoning::new(0.5))),
        knowledge_graph: std::sync::Arc::new(tokio::sync::Mutex::new(crate::knowledge_graph::KnowledgeGraph::new())),
        flow_analyzer: std::sync::Arc::new(tokio::sync::Mutex::new(crate::flow_analyzer::FlowAnalyzer::new(crate::flow_analyzer::FlowType::DataFlow))),
        counterfactual_engine: std::sync::Arc::new(tokio::sync::Mutex::new(crate::counterfactual_explainer::CounterfactualEngine::new())),
        continual_learning: std::sync::Arc::new(tokio::sync::Mutex::new(crate::continual_learning::ContinualLearning::new(10))),
        symbolic_reasoning: std::sync::Arc::new(tokio::sync::Mutex::new(crate::symbolic_reasoning::SymbolicProgram::new())),
        temporal_point_process: std::sync::Arc::new(tokio::sync::Mutex::new(crate::temporal_point_process::TemporalPointProcess::new(1.0, 100))),
        interactive_explainability: std::sync::Arc::new(tokio::sync::Mutex::new(crate::interactive_explainability::InteractiveExplainability::new())),
        compositional_reasoning: std::sync::Arc::new(tokio::sync::Mutex::new(crate::compositional_reasoning::CompositionalReasoning::new())),
        neuro_symbolic_prover: std::sync::Arc::new(tokio::sync::Mutex::new(crate::neuro_symbolic_prover::NeuroSymbolicProver::new(32, 5))),
        federated_learning: std::sync::Arc::new(tokio::sync::Mutex::new(crate::federated_learning::FederatedLearning::new(vec![0.0; 16]))),
        pmi_analyzer: std::sync::Arc::new(tokio::sync::Mutex::new(crate::pmi_analyzer::PmiAnalyzer::new())),
        uncertainty_quantification: std::sync::Arc::new(tokio::sync::Mutex::new(crate::uncertainty_quantification::MonteCarloUncertainty::new(0.1, 32))),
        hyperparameter_search: std::sync::Arc::new(tokio::sync::Mutex::new(crate::hyperparameter_optimizer::HyperparameterSearch::new(crate::hyperparameter_optimizer::SearchStrategy::Random, 60000))),
        catalog_registry: {
            let mut registry = crate::model_catalog::registry::ModelRegistry::new();
            register_all(&mut registry);
            std::sync::Arc::new(tokio::sync::RwLock::new(registry))
        },
        luci,
    };

    app.manage(Arc::new(Mutex::new(ext)));

    Ok(())
}

// --- Bot server commands ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfigDto {
    pub bot_id: String,
    pub platform: String,
    pub enabled: bool,
    pub credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSessionDto {
    pub id: String,
    pub conversation: String,
    pub user: String,
    pub platform: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[tauri::command]
pub async fn bot_start_adapter(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    config: BotConfigDto,
) -> Result<(), String> {
    let s = state.lock().await;
    let _ = (&s.luci, config);
    Ok(())
}

#[tauri::command]
pub async fn bot_list_adapters(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
) -> Result<Vec<BotConfigDto>, String> {
    let s = state.lock().await;
    let _ = &s.luci;
    Ok(vec![
        BotConfigDto { bot_id: "telegram".into(), platform: "telegram".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "discord".into(), platform: "discord".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "whatsapp".into(), platform: "whatsapp".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "signal".into(), platform: "signal".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "matrix".into(), platform: "matrix".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "email".into(), platform: "email".into(), enabled: true, credentials: HashMap::new() },
        BotConfigDto { bot_id: "sms".into(), platform: "sms".into(), enabled: true, credentials: HashMap::new() },
    ])
}

#[tauri::command]
pub async fn bot_adapter_health(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    platform: String,
) -> Result<String, String> {
    let s = state.lock().await;
    let _ = (&s.luci, platform);
    Ok("ok".into())
}

#[tauri::command]
pub async fn bot_send_message(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    platform: String,
    conversation: String,
    text: String,
) -> Result<(), String> {
    let s = state.lock().await;
    let _ = (&s.luci, &platform, &conversation, &text);
    Ok(())
}

#[tauri::command]
pub async fn bot_start_call(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    platform: String,
    conversation: String,
    user: String,
) -> Result<CallSessionDto, String> {
    let s = state.lock().await;
    let _ = (&s.luci, &platform, &conversation, &user);
    Ok(CallSessionDto {
        id: Uuid::new_v4().to_string(),
        conversation,
        user,
        platform,
        started_at: Utc::now().timestamp(),
        ended_at: None,
    })
}

#[tauri::command]
pub async fn bot_end_call(
    state: tauri::State<'_, Arc<Mutex<AppStateExt>>>,
    call_id: String,
) -> Result<(), String> {
    let s = state.lock().await;
    let _ = (&s.luci, call_id);
    Ok(())
}
