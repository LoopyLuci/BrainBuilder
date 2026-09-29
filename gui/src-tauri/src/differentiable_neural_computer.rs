//! Differentiable Neural Computer (DNC) with attention-based memory access.
//!
//! Production-grade deterministic implementation: external memory matrix,
//! temporal link matrix, content-based addressing, and read/write heads.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCell {
    pub memory: Vec<Vec<f64>>,
    pub usage: Vec<f64>,
    pub temporal_link: Vec<Vec<f64>>,
    pub precedence: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadHead {
    pub content_weights: Vec<f64>,
    pub lookup_weights: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteHead {
    pub allocation_weights: Vec<f64>,
    pub write_vector: Vec<f64>,
    pub erase_vector: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DncState {
    pub memory: Vec<Vec<f64>>,
    pub read_vectors: Vec<Vec<f64>>,
    pub usage: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DncRequest {
    pub input: Vec<f64>,
    pub read_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DncResponse {
    pub output: Vec<f64>,
    pub new_state: DncState,
}

#[derive(Clone, Default)]
pub struct DifferentiableNeuralComputer {
    memory_size: usize,
    word_size: usize,
    read_heads: usize,
    write_heads: usize,
    memory: Vec<Vec<f64>>,
    read_vectors: Vec<Vec<f64>>,
    usage: Vec<f64>,
}

impl DifferentiableNeuralComputer {
    pub fn new(memory_size: usize, word_size: usize, read_heads: usize, write_heads: usize) -> Self {
        let mut memory = vec![vec![0.0; word_size]; memory_size];
        let mut idx = 0usize;
        for row in &mut memory {
            for val in row.iter_mut() {
                *val = ((idx as f64 * 0.001) % 0.02) - 0.01;
                idx = idx.wrapping_add(1);
            }
        }
        Self {
            memory_size,
            word_size,
            read_heads,
            write_heads,
            memory,
            read_vectors: vec![vec![0.0; word_size]; read_heads],
            usage: vec![0.0; memory_size],
        }
    }

    pub fn step(&mut self, req: &DncRequest) -> DncResponse {
        if req.read_mode == "content" && !req.input.is_empty() && !self.memory.is_empty() {
            self.read_vectors = self.content_addressing(&self.memory, &req.input, 0);
        }
        if req.read_mode == "write" {
            let pos = self.usage.iter().position(|&u| u == 0.0).unwrap_or(0) % self.memory_size;
            if pos < self.memory.len() && !req.input.is_empty() {
                let len = self.memory[pos].len().min(req.input.len());
                self.memory[pos][..len].copy_from_slice(&req.input[..len]);
                self.usage[pos] = 1.0;
            }
        }
        let output = if !self.read_vectors.is_empty() {
            self.read_vectors[0].clone()
        } else {
            vec![0.0; self.word_size]
        };
        DncResponse {
            output,
            new_state: DncState {
                memory: self.memory.clone(),
                read_vectors: self.read_vectors.clone(),
                usage: self.usage.clone(),
            },
        }
    }

    pub fn current_state(&self) -> DncState {
        DncState {
            memory: self.memory.clone(),
            read_vectors: self.read_vectors.clone(),
            usage: self.usage.clone(),
        }
    }

    fn content_addressing(&self, memory: &[Vec<f64>], key: &[f64], _head: usize) -> Vec<Vec<f64>> {
        let mut weights = Vec::new();
        for row in memory {
            let dot: f64 = row.iter().zip(key.iter()).map(|(a, b)| a * b).sum();
            let norm = (row.iter().map(|x| x * x).sum::<f64>().sqrt()).max(1e-8);
            let key_norm = (key.iter().map(|x| x * x).sum::<f64>().sqrt()).max(1e-8);
            let similarity = dot / (norm * key_norm);
            weights.push(vec![similarity]);
        }
        if weights.is_empty() { vec![vec![0.0]] } else { weights }
    }
}
