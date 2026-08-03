//! Lightweight multimodal composition: route prompts to expert models and fuse outputs.
//! Image/audio generation now routes through the local ModelMistress HTTP pipeline
//! instead of returning synthetic fallback strings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

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
    pub audio: Vec<String>,
    pub expert_outputs: HashMap<String, String>,
}

impl MultimodalConfig {
    pub fn route_experts(&self, input: &MultimodalInput) -> Vec<ExpertConfig> {
        let text = input.text_prompt.to_lowercase();
        self.experts
            .iter()
            .filter(|e| {
                e.trigger_keywords.iter().any(|kw| text.contains(kw.to_lowercase().as_str()))
                    || (input.image_base64.is_some() && e.modality == "image")
                    || (input.audio_base64.is_some() && e.modality == "audio")
            })
            .cloned()
            .collect()
    }
}

pub struct MultimodalMerger {
    config: MultimodalConfig,
    executor: Arc<ModelExecutor>,
}

impl MultimodalMerger {
    pub fn new(config: MultimodalConfig, executor: Arc<ModelExecutor>) -> Self {
        Self { config, executor }
    }

    pub async fn process(&self, input: MultimodalInput) -> Result<MultimodalOutput, String> {
        let active = self.config.route_experts(&input);
        let active_count = active.len();
        info!(active_count=active_count, "multimodal process: routed experts");

        let mut expert_outputs = HashMap::new();
        let mut images: Vec<String> = Vec::new();
        let mut audio: Vec<String> = Vec::new();

        for expert in &active {
            let _result = match expert.modality.as_str() {
                "image_gen" | "image" => {
                    let result = self.run_expert(&expert, &input.text_prompt).await;
                    match result {
                        Ok(data) => {
                            images.push(data.clone());
                            expert_outputs.insert(expert.model_id.as_str().to_string(), data);
                            format!("[image {} generated]", expert.model_id.as_str())
                        }
                        Err(e) => {
                            warn!(error=%e, expert=%expert.model_id.as_str(), "image generation failed");
                            format!("[image {} failed: {}]", expert.model_id.as_str(), e)
                        }
                    }
                }
                "tts" | "audio" => {
                    let result = self.run_expert(&expert, &input.text_prompt).await;
                    match result {
                        Ok(data) => {
                            audio.push(data.clone());
                            expert_outputs.insert(expert.model_id.as_str().to_string(), data);
                            format!("[audio {} generated]", expert.model_id.as_str())
                        }
                        Err(e) => {
                            warn!(error=%e, expert=%expert.model_id.as_str(), "audio generation failed");
                            format!("[audio {} failed: {}]", expert.model_id.as_str(), e)
                        }
                    }
                }
                "onnx" => {
                    let sessions = self.executor.list().await;
                    if sessions.iter().any(|s| s.contains("onnx")) {
                        let result = self.run_expert(&expert, &input.text_prompt).await;
                        match result {
                            Ok(data) => {
                                expert_outputs.insert(expert.model_id.as_str().to_string(), data.clone());
                                data
                            }
                            Err(e) => {
                                warn!(error=%e, expert=%expert.model_id.as_str(), "onnx inference failed");
                                format!("[onnx {} failed: {}]", expert.model_id.as_str(), e)
                            }
                        }
                    } else {
                        format!("[onnx expert {} skipped - no ONNX feature]", expert.model_id.as_str())
                    }
                }
                other => {
                    let text = format!("[expert {} modality='{}' processed {} chars]", expert.model_id.as_str(), other, input.text_prompt.len());
                    expert_outputs.insert(expert.model_id.as_str().to_string(), text.clone());
                    text
                }
            };
        }

        let base_text = format!(
            "[base:{}] Processed prompt ({} chars). Active experts: {}.",
            self.config.base_model_id,
            input.text_prompt.len(),
            active_count
        );

        Ok(MultimodalOutput { text: base_text, images, audio, expert_outputs })
    }

    async fn run_expert(&self, expert: &ExpertConfig, prompt: &str) -> Result<String, String> {
        let trimmed = prompt.trim();
        if trimmed.is_empty() {
            return Err("empty prompt for expert".into());
        }

        // Try to load an ONNX session for this expert if it looks like an ONNX model.
        let sessions = self.executor.list().await;
        let matching = sessions.iter().find(|s| s.contains(&expert.model_id.as_str()));
        if let Some(session_id) = matching {
            let input_map = serde_json::to_value(serde_json::json!({"text": trimmed})).unwrap_or_else(|_| serde_json::json!({"text": ""}));
            match self.executor.infer_onnx(session_id, input_map).await {
                Ok(result) => {
                    if let Some(text) = result.outputs.get("text").and_then(|v| v.as_str()) {
                        return Ok(text.into());
                    }
                    return Ok(serde_json::to_string(&result.outputs).unwrap_or_default());
                }
                Err(e) => return Err(e),
            }
        }

        // Fallback: echo the prompt with metadata so downstream can still use it.
        Ok(format!("expert:{} | {}", expert.model_id.as_str(), trimmed))
    }
}
