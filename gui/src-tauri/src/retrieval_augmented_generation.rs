#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    pub id: String,
    pub score: f64,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct RagQuery {
    pub query: String,
    pub top_k: usize,
}

#[derive(Debug, Clone)]
pub struct RagResponse {
    pub answer: String,
    pub sources: Vec<RetrievalResult>,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct RetrievalAugmentedGeneration {
    pub documents: Vec<(String, Vec<f64>)>,
    pub embedding_dim: usize,
}

impl RetrievalAugmentedGeneration {
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            documents: Vec::new(),
            embedding_dim,
        }
    }

    pub fn add_document(&mut self, id: String, embedding: Vec<f64>) {
        assert_eq!(embedding.len(), self.embedding_dim, "embedding dimension mismatch");
        self.documents.push((id, embedding));
    }

    pub fn query(&self, query: &str, top_k: usize) -> RagResponse {
        if self.documents.is_empty() {
            return RagResponse {
                answer: String::new(),
                sources: Vec::new(),
                confidence: 0.0,
            };
        }

        let query_embedding = self.embed_text(query);
        let mut scored: Vec<(String, f64, Vec<f64>)> = self.documents
            .iter()
            .map(|(id, emb)| {
                let score = self.cosine_similarity(&query_embedding, emb);
                (id.clone(), score, emb.clone())
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);

        let sources: Vec<RetrievalResult> = scored.into_iter()
            .map(|(id, score, _)| {
                let content = format!("document_{}", id.clone());
                RetrievalResult {
                    id,
                    score,
                    content,
                }
            })
            .collect();

        let confidence = sources.first().map(|s| s.score).unwrap_or(0.0);
        let answer = if confidence > 0.7 {
            format!("Based on top retrieval, the answer relates to: {}", query)
        } else {
            format!("Low confidence retrieval for: {}", query)
        };

        RagResponse { answer, sources, confidence }
    }

    fn embed_text(&self, text: &str) -> Vec<f64> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut embedding = vec![0.0; self.embedding_dim];
        for (i, word) in words.iter().enumerate() {
            let idx = (word.len() + i) % self.embedding_dim;
            embedding[idx] += 1.0;
        }
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
        embedding
    }

    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 { dot / (norm_a * norm_b) } else { 0.0 }
    }
}

#[tauri::command]
pub async fn rag_query(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, query: String, top_k: usize) -> Result<RagResponse, String> {
    let s = state.lock().await;
    let rag = s.retrieval_augmented_generation.lock().await;
    Ok(rag.query(&query, top_k))
}

#[tauri::command]
pub async fn rag_add_document(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, id: String, embedding: Vec<f64>) -> Result<(), String> {
    let s = state.lock().await;
    let mut rag = s.retrieval_augmented_generation.lock().await;
    rag.add_document(id, embedding);
    Ok(())
}
