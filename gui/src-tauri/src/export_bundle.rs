//! Export a self-contained model bundle (base + KMs + graph + config).

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

pub fn create_bundle(
    base_model_path: &str,
    km_paths: &[String],
    graph_json: &str,
    output_path: &str,
) -> Result<(), String> {
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = File::create(output_path).map_err(|e| format!("Create bundle: {e}"))?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

    // Base model (stream if large – for demo we read fully)
    if Path::new(base_model_path).exists() {
        let data = fs::read(base_model_path).map_err(|e| e.to_string())?;
        let name = Path::new(base_model_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("base_model.bin");
        zip.start_file(format!("models/{name}"), options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
    }

    let mut km_names = Vec::new();
    for km_path in km_paths {
        if !Path::new(km_path).exists() {
            continue;
        }
        let data = fs::read(km_path).map_err(|e| e.to_string())?;
        let name = Path::new(km_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("module.km")
            .to_string();
        zip.start_file(format!("kms/{name}"), options)
            .map_err(|e| e.to_string())?;
        zip.write_all(&data).map_err(|e| e.to_string())?;
        km_names.push(format!("kms/{name}"));
    }

    zip.start_file("graph.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(graph_json.as_bytes())
        .map_err(|e| e.to_string())?;

    let config = serde_json::json!({
        "format_version": "1.0",
        "base_model": base_model_path,
        "knowledge_modules": km_names,
        "graph": "graph.json",
        "created_at": chrono::Utc::now().to_rfc3339(),
    });
    zip.start_file("bundle_config.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
        .map_err(|e| e.to_string())?;

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
