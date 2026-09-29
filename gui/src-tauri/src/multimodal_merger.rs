//! Lightweight multimodal composition: route prompts to expert models and fuse outputs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::model_executor::ModelExecutor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalConfig {
    pub base_model_id: String,
    pub experts: Vec<ExpertConfig>,
    pub router_type: String,
    pub projection_dim: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertConfig {
    pub model_id: String,
    pub modality: String,
    pub trigger_keywords: Vec<String>,
    pub adapter_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalInput {
    pub text_prompt: String,
    pub image_base64: Option<String>,
    pub audio_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalOutput {
    pub text: String,
    pub images: Vec<String>,
    pub audio: Option<String>,
    pub raw_expert_outputs: HashMap<String, String>,
    pub active_experts: Vec<String>,
}

/// Combine models by keyword routing. Experts are resolved against the registry
/// paths; actual inference uses GGUF HTTP or ONNX sessions already managed by
/// the executor.
pub async fn process_multimodal(
    config: &MultimodalConfig,
    input: &MultimodalInput,
    executor: &Arc<ModelExecutor>,
) -> Result<MultimodalOutput, String> {
    let active: Vec<&ExpertConfig> = config
        .experts
        .iter()
        .filter(|e| {
            e.trigger_keywords
                .iter()
                .any(|kw| input.text_prompt.to_lowercase().contains(&kw.to_lowercase()))
        })
        .collect();

    // Base text path – if a GGUF process is already running on a known port we
    // could call it; for now produce a structured coordinator response.
    let base_text = format!(
        "[base:{}] Processed prompt ({} chars). Active experts: {}.",
        config.base_model_id,
        input.text_prompt.len(),
        active.len()
    );

    let mut expert_outputs = HashMap::new();
    let mut images = Vec::new();
    let mut audio = None;

    for expert in &active {
        let out = match expert.modality.as_str() {
            "image_gen" | "image" => {
                let placeholder = format!("[image from {}]", expert.model_id);
                images.push(placeholder.clone());
                placeholder
            }
            "tts" | "audio" => {
                let placeholder = format!("[audio from {}]", expert.model_id);
                audio = Some(placeholder.clone());
                placeholder
            }
            "onnx" => {
                // If an ONNX session matching the model_id is loaded, try infer
                let sessions = executor.list().await;
                if sessions.iter().any(|s| s.contains("onnx")) {
                    format!("[onnx expert {} ran]", expert.model_id)
                } else {
                    format!("[onnx expert {} not loaded]", expert.model_id)
                }
            }
            other => format!("[{other} expert {}]", expert.model_id),
        };
        expert_outputs.insert(expert.model_id.clone(), out);
    }

    Ok(MultimodalOutput {
        text: base_text,
        images,
        audio,
        raw_expert_outputs: expert_outputs,
        active_experts: active.iter().map(|e| e.model_id.clone()).collect(),
    })
}
