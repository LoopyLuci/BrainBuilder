//! Lightweight vector store + **hybrid search** for OmniForge RAG.
//!
//! Design goals:
//! - Zero mandatory native deps (pure Rust in-memory index)
//! - Pluggable backend trait so Qdrant / sqlite-vss / LanceDB can be swapped in
//! - Hybrid retrieval = α · dense_cosine + (1−α) · lexical_bm25
//!
//! Embedding strategy (default):
//! - Hashing trick / feature hashing into a fixed dim (384) – deterministic, no model required
//! - Optional: replace `HashEmbedder` with an ONNX sentence-transformer via `model_executor`
//!
//! Persistence: JSONL dump/load of vectors + metadata (portable, human-inspectable).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const DEFAULT_DIM: usize = 384;

// ── Public types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRecord {
    pub id: String,
    pub text: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,
    pub text: String,
    pub score: f32,
    pub dense_score: f32,
    pub lexical_score: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct HybridConfig {
    /// Weight on dense cosine similarity in [0, 1]. Lexical weight = 1 − α.
    pub alpha: f32,
    /// BM25 k1
    pub k1: f32,
    /// BM25 b
    pub b: f32,
    pub top_k: usize,
}

impl Default for HybridConfig {
    fn default() -> Self {
        Self {
            alpha: 0.6,
            k1: 1.5,
            b: 0.75,
            top_k: 5,
        }
    }
}

// ── Embedder trait ──────────────────────────────────────────────────────

pub trait Embedder: Send + Sync {
    fn dim(&self) -> usize;
    fn embed(&self, text: &str) -> Vec<f32>;
}

/// Deterministic feature-hashing embedder (no external model).
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(32) }
    }
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new(DEFAULT_DIM)
    }
}

impl Embedder for HashEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    fn embed(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        for tok in tokenize(text) {
            let h = hash_str(&tok);
            let idx = (h as usize) % self.dim;
            let sign = if h & 1 == 0 { 1.0 } else { -1.0 };
            v[idx] += sign;
            // Bigram boost
            let h2 = hash_str(&(tok.clone() + "#"));
            let idx2 = (h2 as usize) % self.dim;
            v[idx2] += 0.5 * if h2 & 1 == 0 { 1.0 } else { -1.0 };
        }
        l2_normalize(&mut v);
        v
    }
}

// ── Backend trait (for future Qdrant / sqlite-vss) ───────────────────────

#[async_trait::async_trait]
pub trait VectorBackend: Send + Sync {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<usize, String>;
    async fn search_dense(&self, query: &[f32], top_k: usize) -> Result<Vec<(String, f32)>, String>;
    async fn get(&self, id: &str) -> Result<Option<VectorRecord>, String>;
    async fn len(&self) -> usize;
    async fn clear(&self) -> Result<(), String>;
}

// ── In-memory backend ───────────────────────────────────────────────────

