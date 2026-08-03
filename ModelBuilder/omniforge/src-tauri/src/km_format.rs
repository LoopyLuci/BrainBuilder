//! Knowledge Module (.km) format – ZIP archive with LoRA adapters + manifest.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KmManifest {
    pub km_version: String,
    pub id: String,
    pub name: String,
    pub description: String,
    pub base_model_fingerprints: Vec<BaseModelFingerprint>,
    pub adapters: Vec<AdapterMeta>,
    pub training: Option<TrainingInfo>,
    pub provenance: ProvenanceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseModelFingerprint {
    pub architecture: String,
    pub model_id: String,
    pub weight_hash: String,
    pub required_adapters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMeta {
    pub name: String,
    pub target_modules: Vec<String>,
    pub rank: u32,
    pub alpha: f32,
    pub weights_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingInfo {
    pub dataset_checksum: String,
    pub base_model_checkpoint: String,
    pub hyperparameters: serde_json::Value,
    pub benchmarks: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    pub created_by: String,
    pub timestamp: String,
    pub license: String,
}

/// Create a .km file from adapter weight bytes + manifest.
pub fn create_km(
    manifest: &KmManifest,
    adapter_files: &[(String, Vec<u8>)],
    output_path: &str,
) -> Result<(), String> {
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(output_path).map_err(|e| format!("Create KM: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("manifest.json", options)
        .map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    zip.write_all(json.as_bytes()).map_err(|e| e.to_string())?;

    for (name, data) in adapter_files {
        zip.start_file(name, options).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

/// Read a .km archive.
pub fn read_km(path: &str) -> Result<(KmManifest, Vec<(String, Vec<u8>)>), String> {
    let file = File::open(path).map_err(|e| format!("Open KM: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("ZIP: {e}"))?;
    let mut manifest = None;
    let mut adapters = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        if name == "manifest.json" {
            manifest = Some(serde_json::from_slice(&data).map_err(|e| e.to_string())?);
        } else {
            adapters.push((name, data));
        }
    }
    let manifest = manifest.ok_or_else(|| "No manifest.json in KM".to_string())?;
    Ok((manifest, adapters))
}

/// Build a minimal KM from a trained adapter directory.
pub fn package_adapter_dir(
    adapter_dir: &str,
    name: &str,
    base_model: &str,
    rank: u32,
    alpha: f32,
    output_km: &str,
) -> Result<String, String> {
    let mut files = Vec::new();
    let dir = Path::new(adapter_dir);
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_file() {
                let data = fs::read(&path).map_err(|e| e.to_string())?;
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("weights.bin")
                    .to_string();
                files.push((format!("adapter/{name}"), data));
            }
        }
    }

    let weights_file = files
        .first()
        .map(|(n, _)| n.clone())
        .unwrap_or_else(|| "adapter/adapter_model.safetensors".into());

    let manifest = KmManifest {
        km_version: "1.0".into(),
        id: format!("km-{}", uuid::Uuid::new_v4()),
        name: name.into(),
        description: format!("Knowledge module for {base_model}"),
        base_model_fingerprints: vec![BaseModelFingerprint {
            architecture: "auto".into(),
            model_id: base_model.into(),
            weight_hash: String::new(),
            required_adapters: vec!["lora".into()],
        }],
        adapters: vec![AdapterMeta {
            name: "default".into(),
            target_modules: vec!["q_proj".into(), "v_proj".into()],
            rank,
            alpha,
            weights_file,
        }],
        training: None,
        provenance: ProvenanceInfo {
            created_by: "OmniForge".into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            license: "MIT".into(),
        },
    };

    create_km(&manifest, &files, output_km)?;
    Ok(manifest.id)
}

/// Reference string for applying a KM to a GGUF base (llama.cpp --lora style).
pub fn apply_km_ref(base_model_path: &str, km_path: &str) -> String {
    format!("{base_model_path}+lora:{km_path}")
}
