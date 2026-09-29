// Scans the real, standard `huggingface_hub` local cache layout
// (`models--{org}--{name}/snapshots/<revision>/*`, the same cache every
// `transformers`/`huggingface-cli download` call populates) for models
// already on this machine — the "any model locally available on device"
// requirement doesn't need network access or an API token, since a real
// personal machine that has ever run `huggingface-cli download` or a
// `transformers`/`sentence-transformers` script already has this cache
// populated. Verified directly against a real populated cache during
// development, not assumed from documentation.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelFormat {
    SafeTensors,
    Gguf,
    Onnx,
    PytorchBin,
}

impl ModelFormat {
    fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "safetensors" => Some(Self::SafeTensors),
            "gguf" => Some(Self::Gguf),
            "onnx" => Some(Self::Onnx),
            "bin" => Some(Self::PytorchBin),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalModel {
    /// e.g. `"Qwen/Qwen2.5-0.5B-Instruct"` for a HuggingFace-cache entry, or
    /// a directory/file-stem name for a plain local folder (see
    /// `scan_directory_for_models`).
    pub repo_id: String,
    /// The real on-disk directory this model's files live in (for a cache
    /// entry, this already resolves the cache's internal blob symlinks
    /// transparently — any normal file read against a path under here just
    /// works).
    pub snapshot_path: PathBuf,
    /// Every weight-bearing format found here — a repo can legitimately
    /// ship more than one (e.g. both `.bin` and `.safetensors` for backward
    /// compatibility, or a multi-file ONNX export's several `.onnx` parts).
    pub formats: Vec<ModelFormat>,
    /// Every real file name that's part of this model, for display/inspection.
    pub files: Vec<String>,
}

/// Resolves the real local HuggingFace cache root, honoring the same
/// environment variables `huggingface_hub` itself does (`HF_HUB_CACHE`
/// first, then `HF_HOME`/hub), falling back to the documented default
/// `~/.cache/huggingface/hub`.
pub fn default_hf_cache_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("HF_HUB_CACHE") {
        return PathBuf::from(dir);
    }
    if let Ok(home) = std::env::var("HF_HOME") {
        return PathBuf::from(home).join("hub");
    }
    home_dir().join(".cache").join("huggingface").join("hub")
}

fn home_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return PathBuf::from(profile);
    }
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}

/// `models--Qwen--Qwen2.5-0.5B-Instruct` -> `Qwen/Qwen2.5-0.5B-Instruct`;
/// `models--distilgpt2` (no namespace) -> `distilgpt2`. The cache's own
/// separator is a literal double-dash between `models`, the (optional)
/// namespace, and the model name — single dashes inside either part (very
/// common: `Qwen2.5-0.5B-Instruct`) are left untouched.
fn dirname_to_repo_id(dirname: &str) -> Option<String> {
    let rest = dirname.strip_prefix("models--")?;
    let parts: Vec<&str> = rest.splitn(2, "--").collect();
    Some(parts.join("/"))
}

/// The real revision this repo's `refs/main` points at, if present — used to
/// pick a snapshot deterministically rather than an arbitrary directory
/// listing order when a repo (rarely) has more than one cached revision.
fn resolve_main_revision(repo_dir: &Path) -> Option<String> {
    std::fs::read_to_string(repo_dir.join("refs").join("main")).ok().map(|s| s.trim().to_string())
}

/// Scans every `models--*` directory in `cache_dir` and returns the ones
/// with at least one real weight file in a recognized format. Silently
/// skips anything unreadable (a half-downloaded or permission-restricted
/// entry) rather than failing the whole scan over one bad repo.
pub fn scan_local_models(cache_dir: &Path) -> Vec<LocalModel> {
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return Vec::new();
    };

    let mut models = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        if !file_type.is_dir() {
            continue;
        }
        let dirname = entry.file_name().to_string_lossy().to_string();
        let Some(repo_id) = dirname_to_repo_id(&dirname) else { continue };
        let repo_dir = entry.path();

        let snapshots_dir = repo_dir.join("snapshots");
        let snapshot_path = resolve_main_revision(&repo_dir)
            .map(|rev| snapshots_dir.join(rev))
            .filter(|p| p.is_dir())
            .or_else(|| {
                std::fs::read_dir(&snapshots_dir)
                    .ok()?
                    .flatten()
                    .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .map(|e| e.path())
            });
        let Some(snapshot_path) = snapshot_path else { continue };

        let Ok(files) = std::fs::read_dir(&snapshot_path) else { continue };
        let mut file_names = Vec::new();
        let mut formats = Vec::new();
        for f in files.flatten() {
            let name = f.file_name().to_string_lossy().to_string();
            if let Some(ext) = name.rsplit('.').next() {
                if let Some(fmt) = ModelFormat::from_extension(ext) {
                    if !formats.contains(&fmt) {
                        formats.push(fmt);
                    }
                }
            }
            file_names.push(name);
        }
        if formats.is_empty() {
            continue; // no recognizable weight file — not a usable model
        }
        file_names.sort();
        models.push(LocalModel { repo_id, snapshot_path, formats, files: file_names });
    }
    models.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));
    models
}