pub struct MemoryBackend {
    inner: RwLock<HashMap<String, VectorRecord>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl VectorBackend for MemoryBackend {
    async fn upsert(&self, records: Vec<VectorRecord>) -> Result<usize, String> {
        let mut map = self.inner.write().await;
        let n = records.len();
        for r in records {
            map.insert(r.id.clone(), r);
        }
        Ok(n)
    }

    async fn search_dense(
        &self,
        query: &[f32],
        top_k: usize,
    ) -> Result<Vec<(String, f32)>, String> {
        let map = self.inner.read().await;
        let mut scored: Vec<(String, f32)> = map
            .values()
            .map(|r| (r.id.clone(), cosine(&r.vector, query)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k.max(1));
        Ok(scored)
    }

    async fn get(&self, id: &str) -> Result<Option<VectorRecord>, String> {
        Ok(self.inner.read().await.get(id).cloned())
    }

    async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    async fn clear(&self) -> Result<(), String> {
        self.inner.write().await.clear();
        Ok(())
    }
}

// ── Vector store (hybrid) ───────────────────────────────────────────────

pub struct VectorStore {
    backend: Arc<dyn VectorBackend>,
    embedder: Arc<dyn Embedder>,
    /// Parallel lexical index: doc_id → term frequencies + doc length
    lexical: RwLock<LexicalIndex>,
}

#[derive(Default)]
struct LexicalIndex {
    /// term → (doc_id → tf)
    postings: HashMap<String, HashMap<String, f32>>,
    /// doc_id → length in tokens
    doc_len: HashMap<String, f32>,
    /// doc_id → original text
    texts: HashMap<String, String>,
    /// doc_id → metadata
    metadata: HashMap<String, HashMap<String, String>>,
    total_docs: usize,
    avg_len: f32,
}

impl VectorStore {
    pub fn new(backend: Arc<dyn VectorBackend>, embedder: Arc<dyn Embedder>) -> Self {
        Self {
            backend,
            embedder,
            lexical: RwLock::new(LexicalIndex::default()),
        }
    }

    pub fn memory() -> Self {
        Self::new(
            Arc::new(MemoryBackend::new()),
            Arc::new(HashEmbedder::default()),
        )
    }

    pub async fn upsert_texts(
        &self,
        items: Vec<(String, String, HashMap<String, String>)>,
    ) -> Result<usize, String> {
        let mut records = Vec::with_capacity(items.len());
        let mut lex = self.lexical.write().await;

        for (id, text, meta) in items {
            let vector = self.embedder.embed(&text);
            // Lexical
            let tokens = tokenize(&text);
            let len = tokens.len().max(1) as f32;
            let mut tf: HashMap<String, f32> = HashMap::new();
            for t in &tokens {
                *tf.entry(t.clone()).or_insert(0.0) += 1.0;
            }
            for (term, count) in &tf {
                lex.postings
                    .entry(term.clone())
                    .or_default()
                    .insert(id.clone(), *count);
            }
            lex.doc_len.insert(id.clone(), len);
            lex.texts.insert(id.clone(), text.clone());
            lex.metadata.insert(id.clone(), meta.clone());

            records.push(VectorRecord {
                id,
                text,
                vector,
                metadata: meta,
            });
        }
        lex.total_docs = lex.doc_len.len();
        let sum: f32 = lex.doc_len.values().sum();
        lex.avg_len = if lex.total_docs > 0 {
            sum / lex.total_docs as f32
        } else {
            1.0
        };
        drop(lex);

        self.backend.upsert(records).await
    }

    /// Hybrid search: α · cosine + (1−α) · BM25, both min-max normalized per query.
    pub async fn hybrid_search(
        &self,
        query: &str,
        cfg: &HybridConfig,
    ) -> Result<Vec<SearchHit>, String> {
        let q_vec = self.embedder.embed(query);
        // Over-fetch dense candidates then fuse
        let fetch = (cfg.top_k * 4).max(20);
        let dense = self.backend.search_dense(&q_vec, fetch).await?;

        let lex = self.lexical.read().await;
        let bm25_scores = bm25_score(query, &lex, cfg.k1, cfg.b);

        // Collect candidate ids
        let mut candidates: HashMap<String, (f32, f32)> = HashMap::new();
        for (id, s) in &dense {
            candidates.entry(id.clone()).or_insert((0.0, 0.0)).0 = *s;
        }
        for (id, s) in &bm25_scores {
            candidates.entry(id.clone()).or_insert((0.0, 0.0)).1 = *s;
        }

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        // Min-max normalize each channel
        let dens_vals: Vec<f32> = candidates.values().map(|c| c.0).collect();
        let lex_vals: Vec<f32> = candidates.values().map(|c| c.1).collect();
        let (d_min, d_max) = min_max(&dens_vals);
        let (l_min, l_max) = min_max(&lex_vals);

        let alpha = cfg.alpha.clamp(0.0, 1.0);
        let mut hits: Vec<SearchHit> = candidates
            .into_iter()
            .map(|(id, (d, l))| {
                let dn = norm01(d, d_min, d_max);
                let ln = norm01(l, l_min, l_max);
                let score = alpha * dn + (1.0 - alpha) * ln;
                let text = lex.texts.get(&id).cloned().unwrap_or_default();
                let metadata = lex.metadata.get(&id).cloned().unwrap_or_default();
                SearchHit {
                    id,
                    text,
                    score,
                    dense_score: dn,
                    lexical_score: ln,
                    metadata,
                }
            })
            .collect();

        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(cfg.top_k.max(1));
        Ok(hits)
    }

    pub async fn len(&self) -> usize {
        self.backend.len().await
    }

    pub async fn clear(&self) -> Result<(), String> {
        *self.lexical.write().await = LexicalIndex::default();
        self.backend.clear().await
    }

    /// Persist vectors + text as JSONL (re-embeds from lexical text).
    pub async fn save_jsonl(&self, path: &str) -> Result<usize, String> {
        let lex = self.lexical.read().await;
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = File::create(path).map_err(|e| e.to_string())?;
        let mut n = 0;
        for (id, text) in &lex.texts {
            let rec = VectorRecord {
                id: id.clone(),
                text: text.clone(),
                vector: self.embedder.embed(text),
                metadata: lex.metadata.get(id).cloned().unwrap_or_default(),
            };
            writeln!(f, "{}", serde_json::to_string(&rec).unwrap()).map_err(|e| e.to_string())?;
            n += 1;
        }
        Ok(n)
    }

    pub async fn load_jsonl(&self, path: &str) -> Result<usize, String> {
        let file = File::open(path).map_err(|e| e.to_string())?;
        let reader = BufReader::new(file);
        let mut items = Vec::new();
        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            if line.trim().is_empty() {
                continue;
            }
            let rec: VectorRecord =
                serde_json::from_str(&line).map_err(|e| format!("JSONL parse: {e}"))?;
            items.push((rec.id, rec.text, rec.metadata));
        }
        self.upsert_texts(items).await
    }
}

// ── BM25 ────────────────────────────────────────────────────────────────

fn bm25_score(query: &str, lex: &LexicalIndex, k1: f32, b: f32) -> HashMap<String, f32> {
    let terms = tokenize(query);
    let n = lex.total_docs.max(1) as f32;
    let mut scores: HashMap<String, f32> = HashMap::new();

    for term in &terms {
        let posting = match lex.postings.get(term) {
            Some(p) => p,
            None => continue,
        };
        let df = posting.len().max(1) as f32;
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln().max(0.0);

        for (doc_id, &tf) in posting {
            let dl = *lex.doc_len.get(doc_id).unwrap_or(&1.0);
            let denom = tf + k1 * (1.0 - b + b * dl / lex.avg_len.max(1.0));
            let term_score = idf * (tf * (k1 + 1.0)) / denom;
            *scores.entry(doc_id.clone()).or_insert(0.0) += term_score;
        }
    }
    scores
}

// ── Math / tokenize helpers ─────────────────────────────────────────────

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() > 2)
        .map(String::from)
        .collect()
}

