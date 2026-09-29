//! Dataset labeling helpers and augmentation via Python.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetEntry {
    pub id: String,
    pub text: Option<String>,
    pub image_path: Option<String>,
    pub audio_path: Option<String>,
    pub labels: Vec<Label>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub value: serde_json::Value,
}

pub async fn augment_dataset(dataset_path: &str, methods: &[String]) -> Result<String, String> {
    let script = PathBuf::from("python").join("augment_dataset.py");
    let output = tokio::process::Command::new(crate::paths::python_cmd())
        .arg(&script)
        .arg("--dataset")
        .arg(dataset_path)
        .arg("--augment")
        .arg(methods.join(","))
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let out_path = format!("{dataset_path}.augmented");
    Ok(out_path)
}

pub fn read_jsonl_preview(path: &str, limit: usize) -> Result<Vec<serde_json::Value>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for line in content.lines().take(limit) {
        if let Ok(v) = serde_json::from_str(line) {
            rows.push(v);
        }
    }
    Ok(rows)
}