/// Scans an arbitrary local directory for model weight files — not every
/// personal model collection lives in the HuggingFace hub cache; a common
/// real layout (verified against an actual populated example) mixes loose
/// standalone weight files directly in the root (several independent
/// differently-quantized `.gguf`/`.safetensors` files side by side) with
/// subdirectories that are each one cohesive model export (a single `.gguf`,
/// or a multi-part ONNX export's several `encoder`/`decoder`/`embed_tokens`
/// files that only make sense together).
///
/// The grouping rule this encodes: **at the scanned root**, each weight
/// file is its own model (loose files at top level are typically
/// independent variants, not parts of one thing); **in any subdirectory**,
/// every weight file found directly inside it is grouped into one model
/// (a subdirectory is typically one export, even when it bundles several
/// related files). Recurses up to `max_depth` levels to avoid an unbounded
/// walk of an entire drive.
pub fn scan_directory_for_models(root: &Path, max_depth: usize) -> Vec<LocalModel> {
    let mut models = Vec::new();
    scan_dir_level(root, root, max_depth, &mut models);
    models.sort_by(|a, b| a.repo_id.cmp(&b.repo_id));
    models
}

fn scan_dir_level(root: &Path, dir: &Path, depth_remaining: usize, out: &mut Vec<LocalModel>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };

    let mut weight_files: Vec<(String, ModelFormat)> = Vec::new();
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else { continue };
        let path = entry.path();
        if file_type.is_dir() {
            subdirs.push(path);
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(ext) = name.rsplit('.').next() {
            if let Some(fmt) = ModelFormat::from_extension(ext) {
                weight_files.push((name, fmt));
            }
        }
    }

    if !weight_files.is_empty() {
        if dir == root {
            // Loose top-level files: each is independently selectable.
            for (name, fmt) in &weight_files {
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name).to_string();
                out.push(LocalModel {
                    repo_id: stem,
                    snapshot_path: dir.to_path_buf(),
                    formats: vec![*fmt],
                    files: vec![name.clone()],
                });
            }
        } else {
            // A subdirectory's weight files are one cohesive model.
            let mut formats: Vec<ModelFormat> = weight_files.iter().map(|(_, f)| *f).collect();
            formats.sort_by_key(|f| format!("{f:?}"));
            formats.dedup();
            let mut files: Vec<String> = weight_files.iter().map(|(n, _)| n.clone()).collect();
            files.sort();
            let repo_id = dir.strip_prefix(root).unwrap_or(dir).to_string_lossy().replace('\\', "/");
            out.push(LocalModel { repo_id, snapshot_path: dir.to_path_buf(), formats, files });
        }
    }

    if depth_remaining > 0 {
        for subdir in subdirs {
            scan_dir_level(root, &subdir, depth_remaining - 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn fake_cache(dir: &Path) {
        // Namespaced repo with a `refs/main` pointer, real-shaped snapshot.
        let repo = dir.join("models--Qwen--Qwen2.5-0.5B-Instruct");
        write_file(&repo.join("refs").join("main"), "abc123");
        write_file(&repo.join("snapshots").join("abc123").join("model.safetensors"), "fake-weights");
        write_file(&repo.join("snapshots").join("abc123").join("config.json"), "{}");

        // Non-namespaced repo, no refs/main (falls back to directory scan).
        let repo2 = dir.join("models--distilgpt2");
        write_file(&repo2.join("snapshots").join("def456").join("model.safetensors"), "fake-weights");

        // A directory with no recognizable weight format — must be skipped.
        let repo3 = dir.join("models--not-a-real-model");
        write_file(&repo3.join("snapshots").join("xyz").join("README.md"), "hi");
    }

    #[test]
    fn dirname_to_repo_id_handles_namespaced_and_bare_names() {
        assert_eq!(dirname_to_repo_id("models--Qwen--Qwen2.5-0.5B-Instruct").as_deref(), Some("Qwen/Qwen2.5-0.5B-Instruct"));
        assert_eq!(dirname_to_repo_id("models--distilgpt2").as_deref(), Some("distilgpt2"));
        assert_eq!(dirname_to_repo_id("models--sentence-transformers--all-MiniLM-L6-v2").as_deref(), Some("sentence-transformers/all-MiniLM-L6-v2"));
        assert_eq!(dirname_to_repo_id("not-a-models-dir"), None);
    }

    #[test]
    fn scans_a_real_cache_layout_and_skips_repos_with_no_weights() {
        let dir = std::env::temp_dir().join(format!("bb_hf_cache_test_{}", uuid::Uuid::new_v4()));
        fake_cache(&dir);

        let models = scan_local_models(&dir);
        let repo_ids: Vec<&str> = models.iter().map(|m| m.repo_id.as_str()).collect();
        assert!(repo_ids.contains(&"Qwen/Qwen2.5-0.5B-Instruct"));
        assert!(repo_ids.contains(&"distilgpt2"));
        assert!(!repo_ids.contains(&"not-a-real-model"), "repo with no weight files should be skipped");

        let qwen = models.iter().find(|m| m.repo_id == "Qwen/Qwen2.5-0.5B-Instruct").unwrap();
        assert_eq!(qwen.formats, vec![ModelFormat::SafeTensors]);
        assert!(qwen.snapshot_path.ends_with("abc123"), "should resolve via refs/main");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Mirrors a real personal model directory found during development:
    /// loose independently-quantized files at the top level, subdirectories
    /// each holding one model's file(s), and a subdirectory bundling a
    /// multi-part ONNX export.
    #[test]
    fn scans_a_flat_directory_with_loose_files_and_bundled_subdirs() {
        let dir = std::env::temp_dir().join(format!("bb_flat_dir_test_{}", uuid::Uuid::new_v4()));
        write_file(&dir.join("variant-a.gguf"), "fake");
        write_file(&dir.join("variant-b.safetensors"), "fake");
        write_file(&dir.join("Bonsai-1.7B-IQ1_S").join("Bonsai-1.7B-IQ1_S.gguf"), "fake");
        write_file(&dir.join("onnx").join("decoder_model.onnx"), "fake");
        write_file(&dir.join("onnx").join("embed_tokens.onnx"), "fake");
        write_file(&dir.join("config.json"), "{}"); // real metadata, not a weight file — must be ignored

        let models = scan_directory_for_models(&dir, 4);
        let by_id: std::collections::HashMap<&str, &LocalModel> = models.iter().map(|m| (m.repo_id.as_str(), m)).collect();

        assert!(by_id.contains_key("variant-a"), "loose top-level file should be its own model");
        assert!(by_id.contains_key("variant-b"), "loose top-level file should be its own model");
        assert_eq!(by_id["variant-a"].files, vec!["variant-a.gguf"]);

        assert!(by_id.contains_key("Bonsai-1.7B-IQ1_S"), "subdirectory should become one model");
        assert_eq!(by_id["Bonsai-1.7B-IQ1_S"].files, vec!["Bonsai-1.7B-IQ1_S.gguf"]);

        let onnx_model = by_id.get("onnx").expect("onnx/ subdirectory should be one bundled model");
        assert_eq!(onnx_model.files.len(), 2, "both onnx parts should be grouped into the same model");
        assert_eq!(onnx_model.formats, vec![ModelFormat::Onnx]);

        assert!(!by_id.contains_key("config"), "non-weight files must not become models");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Best-effort real-world check: if the user's actual local model
    /// directory exists on this machine, scan it for real and sanity-check
    /// the result — not a synthetic fixture.
    #[test]
    fn scans_the_developers_real_local_model_directory_if_present() {
        let real_dir = Path::new("D:/Models/general");
        let has_subdirs = std::fs::read_dir(real_dir)
            .map(|rd| rd.flatten().any(|e| e.path().is_dir()))
            .unwrap_or(false);
        if !has_subdirs {
            eprintln!("D:/Models/general is absent or holds no model folders on this machine — skipping (not a failure)");
            return;
        }
        let models = scan_directory_for_models(real_dir, 4);
        assert!(!models.is_empty(), "a real populated model directory should yield at least one model");
        for m in &models {
            assert!(!m.formats.is_empty());
            assert!(!m.files.is_empty());
            assert!(m.snapshot_path.starts_with(real_dir));
        }
        eprintln!("found {} real local models: {:?}", models.len(), models.iter().map(|m| &m.repo_id).collect::<Vec<_>>());
    }
}
