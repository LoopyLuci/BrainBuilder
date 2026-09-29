#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct SymbolicProgram {
    pub steps: Vec<SymbolicStep>,
    pub variables: HashMap<String, SymbolicValue>,
}

#[derive(Debug, Clone)]
pub struct SymbolicStep {
    pub operation: String,
    pub args: Vec<SymbolicValue>,
    pub result: Option<SymbolicValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SymbolicValue {
    Number(f64),
    Text(String),
    Bool(bool),
    List(Vec<SymbolicValue>),
}

impl std::fmt::Display for SymbolicValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolicValue::Number(n) => write!(f, "{}", n),
            SymbolicValue::Text(t) => write!(f, "\"{}\"", t),
            SymbolicValue::Bool(b) => write!(f, "{}", b),
            SymbolicValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl SymbolicProgram {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            variables: HashMap::new(),
        }
    }

    pub fn add_step(&mut self, operation: impl Into<String>, args: Vec<SymbolicValue>) {
        self.steps.push(SymbolicStep {
            operation: operation.into(),
            args,
            result: None,
        });
    }

    pub fn set_variable(&mut self, name: impl Into<String>, value: SymbolicValue) {
        self.variables.insert(name.into(), value);
    }

    pub fn execute(&mut self) -> Result<VecDeque<SymbolicValue>, String> {
        let ops: Vec<(String, Vec<SymbolicValue>)> = self.steps.iter()
            .map(|step| (step.operation.clone(), step.args.clone()))
            .collect();
        let mut outputs = VecDeque::new();
        for (i, (operation, args)) in ops.into_iter().enumerate() {
            let result = self.evaluate_op(&operation, &args)?;
            if let Some(step) = self.steps.get_mut(i) {
                step.result = Some(result.clone());
            }
            outputs.push_back(result);
        }
        Ok(outputs)
    }

    fn evaluate_op(&self, op: &str, args: &[SymbolicValue]) -> Result<SymbolicValue, String> {
        match op {
            "add" => match (args.get(0), args.get(1)) {
                (Some(SymbolicValue::Number(a)), Some(SymbolicValue::Number(b))) => Ok(SymbolicValue::Number(a + b)),
                _ => Err("add expects two numbers".to_string()),
            },
            "concat" => {
                let parts: Vec<String> = args.iter().map(|v| match v {
                    SymbolicValue::Text(t) => t.clone(),
                    SymbolicValue::Number(n) => n.to_string(),
                    SymbolicValue::Bool(b) => b.to_string(),
                    SymbolicValue::List(_) => "[list]".to_string(),
                }).collect();
                Ok(SymbolicValue::Text(parts.join("")))
            }
            "length" => match args.get(0) {
                Some(SymbolicValue::List(items)) => Ok(SymbolicValue::Number(items.len() as f64)),
                Some(SymbolicValue::Text(t)) => Ok(SymbolicValue::Number(t.len() as f64)),
                _ => Err("length expects list or text".to_string()),
            },
            "contains" => match (args.get(0), args.get(1)) {
                (Some(SymbolicValue::Text(haystack)), Some(SymbolicValue::Text(needle))) => Ok(SymbolicValue::Bool(haystack.contains(needle))),
                _ => Err("contains expects two text values".to_string()),
            },
            _ => Err(format!("unknown operation: {}", op)),
        }
    }
}

#[tauri::command]
pub async fn symbolic_execute(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, program_json: String) -> Result<VecDeque<SymbolicValue>, String> {
    let s = state.lock().await;
    let mut program = s.symbolic_reasoning.lock().await;
    program.execute()
}

#[tauri::command]
pub async fn symbolic_add_step(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, operation: String, args_json: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut program = s.symbolic_reasoning.lock().await;
    let args: Vec<SymbolicValue> = serde_json::from_str(&args_json).map_err(|e| e.to_string())?;
    program.add_step(operation, args);
    Ok(())
}
