//! RAG over Knowledge Module corpora.
//!
//! Pipeline:
//! 1. Extract text from a `.km` (manifest description + any `*.txt` / `*.md` / `*.jsonl` members)
//!    or from a plain directory / JSONL file referenced by a KM.
//! 2. Chunk into overlapping windows.
//! 3. Score with a lightweight lexical retriever (BM25-ish TF)·cosine over bag-of-words.
//!    (Swap in embeddings later via ONNX without changing the API.)
//! 4. Return top-k chunks for injection into prompts / graph RAG nodes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub chunks: Vec<String>,
    pub scores: Vec<f32>,
    pub source: String,
}

#[derive(Debug, Clone)]
struct Chunk {
    text: String,
    tf: HashMap<String, f32>,
}

/// Retrieve top-k passages relevant to `query` from a KM path, directory, or JSONL file.
pub async fn retrieve(source: &str, query: &str, k: usize) -> Result<RetrievalResult, String> {
    if source.is_empty() {
        return Err("RAG source path is empty".into());
    }
    let path = Path::new(source);
    if !path.exists() {
        return Err(format!("RAG source not found: {source}"));
    }

    let documents = load_documents(path)?;
    if documents.is_empty() {
        return Err(format!("No text content extractable from {source}"));
    }

    let chunks = chunk_documents(&documents, 180, 40);
    info!(chunks = chunks.len(), source, "RAG index built");

    let scored = score_chunks(&chunks, query);
    let top: Vec<_> = scored.into_iter().take(k.max(1)).collect();

    Ok(RetrievalResult {
        chunks: top.iter().map(|(c, _)| c.text.clone()).collect(),
        scores: top.iter().map(|(_, s)| *s).collect(),
        source: source.to_string(),
    })
}

/// Build (or refresh) a simple on-disk RAG index sidecar next to a KM.
/// Writes `{km_path}.rag.json` with chunk texts for faster subsequent loads.
pub async fn index_km(km_path: &str) -> Result<String, String> {
    let docs = load_documents(Path::new(km_path))?;
    let chunks = chunk_documents(&docs, 180, 40);
    let sidecar = format!("{km_path}.rag.json");
    let payload = serde_json::json!({
        "source": km_path,
        "chunk_count": chunks.len(),
        "chunks": chunks.iter().map(|c| &c.text).collect::<Vec<_>>(),
    });
    fs::write(&sidecar, serde_json::to_string_pretty(&payload).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(sidecar)
}

/// Compose a RAG-augmented prompt.
pub fn build_rag_prompt(query: &str, result: &RetrievalResult) -> String {
    let mut ctx = String::new();
    for (i, chunk) in result.chunks.iter().enumerate() {
        ctx.push_str(&format!("[{i}] {chunk}\n\n"));
    }
    format!(
        "Use the following knowledge module excerpts to answer the question.\n\n\
         ### Context\n{ctx}\
         ### Question\n{query}\n\n\
         ### Answer\n"
    )
}

// ── Internals ───────────────────────────────────────────────────────────

fn load_documents(path: &Path) -> Result<Vec<String>, String> {
    if path.is_file() {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "km" => load_from_km(path),
            "jsonl" => load_jsonl(path),
            "txt" | "md" => {
                let t = fs::read_to_string(path).map_err(|e| e.to_string())?;
                Ok(vec![t])
            }
            "json" => {
                // Maybe a RAG sidecar
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                    &fs::read_to_string(path).map_err(|e| e.to_string())?,
                ) {
                    if let Some(arr) = v.get("chunks").and_then(|c| c.as_array()) {
                        return Ok(arr
                            .iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect());
                    }
                }
                Ok(vec![fs::read_to_string(path).map_err(|e| e.to_string())?])
            }
            _ => {
                // Try as text
                fs::read_to_string(path)
                    .map(|t| vec![t])
                    .map_err(|e| e.to_string())
            }
        }
    } else if path.is_dir() {
        let mut docs = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if p.is_file() {
                if let Ok(mut d) = load_documents(&p) {
                    docs.append(&mut d);
                }
            }
        }
        Ok(docs)
    } else {
        Err(format!("Unsupported RAG path: {}", path.display()))
    }
}

