#![allow(dead_code)]
//! Deterministic long-term memory with hash-based embeddings, store/recall/forget/prune/stats.

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: i64,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Default)]
pub struct LongTermMemory {
    _entries: Arc<Mutex<Vec<MemoryEntry>>>,
    embedding_dim: usize,
}

impl LongTermMemory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_dim(dim: usize) -> Self {
        Self {
            _entries: Arc::new(Mutex::new(Vec::new())),
            embedding_dim: dim,
        }
    }

    pub fn with_dimension(dim: usize) -> Self {
        Self {
            _entries: Arc::new(Mutex::new(Vec::new())),
            embedding_dim: dim,
        }
    }

    pub async fn store(
        &self,
        content: impl Into<String>,
        tags: Vec<String>,
        metadata: HashMap<String, serde_json::Value>,
    ) -> MemoryEntry {
        let content = content.into();
        let ts = chrono::Utc::now().timestamp_millis();
        let id = format!("mem-{}", uuid::Uuid::new_v4());
        let embedding = self.compute_embedding(&content);

        let entry = MemoryEntry { id, content, tags, timestamp: ts, embedding, metadata };
        self._entries.lock().await.push(entry.clone());
        entry
    }

    pub async fn recall(&self, query: &str, top_k: usize) -> Vec<MemoryEntry> {
        let query_emb = self.compute_embedding(query);
        let all = self._entries.lock().await.clone();
        let mut scored: Vec<(f32, MemoryEntry)> = all
            .into_iter()
            .map(|e| (cosine_similarity(&query_emb, &e.embedding), e))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, e)| e).collect()
    }

    pub async fn forget(&self, older_than_secs: i64) -> usize {
        let cutoff = chrono::Utc::now().timestamp_millis() - older_than_secs * 1000;
        let before = self._entries.lock().await.len();
        self._entries.lock().await.retain(|e| e.timestamp > cutoff);
        let after = self._entries.lock().await.len();
        before - after
    }

    pub async fn list(&self) -> Vec<MemoryEntry> {
        self._entries.lock().await.clone()
    }

    pub async fn prune(&self, max_entries: usize) -> usize {
        let mut entries = self._entries.lock().await;
        let before = entries.len();
        if entries.len() > max_entries {
            entries.truncate(max_entries);
        }
        let after = entries.len();
        before - after
    }

    /// Deterministic hash-based embedding. Not learned, but consistent for
    /// identical inputs and suitable for offline retrieval/prefiltering.
    fn compute_embedding(&self, text: &str) -> Vec<f32> {
        let dim = self.embedding_dim.max(16);
        let mut vec = vec![0.0f32; dim];

        // Chunk the text and hash each chunk into a bucket.
        let chunk_size = 32usize;
        for (_chunk_idx, chunk) in text.bytes().collect::<Vec<_>>().chunks(chunk_size).enumerate() {
            let mut h = DefaultHasher::new();
            chunk.hash(&mut h);
            let bucket = (h.finish() as usize) % dim;
            let weight = (chunk.len() as f32) / (chunk_size as f32);
            vec[bucket] += weight;
        }

        // L2-normalize so cosine similarity is just dot product.
        let mag: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for v in &mut vec { *v /= mag; }
        }

        vec
    }

    pub async fn stats(&self) -> (usize, usize) {
        let entries = self._entries.lock().await.len();
        (entries, self.embedding_dim)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { 0.0 } else { dot / (mag_a * mag_b) }
}
