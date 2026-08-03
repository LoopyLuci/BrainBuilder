//! Hyperdimensional Computing (HDC) binary vector computing model.
//!
//! Production-grade deterministic implementation: high-dimensional binary
//! vectors, bundling, binding, permutation, and similarity search.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcVector {
    pub dim: usize,
    pub bits: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcItemMemory {
    pub symbols: std::collections::HashMap<String, HdcVector>,
}

impl Default for HdcItemMemory {
    fn default() -> Self {
        Self { symbols: std::collections::HashMap::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdcQueryResult {
    pub matched_symbol: String,
    pub similarity: f64,
    pub top_k: Vec<(String, f64)>,
}

#[derive(Clone, Default)]
pub struct HyperdimensionalComputingModel {
    dim: usize,
    item_memory: HdcItemMemory,
}

impl HyperdimensionalComputingModel {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            item_memory: HdcItemMemory { symbols: std::collections::HashMap::new() },
        }
    }

    pub fn random_vector(&self, seed: u64) -> HdcVector {
        let mut bits = Vec::with_capacity(self.dim);
        let mut s = seed;
        for _ in 0..self.dim {
            s = s.wrapping_mul(1103515245).wrapping_add(12345);
            bits.push(s & 1 == 1);
        }
        HdcVector { dim: self.dim, bits }
    }

    pub fn level_vector(&self, level: usize, max_level: usize) -> HdcVector {
        let mut bits = vec![false; self.dim];
        let threshold = (level * self.dim) / max_level.max(1);
        for i in 0..threshold.min(self.dim) {
            bits[i] = true;
        }
        HdcVector { dim: self.dim, bits }
    }

    pub fn bind(&self, a: &HdcVector, b: &HdcVector) -> HdcVector {
        let len = a.dim.min(b.dim).min(self.dim);
        let mut bits = Vec::with_capacity(len);
        for i in 0..len {
            bits.push(a.bits.get(i).copied().unwrap_or(false) ^ b.bits.get(i).copied().unwrap_or(false));
        }
        HdcVector { dim: len, bits }
    }

    pub fn bundle(&self, vectors: &[HdcVector]) -> HdcVector {
        if vectors.is_empty() || self.dim == 0 {
            return HdcVector { dim: self.dim, bits: vec![false; self.dim] };
        }
        let len = vectors[0].dim.min(self.dim);
        let mut bits = vec![false; len];
        for v in vectors {
            for i in 0..len.min(v.dim) {
                bits[i] = bits[i] ^ v.bits.get(i).copied().unwrap_or(false);
            }
        }
        HdcVector { dim: len, bits }
    }

    pub fn permute(&self, v: &HdcVector, shift: usize) -> HdcVector {
        if v.bits.is_empty() {
            return v.clone();
        }
        let n = shift % v.bits.len();
        let mut bits = v.bits.clone();
        bits.rotate_right(n);
        HdcVector { dim: v.dim, bits }
    }

    pub fn similarity(&self, a: &HdcVector, b: &HdcVector) -> f64 {
        if a.bits.is_empty() || b.bits.is_empty() {
            return 0.0;
        }
        let len = a.bits.len().min(b.bits.len());
        let mut same = 0;
        for i in 0..len {
            if a.bits[i] == b.bits[i] {
                same += 1;
            }
        }
        same as f64 / len as f64
    }

    pub fn register_symbol(&mut self, symbol: String, vector: HdcVector) {
        self.item_memory.symbols.insert(symbol, vector);
    }

    pub fn query(&self, query: &HdcVector, top_k: usize) -> HdcQueryResult {
        let mut scores: Vec<(String, f64)> = self.item_memory.symbols.iter()
            .map(|(sym, vec)| (sym.clone(), self.similarity(query, vec)))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top: Vec<(String, f64)> = scores.into_iter().take(top_k).collect();
        let best = top.first().cloned().unwrap_or(("unknown".into(), 0.0));
        HdcQueryResult {
            matched_symbol: best.0,
            similarity: best.1,
            top_k: top,
        }
    }
}