fn load_from_km(path: &Path) -> Result<Vec<String>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut docs = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;

        if name == "manifest.json" {
            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&buf) {
                if let Some(desc) = v.get("description").and_then(|d| d.as_str()) {
                    if !desc.is_empty() {
                        docs.push(desc.to_string());
                    }
                }
                // Include adapter metadata as searchable text
                if let Some(adapters) = v.get("adapters").and_then(|a| a.as_array()) {
                    for a in adapters {
                        docs.push(serde_json::to_string(a).unwrap_or_default());
                    }
                }
            }
        } else if name.ends_with(".txt")
            || name.ends_with(".md")
            || name.ends_with(".jsonl")
            || name.ends_with(".csv")
        {
            if let Ok(text) = String::from_utf8(buf) {
                docs.push(text);
            }
        }
    }
    Ok(docs)
}

fn load_jsonl(path: &Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut docs = Vec::new();
    for line in content.lines() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
                docs.push(t.to_string());
            } else {
                docs.push(line.to_string());
            }
        }
    }
    Ok(docs)
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(String::from)
        .collect()
}

fn chunk_documents(docs: &[String], size: usize, overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    for doc in docs {
        let words: Vec<&str> = doc.split_whitespace().collect();
        if words.is_empty() {
            continue;
        }
        let mut start = 0;
        while start < words.len() {
            let end = (start + size).min(words.len());
            let text = words[start..end].join(" ");
            let tokens = tokenize(&text);
            let mut tf = HashMap::new();
            let len = tokens.len().max(1) as f32;
            for t in tokens {
                *tf.entry(t).or_insert(0.0) += 1.0 / len;
            }
            chunks.push(Chunk { text, tf });
            if end == words.len() {
                break;
            }
            start += size.saturating_sub(overlap).max(1);
        }
    }
    chunks
}

fn score_chunks(chunks: &[Chunk], query: &str) -> Vec<(Chunk, f32)> {
    let q_tokens = tokenize(query);
    let mut q_tf: HashMap<String, f32> = HashMap::new();
    let qlen = q_tokens.len().max(1) as f32;
    for t in &q_tokens {
        *q_tf.entry(t.clone()).or_insert(0.0) += 1.0 / qlen;
    }

    let mut scored: Vec<(Chunk, f32)> = chunks
        .iter()
        .map(|c| {
            // Cosine over TF bags + BM25-ish term frequency boost
            let mut dot = 0.0;
            let mut norm_c = 0.0;
            let mut norm_q = 0.0;
            for (term, &q_w) in &q_tf {
                norm_q += q_w * q_w;
                if let Some(&c_w) = c.tf.get(term) {
                    dot += q_w * c_w;
                }
            }
            for &c_w in c.tf.values() {
                norm_c += c_w * c_w;
            }
            let cosine = if norm_c > 0.0 && norm_q > 0.0 {
                dot / (norm_c.sqrt() * norm_q.sqrt())
            } else {
                0.0
            };
            // Simple coverage bonus
            let coverage = q_tokens
                .iter()
                .filter(|t| c.tf.contains_key(*t))
                .count() as f32
                / qlen;
            (c.clone(), cosine + 0.15 * coverage)
        })
        .filter(|(_, s)| *s > 0.0)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}


/// Hybrid retrieval using the in-process [`vector_store::VectorStore`].
/// Indexes `source` on the fly (fine for moderate corpora).
pub async fn retrieve_hybrid(
    source: &str,
    query: &str,
    k: usize,
    alpha: f32,
) -> Result<RetrievalResult, String> {
    use crate::vector_store::{HybridConfig, VectorStore};

    let docs = load_documents(Path::new(source))?;
    if docs.is_empty() {
        return Err(format!("No documents in {source}"));
    }
    let store = VectorStore::memory();
    let items: Vec<_> = docs
        .into_iter()
        .enumerate()
        .map(|(i, text)| (format!("doc-{i}"), text, std::collections::HashMap::new()))
        .collect();
    store.upsert_texts(items).await?;
    let hits = store
        .hybrid_search(
            query,
            &HybridConfig {
                alpha: alpha.clamp(0.0, 1.0),
                top_k: k.max(1),
                ..Default::default()
            },
        )
        .await?;
    Ok(RetrievalResult {
        chunks: hits.iter().map(|h| h.text.clone()).collect(),
        scores: hits.iter().map(|h| h.score).collect(),
        source: source.to_string(),
    })
}