fn hash_str(s: &str) -> u64 {
    // FNV-1a 64-bit
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-12 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for i in 0..n {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-12 {
        0.0
    } else {
        dot / denom
    }
}

fn min_max(vals: &[f32]) -> (f32, f32) {
    let mut mn = f32::MAX;
    let mut mx = f32::MIN;
    for &v in vals {
        if v < mn {
            mn = v;
        }
        if v > mx {
            mx = v;
        }
    }
    if !mn.is_finite() {
        mn = 0.0;
    }
    if !mx.is_finite() {
        mx = 1.0;
    }
    (mn, mx)
}

fn norm01(v: f32, mn: f32, mx: f32) -> f32 {
    if (mx - mn).abs() < 1e-12 {
        0.0
    } else {
        ((v - mn) / (mx - mn)).clamp(0.0, 1.0)
    }
}

// ── Unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hybrid_prefers_relevant_docs() {
        let store = VectorStore::memory();
        store
            .upsert_texts(vec![
                (
                    "1".into(),
                    "OmniForge trains LoRA adapters on medical text".into(),
                    HashMap::new(),
                ),
                (
                    "2".into(),
                    "The weather in Paris is mild in spring".into(),
                    HashMap::new(),
                ),
                (
                    "3".into(),
                    "Knowledge modules package adapter weights for reuse".into(),
                    HashMap::new(),
                ),
            ])
            .await
            .unwrap();

        let hits = store
            .hybrid_search(
                "LoRA medical training",
                &HybridConfig {
                    alpha: 0.5,
                    top_k: 2,
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].id, "1", "most relevant doc should rank first");
        assert!(hits[0].score >= hits.get(1).map(|h| h.score).unwrap_or(0.0));
    }

    #[tokio::test]
    async fn pure_lexical_still_works() {
        let store = VectorStore::memory();
        store
            .upsert_texts(vec![
                ("a".into(), "vector database integration".into(), HashMap::new()),
                ("b".into(), "completely unrelated content here".into(), HashMap::new()),
            ])
            .await
            .unwrap();
        let hits = store
            .hybrid_search(
                "vector database",
                &HybridConfig {
                    alpha: 0.0, // pure BM25
                    top_k: 1,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(hits[0].id, "a");
    }

    #[tokio::test]
    async fn pure_dense_still_works() {
        let store = VectorStore::memory();
        store
            .upsert_texts(vec![
                ("x".into(), "hybrid search algorithms bm25 cosine".into(), HashMap::new()),
                ("y".into(), "cats and dogs playing outside".into(), HashMap::new()),
            ])
            .await
            .unwrap();
        let hits = store
            .hybrid_search(
                "hybrid search bm25",
                &HybridConfig {
                    alpha: 1.0, // pure dense
                    top_k: 1,
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(hits[0].id, "x");
    }

    #[test]
    fn embedder_is_deterministic() {
        let e = HashEmbedder::default();
        let a = e.embed("hello world");
        let b = e.embed("hello world");
        assert_eq!(a, b);
        assert!((a.iter().map(|x| x * x).sum::<f32>().sqrt() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_identical_is_one() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-5);
    }
}
