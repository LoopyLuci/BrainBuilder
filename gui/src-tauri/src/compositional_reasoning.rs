#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct CompositionalReasoning {
    pub primitives: Vec<PrimitiveSkill>,
    pub programs: Vec<Composition>,
    pub execution_trace: VecDeque<TraceStep>,
}

#[derive(Debug, Clone)]
pub struct PrimitiveSkill {
    pub name: String,
    pub signature: String,
    pub deterministic: bool,
}

#[derive(Debug, Clone)]
pub struct Composition {
    pub name: String,
    pub steps: Vec<String>,
    pub repeatable: bool,
}

#[derive(Debug, Clone)]
pub struct TraceStep {
    pub step: usize,
    pub primitive: String,
    pub input: String,
    pub output: String,
    pub duration_ms: u64,
}

impl CompositionalReasoning {
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            programs: Vec::new(),
            execution_trace: VecDeque::with_capacity(128),
        }
    }

    pub fn register_primitive(&mut self, name: impl Into<String>, signature: impl Into<String>) {
        self.primitives.push(PrimitiveSkill {
            name: name.into(),
            signature: signature.into(),
            deterministic: true,
        });
    }

    pub fn compose(&mut self, name: impl Into<String>, steps: Vec<impl Into<String>>, repeatable: bool) {
        self.programs.push(Composition {
            name: name.into(),
            steps: steps.into_iter().map(Into::into).collect(),
            repeatable,
        });
    }

    pub fn execute_program(&mut self, program_name: &str, input: impl Into<String>) -> Result<VecDeque<TraceStep>, String> {
        let program = self.programs.iter()
            .find(|p| p.name == program_name)
            .ok_or_else(|| format!("program not found: {}", program_name))?;

        let mut trace = VecDeque::new();
        let mut current_input = input.into();

        for (i, primitive_name) in program.steps.iter().enumerate() {
            let primitive = self.primitives.iter()
                .find(|p| p.name == *primitive_name)
                .ok_or_else(|| format!("primitive not found: {}", primitive_name))?;

            let output = if primitive.deterministic {
                format!("{}_out", primitive_name)
            } else {
                format!("{}_stochastic", primitive_name)
            };

            let step = TraceStep {
                step: i,
                primitive: primitive_name.clone(),
                input: current_input.clone(),
                output: output.clone(),
                duration_ms: 1,
            };
            trace.push_back(step);
            current_input = output;
        }

        self.execution_trace.extend(trace.clone());
        Ok(trace)
    }

    pub fn programs(&self) -> &[Composition] {
        &self.programs
    }

    pub fn primitives(&self) -> &[PrimitiveSkill] {
        &self.primitives
    }
}

#[tauri::command]
pub async fn compositional_compose(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, name: String, steps: Vec<String>, repeatable: bool) -> Result<(), String> {
    let s = state.lock().await;
    let mut model = s.compositional_reasoning.lock().await;
    model.compose(name, steps, repeatable);
    Ok(())
}

#[tauri::command]
pub async fn compositional_execute(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, program_name: String, input: String) -> Result<VecDeque<TraceStep>, String> {
    let s = state.lock().await;
    let mut model = s.compositional_reasoning.lock().await;
    model.execute_program(&program_name, input)
}
