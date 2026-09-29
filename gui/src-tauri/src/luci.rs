use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::luci_store::LuciStore;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LuciMood {
    Cheerful,
    Curious,
    Focused,
    Concerned,
    Sleepy,
    Overwhelmed,
}

impl LuciMood {
    fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "cheerful" => Some(Self::Cheerful),
            "curious" => Some(Self::Curious),
            "focused" => Some(Self::Focused),
            "concerned" => Some(Self::Concerned),
            "sleepy" => Some(Self::Sleepy),
            "overwhelmed" => Some(Self::Overwhelmed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciPersonality {
    pub name: String,
    pub archetype: String,
    pub tone: String,
    pub default_mood: LuciMood,
    pub values: Vec<String>,
}

impl LuciPersonality {
    pub fn default() -> Self {
        Self {
            name: "Luci".to_string(),
            archetype: "sweet, fun-loving AI companion".to_string(),
            tone: "warm, playful, precise when needed".to_string(),
            default_mood: LuciMood::Cheerful,
            values: vec![
                "honesty".into(),
                "helpfulness".into(),
                "curiosity".into(),
                "safety".into(),
                "humility".into(),
            ],
        }
    }

    fn mood_from_str(value: &str) -> LuciMood {
        match value.to_ascii_lowercase().as_str() {
            "cheerful" => LuciMood::Cheerful,
            "curious" => LuciMood::Curious,
            "focused" => LuciMood::Focused,
            "concerned" => LuciMood::Concerned,
            "sleepy" => LuciMood::Sleepy,
            "overwhelmed" => LuciMood::Overwhelmed,
            _ => LuciMood::Cheerful,
        }
    }

    fn apply_disk_overrides(&mut self, overrides: &LuciPersonality) {
        if !overrides.name.is_empty() {
            self.name = overrides.name.clone();
        }
        if !overrides.archetype.is_empty() {
            self.archetype = overrides.archetype.clone();
        }
        if !overrides.tone.is_empty() {
            self.tone = overrides.tone.clone();
        }
        if !overrides.values.is_empty() {
            self.values = overrides.values.clone();
        }
    }

    fn load_personality_from_path(path: std::path::PathBuf) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        let overrides: Self = serde_json::from_str(&contents).ok()?;
        Some(overrides)
    }

    pub fn load_builtin() -> Self {
        let mut personality = Self::default();
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let candidates = [
            manifest_dir.join("../../models/built-in/luci/personality.json"),
            manifest_dir.join("../models/built-in/luci/personality.json"),
            manifest_dir.join("models/built-in/luci/personality.json"),
        ];
        for path in candidates {
            if let Some(overrides) = Self::load_personality_from_path(path) {
                personality.apply_disk_overrides(&overrides);
                return personality;
            }
        }
        personality
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub mood: Option<LuciMood>,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciContext {
    pub user_id: Option<String>,
    pub session_id: String,
    pub active_plan_id: Option<String>,
    pub current_task: Option<String>,
    pub recent_tool_calls: VecDeque<String>,
    pub last_user_emotion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciState {
    pub personality: LuciPersonality,
    pub context: LuciContext,
    pub mood: LuciMood,
    pub confidence: f64,
    pub conversation_history: VecDeque<ConversationTurn>,
    pub self_model: HashMap<String, f64>,
    pub pending_reflections: VecDeque<crate::luci_store::Reflection>,
}

impl Default for LuciState {
    fn default() -> Self {
        Self {
            personality: LuciPersonality::load_builtin(),
            context: LuciContext {
                user_id: None,
                session_id: Uuid::new_v4().to_string(),
                active_plan_id: None,
                current_task: None,
                recent_tool_calls: VecDeque::new(),
                last_user_emotion: None,
            },
            mood: LuciPersonality::load_builtin().default_mood,
            confidence: 0.8,
            conversation_history: VecDeque::with_capacity(64),
            self_model: HashMap::new(),
            pending_reflections: VecDeque::with_capacity(8),
        }
    }
}

pub struct Luci {
    pub state: Mutex<LuciState>,
    pub store: Arc<Mutex<LuciStore>>,
    pub tools: Mutex<HashMap<String, crate::luci_store::LuciTool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciResponse {
    pub message: String,
    pub mood: LuciMood,
    pub confidence: f64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciStatus {
    pub name: String,
    pub mood: LuciMood,
    pub confidence: f64,
    pub active_plan_id: Option<String>,
    pub conversation_turn_count: usize,
    pub tool_count: usize,
    pub memory_count: usize,
}

impl Luci {
    pub async fn new(store: Arc<Mutex<LuciStore>>) -> Self {
        let state = Mutex::new(LuciState::default());
        let tools = Mutex::new(HashMap::new());
        Self { state, store, tools }
    }

    pub async fn greet(&self, user_name: Option<&str>) -> String {
        let mut s = self.state.lock().await;
        let greeting = match user_name {
            Some(name) => format!(
                "Hey {name}! Luci here — ready to build, learn, \
                 and maybe break a few things (safely). \
                 What shall we do today?"
            ),
            None => {
                "Hey! Luci here — your sweet, slightly chaotic AI \
                 companion. What are we building today?"
                    .to_string()
            }
        };
        let mood = s.mood.clone();
        s.conversation_history.push_back(ConversationTurn {
            role: "user".to_string(),
            content: greeting.clone(),
            mood: Some(mood),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        });
        greeting
    }

    pub async fn chat(&self, user_message: &str) -> LuciResponse {
        let mut s = self.state.lock().await;
        let current_mood = s.mood.clone();
        s.conversation_history.push_back(ConversationTurn {
            role: "user".to_string(),
            content: user_message.to_string(),
            mood: Some(current_mood.clone()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        });

        let (reply, mood_shift) = self.generate_reply(current_mood, user_message).await;
        s.mood = mood_shift.clone();
        s.conversation_history.push_back(ConversationTurn {
            role: "assistant".to_string(),
            content: reply.clone(),
            mood: Some(mood_shift),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        });

        while s.conversation_history.len() > 64 {
            s.conversation_history.pop_front();
        }

        LuciResponse {
            message: reply,
            mood: s.mood.clone(),
            confidence: s.confidence,
            suggestions: vec![],
        }
    }

    async fn generate_reply(&self, mood: LuciMood, user_message: &str) -> (String, LuciMood) {
        let lowered = user_message.to_lowercase();
        let mood = if lowered.contains("tired") || lowered.contains("overwhelm") {
            LuciMood::Concerned
        } else if lowered.contains("hello") || lowered.contains("hi") || lowered.contains("hey") {
            LuciMood::Cheerful
        } else if lowered.contains("learn") || lowered.contains("teach") || lowered.contains("why") {
            LuciMood::Curious
        } else if lowered.contains("focus") || lowered.contains("plan") || lowered.contains("build") {
            LuciMood::Focused
        } else {
            mood
        };

        let reply = if lowered.contains("tired") || lowered.contains("exhausted") || lowered.contains("overwhelm") {
            "I can tell you’re carrying a lot right now. \
             Let’s take it one micro-step at a time: \
             what’s the smallest next action you can stomach? \
             I’ll handle the rest."
                .to_string()
        } else if lowered.contains("hello") || lowered.contains("hi") || lowered.contains("hey") {
            "Hey there! I’m Luci — part researcher, part builder, \
             part chaos gremlin. What shall we create today?"
                .to_string()
        } else if lowered.contains("learn") || lowered.contains("teach") || lowered.contains("why") {
            "Great instinct. Here’s what I can unpack: the concept, \
             the math, the code, and why it matters. Which layer do you want first?"
                .to_string()
        } else if lowered.contains("plan") || lowered.contains("roadmap") {
            "Let’s turn this into a step-by-step plan. \
             Give me 30 seconds to sketch phases, milestones, and failure modes."
                .to_string()
        } else if lowered.contains("code") || lowered.contains("build") || lowered.contains("implement") {
            "Builder mode activated. I’ll scaffold the structure, \
             write the first cut, and flag where we’ll need your taste."
                .to_string()
        } else {
            "Got it. I’m treating that as a first-class task: \
             I’ll break it down, pick the right tools, and keep you in the loop."
                .to_string()
        };

        (reply, mood)
    }

    pub async fn propose_plan(&self, title: &str, description: &str, steps: &[String]) -> Result<crate::luci_store::TaskPlan, String> {
        let mut s = self.state.lock().await;
        let plan = crate::luci_store::TaskPlan {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: description.to_string(),
            status: "pending".to_string(),
            steps: steps.to_vec(),
            current_step: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
            result: None,
        };
        s.context.active_plan_id = Some(plan.id.clone());
        Ok(plan)
    }

    pub async fn list_plans(&self, status: Option<&str>) -> Result<Vec<crate::luci_store::TaskPlan>, String> {
        let store = self.store.lock().await;
        let plans = store.list_plans(status).await.map_err(|e| e.to_string())?;
        Ok(plans)
    }

    pub async fn update_plan_status(
        &self,
        plan_id: &str,
        status: &str,
        result: Option<&str>,
    ) -> Result<(), String> {
        let plans = self.list_plans(None).await?;
        if let Some(mut plan) = plans.into_iter().find(|p| p.id == plan_id) {
            plan.status = status.to_string();
            plan.updated_at = Utc::now();
            if matches!(status, "completed" | "failed" | "cancelled") {
                plan.completed_at = Some(Utc::now());
            }
            plan.result = result.map(|s| s.to_string());
            let store = self.store.lock().await;
            store.save_plan(plan).await.map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub async fn reflect(
        &self,
        plan_id: Option<&str>,
        what_went_well: &str,
        what_failed: &str,
        lessons: &str,
        score: f64,
    ) -> Result<String, String> {
        let reflection = crate::luci_store::Reflection {
            id: Uuid::new_v4().to_string(),
            plan_id: plan_id.map(|s| s.to_string()),
            what_went_well: what_went_well.to_string(),
            what_failed: what_failed.to_string(),
            lessons_learned: lessons.to_string(),
            score,
            created_at: Utc::now(),
        };
        let store = self.store.lock().await;
        store.save_reflection(reflection.clone()).await.map_err(|e| e.to_string())?;
        Ok(reflection.id)
    }

    pub async fn recent_reflections(&self, limit: usize) -> Result<Vec<crate::luci_store::Reflection>, String> {
        let store = self.store.lock().await;
        let reflections = store.recent_reflections(limit).await.map_err(|e| e.to_string())?;
        Ok(reflections)
    }

    pub async fn set_preference(&self, key: &str, value: &str) -> Result<(), String> {
        let store = self.store.lock().await;
        store.set_preference(key, value).await.map_err(|e| e.to_string())
    }

    pub async fn get_preference(&self, key: &str) -> Result<Option<String>, String> {
        let store = self.store.lock().await;
        let value = store.get_preference(key).await.map_err(|e| e.to_string())?;
        Ok(value)
    }

    pub async fn remember_fact(&self, content: &str, importance: f64) -> Result<String, String> {
        let entry = crate::luci_store::MemoryEntry {
            id: Uuid::new_v4().to_string(),
            kind: "fact".to_string(),
            content: content.to_string(),
            embedding: None,
            importance,
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
        };
        let store = self.store.lock().await;
        store.remember(entry.clone()).await.map_err(|e| e.to_string())?;
        Ok(entry.id)
    }

    pub async fn recall_memories(&self, kind: Option<&str>, limit: usize) -> Result<Vec<crate::luci_store::MemoryEntry>, String> {
        let store = self.store.lock().await;
        let memories = store.recall(kind, limit).await.map_err(|e| e.to_string())?;
        Ok(memories)
    }

    pub async fn forget_memory(&self, id: &str) -> Result<(), String> {
        let store = self.store.lock().await;
        store.forget(id).await.map_err(|e| e.to_string())
    }

    pub async fn audit(&self, event_type: &str, details: serde_json::Value) -> Result<(), String> {
        let store = self.store.lock().await;
        store.audit(event_type, details).await.map_err(|e| e.to_string())
    }

    pub async fn recent_audit(&self, limit: usize) -> Result<Vec<crate::luci_store::AuditEvent>, String> {
        let store = self.store.lock().await;
        let events = store.recent_audit(limit).await.map_err(|e| e.to_string())?;
        Ok(events)
    }

    pub async fn register_tool(&self, tool: crate::luci_store::LuciTool) -> Result<(), String> {
        let name = tool.name.clone();
        let description = tool.description.clone();
        let schema = tool.schema.clone();
        {
            let mut tools = self.tools.lock().await;
            tools.insert(name.clone(), tool);
        }
        let store = self.store.lock().await;
        store.register_tool(&name, &description, schema).await.map_err(|e| e.to_string())
    }

    pub async fn list_tools(&self) -> Result<Vec<crate::luci_store::LuciTool>, String> {
        let tools = self.tools.lock().await;
        Ok(tools.values().cloned().collect())
    }

    pub async fn record_tool_result(&self, name: &str, success: bool) -> Result<(), String> {
        let mut tools = self.tools.lock().await;
        if let Some(tool) = tools.get_mut(name) {
            if success {
                tool.success_count += 1;
            } else {
                tool.failure_count += 1;
            }
        }
        Ok(())
    }

    pub async fn save_prompt_candidate(
        &self,
        prompt_text: &str,
        score: f64,
        generation: i64,
        parent_id: Option<&str>,
    ) -> Result<(), String> {
        let store = self.store.lock().await;
        store.save_prompt(prompt_text, score, generation, parent_id).await.map_err(|e| e.to_string())
    }

    pub async fn best_prompts(&self, limit: usize) -> Result<Vec<(String, f64, i64)>, String> {
        let store = self.store.lock().await;
        let prompts = store.best_prompts(limit).await.map_err(|e| e.to_string())?;
        Ok(prompts)
    }

    pub async fn improve(&self) -> Result<String, String> {
        let reflections = self.recent_reflections(10).await?;
        let best_prompts = self.best_prompts(3).await?;
        let mut notes = Vec::new();
        notes.push(format!("reviewed {} reflections", reflections.len()));
        if let Some((best, score, gen)) = best_prompts.first() {
            notes.push(format!("best prompt score={:.2} generation={}", score, gen));
        }
        let summary = notes.join("; ");
        self.audit("self_improvement", serde_json::json!({"summary": summary, "reflections": reflections.len()})).await?;
        Ok(summary)
    }

    pub async fn status(&self) -> LuciStatus {
        let s = self.state.lock().await;
        let tools = self.tools.lock().await;
        let store = self.store.lock().await;
        let memory_count = store.recall(None, 1).await.map(|v| v.len()).unwrap_or_default();
        LuciStatus {
            name: s.personality.name.clone(),
            mood: s.mood.clone(),
            confidence: s.confidence,
            active_plan_id: s.context.active_plan_id.clone(),
            conversation_turn_count: s.conversation_history.len(),
            tool_count: tools.len(),
            memory_count,
        }
    }

    // --- Skill/tool learning ---

    pub async fn register_skill(&self, skill: crate::luci_store::SkillDefinition) -> Result<(), String> {
        let name = skill.name.clone();
        let store = self.store.lock().await;
        store.save_skill(skill).await.map_err(|e| e.to_string())
    }

    pub async fn list_skills(&self, limit: usize) -> Result<Vec<crate::luci_store::SkillDefinition>, String> {
        let store = self.store.lock().await;
        store.list_skills(limit).await.map_err(|e| e.to_string())
    }

    pub async fn observe_and_learn(&self, task: &str, observation: &str, outcome: serde_json::Value) -> Result<String, String> {
        let mem_id = uuid::Uuid::new_v4().to_string();
        let entry = crate::luci_store::MemoryEntry {
            id: mem_id.clone(),
            kind: "observation".to_string(),
            content: format!("task={task}; observation={observation}; outcome={}", outcome.to_string()),
            embedding: None,
            importance: 0.7,
            created_at: Utc::now(),
            accessed_at: Utc::now(),
            access_count: 0,
        };
        let store = self.store.lock().await;
        store.remember(entry).await.map_err(|e| e.to_string())?;
        self.audit("observe_and_learn", serde_json::json!({"task": task, "observation": observation, "outcome": outcome})).await?;
        Ok(mem_id)
    }

    pub async fn imitate_skill(&self, from_case: &crate::luci_store::TaskCase) -> Result<crate::luci_store::SkillDefinition, String> {
        let skill = crate::luci_store::SkillDefinition {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("imitated:{}", from_case.title),
            description: from_case.description.clone(),
            tags: from_case.tags.clone(),
            params: from_case.output_example.clone(),
            implementation: format!("observed from case {}", from_case.id),
            source: format!("task_case:{}", from_case.id),
            confidence: 0.5,
            success_count: 0,
            failure_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let store = self.store.lock().await;
        store.save_skill(skill.clone()).await.map_err(|e| e.to_string())?;
        Ok(skill)
    }

    pub async fn decompose_task(&self, task: &str) -> Result<crate::luci_store::TaskPlan, String> {
        let steps = vec![
            format!("Clarify goal for: {}", task),
            "Find analogous task cases or skills".into(),
            "Draft execution steps".into(),
            "Execute with available tools/skills".into(),
            "Observe outcome".into(),
            "Reflect and improve".into(),
        ];
        self.propose_plan(task, "Autodecomposed from novel task", &steps).await
    }

    // --- Model training/loading ops ---

    pub async fn register_model(&self, model: crate::luci_store::ModelRecord) -> Result<(), String> {
        let store = self.store.lock().await;
        store.register_model(model).await.map_err(|e| e.to_string())
    }

    pub async fn list_models(&self) -> Result<Vec<crate::luci_store::ModelRecord>, String> {
        let store = self.store.lock().await;
        store.list_models().await.map_err(|e| e.to_string())
    }

    pub async fn register_dataset(&self, dataset: crate::luci_store::DatasetRecord) -> Result<(), String> {
        let store = self.store.lock().await;
        store.register_dataset(dataset).await.map_err(|e| e.to_string())
    }

    pub async fn list_datasets(&self) -> Result<Vec<crate::luci_store::DatasetRecord>, String> {
        let store = self.store.lock().await;
        store.list_datasets().await.map_err(|e| e.to_string())
    }

    pub async fn start_training(&self, model_id: &str, mode: &str, dataset_ids: Vec<String>) -> Result<crate::luci_store::TrainingJob, String> {
        let job = crate::luci_store::TrainingJob {
            id: uuid::Uuid::new_v4().to_string(),
            model_id: model_id.into(),
            mode: mode.into(),
            dataset_ids,
            status: "queued".into(),
            metrics: serde_json::Value::Null,
            artifact_path: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let store = self.store.lock().await;
        store.save_training_job(job.clone()).await.map_err(|e| e.to_string())?;
        Ok(job)
    }

    pub async fn list_training_jobs(&self) -> Result<Vec<crate::luci_store::TrainingJob>, String> {
        let store = self.store.lock().await;
        store.list_training_jobs().await.map_err(|e| e.to_string())
    }
}

