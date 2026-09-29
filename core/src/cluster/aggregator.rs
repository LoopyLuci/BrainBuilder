//! Host-side gradient accumulation for a distributed training step: collects
//! each joined client's `SubmitGradients` for the current step and produces
//! the plain average once every expected client has reported in. Pure data
//! aggregation — no networking here (that's `runtime::distributed`), no
//! Python/tensor-runtime dependency (that's `PythonBridge`) — so it's real,
//! deterministic, and independently testable.
use crate::cluster::protocol::TensorPayload;
use std::collections::HashMap;
use uuid::Uuid;

pub struct GradientAggregator {
    step: u64,
    expected_clients: usize,
    received: HashMap<Uuid, HashMap<String, TensorPayload>>,
}

impl GradientAggregator {
    pub fn new(step: u64, expected_clients: usize) -> Self {
        Self { step, expected_clients, received: HashMap::new() }
    }

    pub fn step(&self) -> u64 {
        self.step
    }

    /// Records one client's gradients for the current step. Returns `false`
    /// (and drops the submission) if it's for a different step than this
    /// aggregator is collecting — a slow/stale client can't corrupt the
    /// current round's average.
    pub fn submit(&mut self, client: Uuid, step: u64, gradients: HashMap<String, TensorPayload>) -> bool {
        if step != self.step {
            return false;
        }
        self.received.insert(client, gradients);
        true
    }

    pub fn is_ready(&self) -> bool {
        self.received.len() >= self.expected_clients
    }

    pub fn received_count(&self) -> usize {
        self.received.len()
    }

    /// Plain (unweighted) average across every client that has reported in
    /// so far. Real element-wise math, fails on a shape mismatch rather than
    /// silently producing garbage — a client with a divergent local graph
    /// (bug or malicious) can't corrupt the average unnoticed.
    pub fn average(&self) -> Result<HashMap<String, TensorPayload>, String> {
        if self.received.is_empty() {
            return Err("no gradients submitted for this step yet".to_string());
        }
        let mut port_names: Vec<&String> = Vec::new();
        for grads in self.received.values() {
            for name in grads.keys() {
                if !port_names.contains(&name) {
                    port_names.push(name);
                }
            }
        }

        let n = self.received.len() as f32;
        let mut averaged = HashMap::new();
        for name in port_names {
            let mut sum: Option<Vec<f32>> = None;
            let mut shape: Option<Vec<i64>> = None;
            for grads in self.received.values() {
                let payload = grads
                    .get(name)
                    .ok_or_else(|| format!("client submitted no gradient for port `{name}`"))?;
                match (&mut sum, &shape) {
                    (None, _) => {
                        shape = Some(payload.shape.clone());
                        sum = Some(payload.data.clone());
                    }
                    (Some(acc), Some(expected_shape)) => {
                        if &payload.shape != expected_shape {
                            return Err(format!(
                                "gradient shape mismatch for port `{name}`: {:?} vs {:?}",
                                payload.shape, expected_shape
                            ));
                        }
                        if acc.len() != payload.data.len() {
                            return Err(format!("gradient length mismatch for port `{name}`"));
                        }
                        for (a, b) in acc.iter_mut().zip(&payload.data) {
                            *a += b;
                        }
                    }
                    _ => unreachable!(),
                }
            }
            let mut data = sum.expect("at least one submission guaranteed by outer loop");
            for v in &mut data {
                *v /= n;
            }
            averaged.insert(name.clone(), TensorPayload { shape: shape.expect("shape set alongside sum"), data });
        }
        Ok(averaged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(values: &[f32]) -> TensorPayload {
        TensorPayload { shape: vec![values.len() as i64], data: values.to_vec() }
    }

    #[test]
    fn averages_two_real_clients_gradients() {
        let mut agg = GradientAggregator::new(0, 2);
        let mut a = HashMap::new();
        a.insert("w".to_string(), payload(&[2.0, 4.0]));
        let mut b = HashMap::new();
        b.insert("w".to_string(), payload(&[4.0, 8.0]));

        assert!(agg.submit(Uuid::new_v4(), 0, a));
        assert!(!agg.is_ready());
        assert!(agg.submit(Uuid::new_v4(), 0, b));
        assert!(agg.is_ready());

        let avg = agg.average().unwrap();
        assert_eq!(avg["w"].data, vec![3.0, 6.0]);
    }

    #[test]
    fn rejects_a_submission_for_a_stale_step() {
        let mut agg = GradientAggregator::new(5, 1);
        let mut grads = HashMap::new();
        grads.insert("w".to_string(), payload(&[1.0]));
        assert!(!agg.submit(Uuid::new_v4(), 4, grads));
        assert_eq!(agg.received_count(), 0);
    }

    #[test]
    fn rejects_a_shape_mismatch_instead_of_producing_garbage() {
        let mut agg = GradientAggregator::new(0, 2);
        let mut a = HashMap::new();
        a.insert("w".to_string(), payload(&[1.0, 2.0]));
        let mut b = HashMap::new();
        b.insert("w".to_string(), payload(&[1.0, 2.0, 3.0]));
        agg.submit(Uuid::new_v4(), 0, a);
        agg.submit(Uuid::new_v4(), 0, b);
        assert!(agg.average().is_err());
    }
}
