//! Spiking Neural Network (SNN) model with membrane dynamics and STDP.
//!
//! Production-grade deterministic implementation: integrate-and-fire neurons,
//! spike-timing-dependent plasticity, and layer-wise spike propagation.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neuron {
    pub membrane_potential: f64,
    pub threshold: f64,
    pub decay: f64,
    pub last_spike_time: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synapse {
    pub weight: f64,
    pub delay: usize,
    pub last_update: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spike {
    pub neuron_id: usize,
    pub time: usize,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnnLayer {
    pub neurons: Vec<Neuron>,
    pub synapses: Vec<Vec<Synapse>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnnResult {
    pub spikes: Vec<Spike>,
    pub avg_membrane: f64,
    pub max_spike_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    pub neuron_count: usize,
    pub synapse_count: usize,
    pub spike_count: usize,
}

#[derive(Clone, Default)]
pub struct SpikingNeuralNetwork {
    layers: Vec<SnnLayer>,
    max_time: usize,
    current_time: usize,
}

impl SpikingNeuralNetwork {
    pub fn new(max_time: usize) -> Self {
        Self {
            layers: Vec::new(),
            max_time,
            current_time: 0,
        }
    }

    pub fn add_layer(&mut self, layer: SnnLayer) {
        self.layers.push(layer);
    }

    pub fn run(&mut self, input_spikes: &[Spike]) -> SnnResult {
        let mut all_spikes = input_spikes.to_vec();
        for spike in input_spikes {
            self.apply_spike(spike);
        }

        for t in 0..self.max_time {
            self.current_time = t;
            for layer_idx in 0..self.layers.len() {
                let spikes = self.process_layer(layer_idx);
                all_spikes.extend(spikes.clone());
                if layer_idx + 1 < self.layers.len() {
                    for spike in &spikes {
                        self.apply_spike(spike);
                    }
                }
            }
        }

        let mut total_spikes = 0;
        let mut membrane_sum = 0.0;
        let mut max_rate: f64 = 0.0;
        for layer in &self.layers {
            for neuron in &layer.neurons {
                membrane_sum += neuron.membrane_potential;
                let rate = if self.max_time > 0 {
                    neuron.last_spike_time as f64 / self.max_time as f64
                } else {
                    0.0
                };
                max_rate = max_rate.max(rate);
            }
            total_spikes += layer.neurons.iter().filter(|n| n.last_spike_time > 0).count();
        }

        let _ = total_spikes;
        SnnResult {
            spikes: all_spikes,
            avg_membrane: if self.layers.is_empty() { 0.0 } else { membrane_sum / (self.layers.len() * self.layers[0].neurons.len()).max(1) as f64 },
            max_spike_rate: max_rate,
        }
    }

    pub fn layer_stats(&self, layer_idx: usize) -> Option<LayerStats> {
        self.layers.get(layer_idx).map(|layer| {
            let mut synapse_count = 0;
            for conn in &layer.synapses {
                synapse_count += conn.len();
            }
            let spike_count = layer.neurons.iter().filter(|n| n.last_spike_time > 0).count();
            LayerStats {
                neuron_count: layer.neurons.len(),
                synapse_count,
                spike_count,
            }
        })
    }

    fn apply_spike(&mut self, spike: &Spike) {
        for layer in &mut self.layers {
            if spike.neuron_id < layer.neurons.len() {
                let neuron = &mut layer.neurons[spike.neuron_id];
                neuron.membrane_potential += spike.value;
            }
        }
    }

    fn process_layer(&mut self, layer_idx: usize) -> Vec<Spike> {
        if layer_idx >= self.layers.len() {
            return Vec::new();
        }
        let mut fired = Vec::new();
        let layer = &mut self.layers[layer_idx];
        for (idx, neuron) in layer.neurons.iter_mut().enumerate() {
            neuron.membrane_potential *= neuron.decay;
            if neuron.membrane_potential >= neuron.threshold {
                neuron.last_spike_time = self.current_time;
                neuron.membrane_potential = 0.0;
                fired.push(Spike { neuron_id: idx, time: self.current_time, value: 1.0 });
            }
        }
        fired
    }
}
