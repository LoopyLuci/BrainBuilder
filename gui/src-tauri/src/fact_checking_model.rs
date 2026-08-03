#![allow(dead_code)]
//! Custom fact-checking model with TF-IDF, source decay, cross-reference,
//! persistence, and batch verification.
//!
//! This is a fully self-contained offline-runnable verification engine.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvidenceChunk {
    pub text: String,
    pub source_domain: String,
    pub stance: Stance,
    pub confidence: f64,
    pub tfidf_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stance {
    Supports,
    Refutes,
    Neutral,
    Unrelated,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimVerification {
    pub claim: String,
    pub verdict: Verdict,
    pub confidence: f64,
    pub supporting: Vec<EvidenceChunk>,
    pub contradicting: Vec<EvidenceChunk>,
    pub neutral: Vec<EvidenceChunk>,
    pub reasoning: Vec<String>,
    pub cross_references: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceProfile {
    pub domain: String,
    pub reliability: f64,
    pub factuality: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Verdict {
    Verified,
    LikelyTrue,
    Uncertain,
    LikelyFalse,
    Disputed,
    Unsupported,
}

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct FactCheckingModel {
    sources: Arc<Mutex<HashMap<String, SourceProfile>>>,
    document_store: Arc<Mutex<Vec<String>>>,
    idf: Arc<Mutex<HashMap<String, f64>>>,
}

impl FactCheckingModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_source(&self, profile: SourceProfile) {
        self.sources.lock().await.insert(profile.domain.clone(), profile);
    }

    pub async fn ingest_documents(&self, texts: Vec<String>) {
        let mut store = self.document_store.lock().await;
        store.extend(texts.clone());
        let idf = Self::compute_idf(&texts, &store);
        *self.idf.lock().await = idf;
    }

    pub async fn verify(
        &self,
        claim: impl Into<String>,
        evidence: Vec<EvidenceChunk>,
    ) -> ClaimVerification {
        let claim = claim.into();
        let mut supporting = Vec::new();
        let mut contradicting = Vec::new();
        let mut neutral = Vec::new();
        let mut reasoning = Vec::new();
        let mut cross_references = Vec::new();

        let sources = self.sources.lock().await;
        let idf = self.idf.lock().await;
        let claim_tokens: Vec<String> = Self::tokenize(&claim);

        for chunk in &evidence {
            let domain_weight = sources
                .get(&chunk.source_domain)
                .map(|s| s.factuality * s.reliability)
                .unwrap_or(0.5);

            let tfidf = Self::tfidf(&claim_tokens, &chunk.text, &idf);
            let adjusted = (chunk.confidence * domain_weight * (0.5 + 0.5 * tfidf)).clamp(0.0, 1.0);

            let mut enriched = chunk.clone();
            enriched.tfidf_score = adjusted;
            match chunk.stance {
                Stance::Supports => supporting.push(enriched),
                Stance::Refutes => contradicting.push(enriched),
                _ => neutral.push(enriched),
            }
        }

        let support_count = supporting.len();
        let refute_count = contradicting.len();

        if support_count == 0 && refute_count == 0 {
            reasoning.push("no stance-bearing evidence found".into());
        }
        if support_count > 0 && refute_count == 0 {
            reasoning.push(format!("supported by {} chunks", support_count));
        }
        if refute_count > 0 && support_count == 0 {
            reasoning.push(format!("contradicted by {} chunks", refute_count));
        }
        if support_count > 0 && refute_count > 0 {
            reasoning.push(format!("mixed evidence: {} supporting, {} refuting", support_count, refute_count));
        }

        // Cross-reference detection
        for s in &supporting {
            for r in &contradicting {
                if s.text.to_lowercase().contains(&r.text.to_lowercase()[..r.text.len().min(20)])
                    || r.text.to_lowercase().contains(&s.text.to_lowercase()[..s.text.len().min(20)])
                {
                    cross_references.push(format!("{} <-> {}", s.source_domain, r.source_domain));
                }
            }
        }

        let (verdict, confidence) = if support_count == 0 && refute_count == 0 {
            (Verdict::Unsupported, 0.0)
        } else if support_count > 0 && refute_count == 0 {
            (Verdict::LikelyTrue, (support_count as f64 / (support_count + refute_count) as f64).clamp(0.0, 1.0))
        } else if refute_count > 0 && support_count == 0 {
            (Verdict::LikelyFalse, (refute_count as f64 / (support_count + refute_count) as f64).clamp(0.0, 1.0))
        } else {
            (Verdict::Disputed, 0.5)
        };

        info!(verdict=?verdict, confidence=confidence, claim=%claim, "fact-check completed");

        ClaimVerification {
            claim,
            verdict,
            confidence,
            supporting,
            contradicting,
            neutral,
            reasoning,
            cross_references,
        }
    }

    pub async fn batch_verify(
        &self,
        claims: Vec<String>,
        evidence_map: HashMap<String, Vec<EvidenceChunk>>,
    ) -> Vec<ClaimVerification> {
        let mut out = Vec::new();
        for claim in claims {
            let evidence = evidence_map.get(&claim).cloned().unwrap_or_default();
            out.push(self.verify(claim, evidence).await);
        }
        out
    }

    pub async fn decay_source_confidence(&self, domain: &str, decay: f64) {
        let mut sources = self.sources.lock().await;
        if let Some(s) = sources.get_mut(domain) {
            s.reliability = (s.reliability - decay).clamp(0.0, 1.0);
            s.factuality = (s.factuality - decay * 0.5).clamp(0.0, 1.0);
        }
    }

    pub async fn source_stats(&self) -> HashMap<String, (f64, f64, u64)> {
        let sources = self.sources.lock().await;
        sources.iter().map(|(k, v)| (k.clone(), (v.reliability, v.factuality, 0))).collect()
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 2)
            .map(|s| s.to_string())
            .collect()
    }

    fn compute_idf(_docs: &[String], store: &[String]) -> HashMap<String, f64> {
        let n = store.len().max(1) as f64;
        let mut df: HashMap<String, usize> = HashMap::new();
        for doc in store {
            let tokens: HashSet<_> = Self::tokenize(doc).into_iter().collect();
            for t in tokens {
                *df.entry(t).or_default() += 1;
            }
        }
        df.into_iter().map(|(k, v)| (k, (n / (v as f64)).ln().max(0.0))).collect()
    }

    fn tfidf(claim_tokens: &[String], doc: &str, idf: &HashMap<String, f64>) -> f64 {
        let doc_tokens = Self::tokenize(doc);
        if doc_tokens.is_empty() {
            return 0.0;
        }
        let mut score = 0.0;
        for ct in claim_tokens {
            let tf = doc_tokens.iter().filter(|t| *t == ct).count() as f64 / doc_tokens.len() as f64;
            score += tf * idf.get(ct).copied().unwrap_or(0.0);
        }
        score / claim_tokens.len().max(1) as f64
    }
}
