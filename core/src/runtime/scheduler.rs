use crate::bbir::BBIRGraph;
use crate::component::descriptor::ShapeExpr;
use crate::runtime::device::Device;
use crate::AppContext;
use crate::Result;
use crate::Tensor;
use crate::interop::arena::SharedArena;
use crate::interop::python::PythonBridge;
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

/// Compiled execution plan: resolves which device runs each op.
#[derive(Clone)]
pub struct ExecutionPlan {
    pub epochs: usize,
    pub graph: BBIRGraph,
    pub operations: Vec<ExecutableOp>,
    pub device_assignments: std::collections::HashMap<String, String>,
    /// Parameter ports pre-seeded from a real pretrained file (transfer
    /// learning): keyed by the same namespaced port name the trainer binds
    /// (`node_id:port`), value is the loaded tensor. The trainer starts its
    /// weight map from these instead of random-initializing them; for a port
    /// the descriptor marks non-trainable (e.g. `lora_linear`'s base
    /// `weight`), the optimizer never touches it, so it stays frozen — a real
    /// frozen pretrained backbone with trainable adapters/head around it.
    pub preset_weights: std::collections::HashMap<String, Tensor>,
    arena: Arc<SharedArena>,
}

#[derive(Clone)]
pub struct ExecutableOp {
    pub node_id: String,
    pub component: String,
    pub language: String,
    pub entry: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    /// Subset of `inputs` that are learnable parameters (role = Parameter in
    /// the component descriptor), e.g. `linear`'s `weight`. These are never
    /// wired by an edge or fed from the dataset — the trainer auto-initializes
    /// and optimizes them.
    pub param_inputs: Vec<String>,
    /// Subset of `param_inputs` the optimizer actually updates (excludes
    /// frozen parameters, e.g. `lora_linear`'s base `weight` — see
    /// `ComponentDescriptor::trainable_parameter_ports`).
    pub trainable_inputs: Vec<String>,
    /// Subset of `param_inputs` that should be freshly initialized with
    /// zeros rather than `torch.randn` (e.g. LoRA's `lora_b` — see
    /// `ComponentDescriptor::zero_init_parameter_ports`).
    pub zero_init_inputs: Vec<String>,
    /// This node's hyperparameter values (from the BBIR graph, e.g.
    /// `{"eps": 1e-5}` for `layernorm`), forwarded to the Python worker so a
    /// component's `forward` can accept them as keyword arguments. Also used
    /// at compile time (see `resolved_param_shapes`) to size a learnable
    /// parameter concretely instead of guessing from data shape alone.
    pub hyperparams: serde_json::Value,
    /// Concrete shape for each of this op's `param_inputs`, resolved from the
    /// descriptor's declared shape expression + this node's hyperparameters
    /// (e.g. `layernorm`'s `weight: [:features]` with hyperparam
    /// `features: 64` resolves to `[64]`). `None` for a port whose shape
    /// couldn't be fully resolved (missing hyperparam, or an expression too
    /// complex to evaluate statically) — those fall back to the older
    /// data-shape heuristic in `ExecutionPlan::train_step`.
    pub resolved_param_shapes: std::collections::HashMap<String, Vec<i64>>,
}

/// `edn_rs`'s own `Edn::to_json()` (what `interop::edn_value::edn_to_json`
/// delegates to, used for every `ShapeExpr::Symbolic` value) camelCases
/// dash-separated EDN keywords: `:vocab-size` comes out as the *string*
/// `"vocabSize"`, not `"vocab-size"` — verified directly against the real
/// parsed descriptor (`registry.get_by_name("embedding").inputs`), not
/// assumed. `components/*.edn`'s hyperparameter names are snake_case
/// (`vocab_size`, `out_features`, ...), so a shape symbol has to be
/// converted back before it can look anything up.
pub(crate) fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if c.is_uppercase() {
            out.push('_');
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Resolves a declared parameter shape expression against a node's
/// hyperparameters. A symbolic dim like `:out-features` (which arrives here
/// as the camelCased string `"outFeatures"` — see `camel_to_snake`) looks up
/// hyperparam `out_features`; a fixed numeric literal is used as-is. Returns
/// `None` (rather than a partially-wrong shape) the moment any dimension
/// can't be resolved to a positive integer, so callers can fall back to a
/// different strategy instead of silently training with a malformed weight.
pub(crate) fn resolve_shape_expr(shape: &ShapeExpr, hyperparams: &serde_json::Value) -> Option<Vec<i64>> {
    resolve_shape_dims_partial(shape, hyperparams)?.into_iter().collect()
}

/// A single symbolic-or-fixed dimension resolved against a node's
/// hyperparameters, or `None` if it names something hyperparameters don't
/// define (e.g. `:batch`, which is only known once real data flows through
/// the graph at runtime, never a hyperparameter).
fn resolve_dim(dim: &serde_json::Value, hyperparams: &serde_json::Value) -> Option<i64> {
    match dim {
        serde_json::Value::Number(n) => n.as_i64().filter(|&v| v > 0),
        serde_json::Value::String(s) => {
            let key = camel_to_snake(s.trim_start_matches(':'));
            hyperparams.get(&key)?.as_i64().filter(|&v| v > 0)
        }
        _ => None,
    }
}

/// Resolves every dimension of `shape` independently, rather than bailing
/// out entirely the moment one fails (`resolve_shape_expr`'s behavior,
/// which is right for weight-tensor allocation but wrong for shape
/// *validation*: an I/O shape like `[:batch, :out-features]` will almost
/// always have an unresolvable `:batch`, and a validator that gives up on
/// the whole shape because of it would never check `:out-features` either).
/// Returns `None` only if `shape` isn't dimension-array-shaped at all (a
/// malformed symbolic descriptor); otherwise one `Option<i64>` per
/// dimension, `None` at positions that just aren't hyperparameter-resolvable
/// (e.g. `:batch`) rather than for the whole shape.
pub(crate) fn resolve_shape_dims_partial(
    shape: &ShapeExpr,
    hyperparams: &serde_json::Value,
) -> Option<Vec<Option<i64>>> {
    match shape {
        ShapeExpr::Fixed(dims) => Some(dims.iter().map(|d| Some(*d as i64)).collect()),
        ShapeExpr::Symbolic(value) => {
            let dims = value.as_array()?;
            Some(dims.iter().map(|d| resolve_dim(d, hyperparams)).collect())
        }
    }
}

#[cfg(test)]
mod shape_resolution_tests {
    use super::*;

    #[test]
    fn resolves_a_camel_cased_symbolic_dim_against_a_snake_case_hyperparam() {
        // Regression test for the real bug this fixes: edn_rs's to_json()
        // camelCases `:vocab-size` into the string "vocabSize", which must
        // be converted back to match the `vocab_size` hyperparameter key —
        // caught by actually training an embedding+linear graph
        // (sequence_training.rs), not by inspection.
        let shape = ShapeExpr::Symbolic(serde_json::json!(["vocabSize", "embeddingDim"]));
        let hp = serde_json::json!({"vocab_size": 8, "embedding_dim": 16});
        assert_eq!(resolve_shape_expr(&shape, &hp), Some(vec![8, 16]));
    }

    #[test]
    fn returns_none_when_a_hyperparam_is_missing_rather_than_guessing() {
        let shape = ShapeExpr::Symbolic(serde_json::json!(["outFeatures", "inFeatures"]));
        assert_eq!(resolve_shape_expr(&shape, &serde_json::json!({"out_features": 4})), None);
    }

    // The DSpark drafter components carry rank-3 parameter shapes (e.g.
    // parallel_intern's heads_weight [:draft-len :vocab :features]). Prove every
    // one of their parameter ports resolves to a concrete, fully-sized tensor
    // from the descriptor's default hyperparameters — i.e. they are trainable
    // when wired, not just placeable, and the resolver handles >2-D shapes.
    #[test]
    fn dspark_component_parameter_shapes_resolve_to_concrete_dims() {
        use crate::component::registry::ComponentRegistry;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        let mut reg = ComponentRegistry::new();
        reg.load_from_dir(&dir).expect("load real components");

        // (component, hyperparams, expected concrete shape per parameter port)
        let cases: &[(&str, serde_json::Value, &[(&str, Vec<i64>)])] = &[
            (
                "parallel_intern",
                serde_json::json!({ "features": 256, "vocab": 1000, "draft_len": 8 }),
                &[("heads_weight", vec![8, 1000, 256])],
            ),
            (
                "low_rank_markov_head",
                serde_json::json!({ "features": 256, "rank": 32, "vocab": 1000 }),
                &[("compress_weight", vec![32, 256]), ("expand_weight", vec![1000, 32])],
            ),
            (
                "confidence_head",
                serde_json::json!({ "features": 256, "proj": 128 }),
                &[("hidden_weight", vec![128, 256]), ("score_weight", vec![1, 128])],
            ),
        ];

        for (name, hp, expected) in cases {
            let desc = reg.get_by_name(name).unwrap_or_else(|| panic!("{name} should be registered"));
            for (port_name, want) in *expected {
                let port = desc
                    .inputs
                    .iter()
                    .find(|p| p.name == *port_name)
                    .unwrap_or_else(|| panic!("{name} has no port {port_name}"));
                let resolved = resolve_shape_expr(&port.tensor.shape, hp);
                assert_eq!(resolved.as_deref(), Some(&want[..]), "{name}.{port_name} shape");
            }
        }
    }
}

impl ExecutionPlan {
    pub fn prepare_inputs(&self, batch: RecordBatch) -> Result<Vec<Tensor>> {
        crate::runtime::arrow_bridge::record_batch_to_tensors(&batch, &self.arena)
    }

    /// Inference-only forward pass. `data_inputs` feed the graph's leaf data
    /// ports positionally; `weights` supply the learnable parameter ports
    /// (e.g. loaded from a checkpoint). Errors if a parameter port has no
    /// supplied weight — you can't run a parameterized graph without its
    /// trained weights.
    pub fn forward(
        &self,
        data_inputs: &[Tensor],
        weights: &std::collections::HashMap<String, Tensor>,
        bridge: &PythonBridge,
        device: &dyn Device,
    ) -> Result<Vec<Tensor>> {
        let mut intermediate = self.bind_data_ports(data_inputs.to_vec())?;
        for port in self.weight_ports() {
            let tensor = weights.get(&port).cloned().ok_or_else(|| {
                crate::interop::protocol::BrainBuilderError::ConfigError(format!(
                    "forward pass needs weight for parameter port `{port}` — load a trained \
                     checkpoint first"
                ))
            })?;
            intermediate.insert(port, tensor);
        }

        for op in &self.operations {
            let op_inputs: Vec<Tensor> = op
                .inputs
                .iter()
                .filter_map(|name| intermediate.get(name).cloned())
                .collect();
            let result = match op.language.as_str() {
                "python" => {
                    // Real predictions must be deterministic, not randomly
                    // dropping units the way a training step does — see
                    // `dropout.py`'s `training` kwarg. Components with no
                    // `training` parameter of their own silently ignore this
                    // (`call_with_hyperparams` filters kwargs by the
                    // callee's actual signature).
                    let mut hyperparams = op.hyperparams.clone();
                    if let Some(obj) = hyperparams.as_object_mut() {
                        obj.insert("training".to_string(), serde_json::Value::Bool(false));
                    }
                    bridge.call_component(&op.component, &op.entry, op_inputs, &hyperparams)?
                }
                "rust" => {
                    let refs: Vec<&Tensor> = op_inputs.iter().collect();
                    device.exec(&op.entry, &refs)?
                }
                other => {
                    return Err(crate::interop::protocol::BrainBuilderError::UnsupportedLanguage(
                        other.to_string(),
                    ))
                }
            };
            for out_name in &op.outputs {
                intermediate.insert(out_name.clone(), result.clone());
            }
        }
        Ok(intermediate.into_values().collect())
    }

    /// The port name of the graph's final output — the last op's first
    /// declared output. (BBIR doesn't have a distinct "graph output" concept
    /// beyond node ports, so this mirrors `forward`'s existing simplification
    /// rather than inventing a new one.)
    fn output_port(&self) -> Option<&str> {
        self.operations.last().and_then(|op| op.outputs.first()).map(String::as_str)
    }

    /// Binds prepared batch tensors (one per dataset column) onto the
    /// graph's data ports. If there's exactly one data port but multiple
    /// columns, they're stacked into one real `(batch, features)` matrix
    /// (what `linear`/`conv2d`-style components need); otherwise columns map
    /// onto ports 1:1 positionally (elementwise components like `scale`).
    fn bind_data_ports(
        &self,
        batch_tensors: Vec<Tensor>,
    ) -> Result<std::collections::HashMap<String, Tensor>> {
        let data_ports = self.data_ports();
        let err = |msg: String| crate::interop::protocol::BrainBuilderError::ConfigError(msg);

        if data_ports.len() == 1 && batch_tensors.len() > 1 {
            let matrix = crate::runtime::arrow_bridge::stack_columns_into_matrix(&batch_tensors, &self.arena)?;
            return Ok(std::collections::HashMap::from([(data_ports[0].clone(), matrix)]));
        }
        if batch_tensors.len() != data_ports.len() {
            return Err(err(format!(
                "batch has {} data column(s) but the graph declares {} data port(s)",
                batch_tensors.len(),
                data_ports.len()
            )));
        }
        Ok(data_ports.into_iter().zip(batch_tensors).collect())
    }

    /// "Leaf" data input ports fed from the dataset: input ports that are
    /// neither produced by another op's output nor a learnable parameter.
    /// Parameter classification comes from the component descriptor's port
    /// roles (`ExecutableOp::param_inputs`), so this is correct even for
    /// multi-input ops like `linear(input, weight)` where `input` is data and
    /// `weight` is a parameter.
    fn data_ports(&self) -> Vec<String> {
        let produced: std::collections::HashSet<&String> =
            self.operations.iter().flat_map(|op| op.outputs.iter()).collect();
        let mut data_ports = Vec::new();
        for op in &self.operations {
            for port in &op.inputs {
                let is_param = op.param_inputs.contains(port);
                if !is_param && !produced.contains(port) && !data_ports.contains(port) {
                    data_ports.push(port.clone());
                }
            }
        }
        data_ports
    }

    /// Learnable parameter ports across all ops (role = Parameter), lazily
    /// initialized and optimized by the trainer.
    fn weight_ports(&self) -> Vec<String> {
        let mut weight_ports: Vec<String> = Vec::new();
        for op in &self.operations {
            for port in &op.param_inputs {
                if !weight_ports.contains(port) {
                    weight_ports.push(port.clone());
                }
            }
        }
        weight_ports
    }

    /// Subset of `weight_ports()` the optimizer actually updates — see
    /// `ExecutableOp::trainable_inputs`. Frozen parameters (LoRA's base
    /// `weight`) are still auto-initialized and bound into every forward
    /// pass by `weight_ports()`/the init loop in `train_step`, but must not
    /// be handed to the optimizer or the worker will train them anyway.
    fn trainable_weight_ports(&self) -> Vec<String> {
        let mut ports: Vec<String> = Vec::new();
        for op in &self.operations {
            for port in &op.trainable_inputs {
                if !ports.contains(port) {
                    ports.push(port.clone());
                }
            }
        }
        ports
    }

    /// The concrete shape resolved at compile time for a parameter port (see
    /// `resolve_shape_expr`), if its descriptor + this node's hyperparameters
    /// fully determined it.
    fn resolved_shape_for(&self, port: &str) -> Option<Vec<i64>> {
        self.operations
            .iter()
            .find_map(|op| op.resolved_param_shapes.get(port))
            .cloned()
    }

    /// Whether `port` is declared `:zero-init true` (e.g. LoRA's `lora_b`).
    fn is_zero_init(&self, port: &str) -> bool {
        self.operations.iter().any(|op| op.zero_init_inputs.iter().any(|p| p == port))
    }

    /// Runs one real forward -> loss -> backward -> optimizer-step training
    /// iteration (see `PythonBridge::train_step` for why this can't be
    /// decomposed into separate forward/backward calls: DLPack doesn't carry
    /// autograd history across the Rust<->Python boundary).
    ///
    /// The batch's *last* column is treated as the training target and the
    /// rest as the graph's data input, by convention — `DataSourceConfig`
    /// doesn't have a formal "label column" field yet to make this explicit.
    /// `weights` is threaded through by the caller across the whole training
    /// loop (epochs x batches) so learned parameters persist between steps;
    /// entries are lazily initialized here via real `torch.randn` on first
    /// use.
    #[allow(clippy::too_many_arguments)]
    pub fn train_step(
        &self,
        step: usize,
        mut batch_tensors: Vec<Tensor>,
        weights: &mut std::collections::HashMap<String, Tensor>,
        loss_name: &str,
        optimizer_name: &str,
        lr: f64,
        weight_decay: f64,
        grad_clip: f64,
        label_smoothing: f64,
        momentum: f64,
        bridge: &PythonBridge,
    ) -> Result<LossValue> {
        let err = |msg: &str| crate::interop::protocol::BrainBuilderError::ConfigError(msg.to_string());
        let target = batch_tensors.pop().ok_or_else(|| err("training batch is empty"))?;
        let mut inputs = self.bind_data_ports(batch_tensors)?;
        self.bind_weights_lazy_init(&mut inputs, weights, bridge)?;

        let output_port = self
            .output_port()
            .ok_or_else(|| err("execution plan has no operations / output port"))?;

        let (loss_value, updated_weights) = bridge.train_step(
            &self.operations,
            inputs,
            output_port,
            &target,
            loss_name,
            optimizer_name,
            lr,
            weight_decay,
            grad_clip,
            label_smoothing,
            momentum,
            &self.trainable_weight_ports(),
        )?;

        weights.extend(updated_weights);

        Ok(LossValue { step, value: loss_value })
    }

    /// Forward -> loss -> backward, stopping short of the optimizer step —
    /// the local half of data-parallel distributed training. Mirrors
    /// `train_step`'s weight binding/lazy-init exactly (so a client's
    /// gradients are computed against the same weight shapes/values a local
    /// `train_step` would use) but calls `PythonBridge::compute_gradients`
    /// instead, returning the gradients for the caller (the cluster host)
    /// to average across every client before anyone applies an update.
    pub fn compute_gradients_step(
        &self,
        mut batch_tensors: Vec<Tensor>,
        weights: &mut std::collections::HashMap<String, Tensor>,
        loss_name: &str,
        bridge: &PythonBridge,
    ) -> Result<(f32, std::collections::HashMap<String, Tensor>)> {
        let err = |msg: &str| crate::interop::protocol::BrainBuilderError::ConfigError(msg.to_string());
        let target = batch_tensors.pop().ok_or_else(|| err("training batch is empty"))?;
        let mut inputs = self.bind_data_ports(batch_tensors)?;
        self.bind_weights_lazy_init(&mut inputs, weights, bridge)?;

        let output_port = self
            .output_port()
            .ok_or_else(|| err("execution plan has no operations / output port"))?;

        bridge.compute_gradients(
            &self.operations,
            inputs,
            output_port,
            &target,
            loss_name,
            &self.trainable_weight_ports(),
        )
    }

    /// The cluster host's half of `compute_gradients_step`: applies one
    /// optimizer step using gradients already averaged across every joined
    /// client, restricted to `trainable_weight_ports()` — the same subset
    /// `train_step`/`compute_gradients_step` use, so a frozen parameter
    /// (e.g. LoRA's base weight) never gets optimized just because it was
    /// present in `weights`.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_averaged_gradients_step(
        &self,
        weights: &std::collections::HashMap<String, Tensor>,
        averaged_gradients: &std::collections::HashMap<String, Tensor>,
        optimizer_name: &str,
        lr: f64,
        weight_decay: f64,
        grad_clip: f64,
        bridge: &PythonBridge,
    ) -> Result<std::collections::HashMap<String, Tensor>> {
        let trainable = self.trainable_weight_ports();
        let trainable_weights: std::collections::HashMap<String, Tensor> = weights
            .iter()
            .filter(|(name, _)| trainable.contains(name))
            .map(|(name, t)| (name.clone(), t.clone()))
            .collect();
        bridge.apply_averaged_gradients(&trainable_weights, averaged_gradients, optimizer_name, lr, weight_decay, grad_clip)
    }

    /// Binds every learnable parameter port into `inputs`, lazily
    /// initializing (`torch.randn`/zero, shape from the descriptor or
    /// inferred from the bound data) any port `weights` doesn't already
    /// have a value for — shared by `train_step` and `compute_gradients_step`
    /// so both bind weights identically.
    fn bind_weights_lazy_init(
        &self,
        inputs: &mut std::collections::HashMap<String, Tensor>,
        weights: &mut std::collections::HashMap<String, Tensor>,
        bridge: &PythonBridge,
    ) -> Result<()> {
        let weight_ports = self.weight_ports();
        let init_shape = param_init_shape(inputs.values().next());
        for port in &weight_ports {
            let tensor = match weights.get(port) {
                Some(t) => t.clone(),
                None => {
                    // Elementwise components (`scale`) get a scalar `[1]`:
                    // PyTorch broadcasts it against a batch of any size, so
                    // the same trained weight works at inference time
                    // regardless of row count. Matmul-based components
                    // (`linear`) need a real `(out_features, in_features)`
                    // weight instead — a scalar can't broadcast through
                    // `torch.nn.functional.linear` — so when the bound data
                    // tensor is 2-D `(batch, features)`, size the weight as
                    // `(1, features)`. When the descriptor's declared shape
                    // was fully resolvable from this node's hyperparameters
                    // (e.g. `linear` with `out_features` set, or any of the
                    // newer components — `layernorm`, `embedding`, attention
                    // projections — which *require* explicit hyperparameters
                    // to be sized correctly), prefer that real shape instead.
                    let shape = self.resolved_shape_for(port).unwrap_or_else(|| init_shape.clone());
                    let t = if self.is_zero_init(port) {
                        bridge.zero_tensor(&shape)?
                    } else {
                        bridge.random_tensor(&shape)?
                    };
                    weights.insert(port.clone(), t.clone());
                    t
                }
            };
            inputs.insert(port.clone(), tensor);
        }
        Ok(())
    }
}

/// Shape to lazily-init a not-yet-learned parameter with, inferred from a
/// bound data tensor's rank: `[1]` (broadcastable scalar) for 1-D data, or
/// `[1, features]` for 2-D `(batch, features)` data (matmul-shaped, for
/// `linear`-style components).
fn param_init_shape(sample_data_tensor: Option<&Tensor>) -> Vec<i64> {
    let Some(tensor) = sample_data_tensor else {
        return vec![1];
    };
    unsafe {
        let t = &(*tensor.0).dl_tensor;
        if t.ndim == 2 {
            vec![1, *t.shape.offset(1)]
        } else {
            vec![1]
        }
    }
}

pub struct LossValue {
    pub step: usize,
    pub value: f32,
}

/// Topologically sorts `graph.nodes` by `graph.edges` (Kahn's algorithm) so
/// `compile()` builds `operations` in real dependency order rather than
/// assuming the GUI's node array happens to already be sorted (drag-and-drop
/// gives no such guarantee). Errors clearly on a cycle instead of silently
/// truncating or mis-ordering the graph.
fn topological_order<'a>(graph: &'a BBIRGraph) -> Result<Vec<&'a crate::bbir::BBIRNode>> {
    use std::collections::{HashMap, HashSet, VecDeque};

    let mut in_degree: HashMap<&str, usize> = graph.nodes.iter().map(|n| (n.id.as_str(), 0)).collect();
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &graph.edges {
        *in_degree.entry(edge.to_node.as_str()).or_insert(0) += 1;
        dependents.entry(edge.from_node.as_str()).or_default().push(edge.to_node.as_str());
    }

    // Seed the queue in declaration order (not hashmap order) so a graph with
    // no edges at all — the common case for today's single-node graphs —
    // keeps its original, predictable order.
    let mut queue: VecDeque<&str> = graph
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .filter(|id| in_degree.get(id).copied().unwrap_or(0) == 0)
        .collect();

    let node_by_id: HashMap<&str, &crate::bbir::BBIRNode> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut ordered = Vec::with_capacity(graph.nodes.len());
    while let Some(id) = queue.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        if let Some(&node) = node_by_id.get(id) {
            ordered.push(node);
        }
        if let Some(next) = dependents.get(id) {
            for &dep in next {
                let degree = in_degree.get_mut(dep).expect("dependent node must be in in_degree map");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if ordered.len() != graph.nodes.len() {
        return Err(crate::interop::protocol::BrainBuilderError::GraphError(
            "graph contains a cycle — cannot compile into a linear execution order".into(),
        ));
    }
    Ok(ordered)
}

pub fn compile(graph: &BBIRGraph, ctx: &AppContext) -> Result<ExecutionPlan> {
    let registry = ctx.registry.read().map_err(|_| {
        crate::interop::protocol::BrainBuilderError::ConfigError("registry lock poisoned".into())
    })?;

    // Real edge-driven wiring: a connected input resolves to its producer's
    // namespaced output key; an unconnected one (leaf data, or a learnable
    // parameter) gets a key scoped to *this* node. Every previous version of
    // this function used the bare descriptor port name (e.g. "weight")
    // directly as the cross-op binding key — harmless for the single-node
    // graphs every existing test happened to use, but silently wrong the
    // moment two nodes share a declared port name (two `linear` layers, or
    // an `embedding` node feeding a `linear` projection — both declare
    // "weight"): the second node's value would overwrite the first's in the
    // flat intermediate map, and `graph.edges` was never even consulted to
    // notice the two nodes were supposed to be wired together at all.
    let edge_source: std::collections::HashMap<(&str, &str), (&str, &str)> = graph
        .edges
        .iter()
        .map(|e| ((e.to_node.as_str(), e.to_port.as_str()), (e.from_node.as_str(), e.from_port.as_str())))
        .collect();
    let namespaced = |node_id: &str, port: &str| format!("{node_id}:{port}");

    let ordered_nodes = topological_order(graph)?;
    let ops: Vec<ExecutableOp> = ordered_nodes
        .into_iter()
        .map(|n| {
            // Prefer the descriptor's real implementation (language + entry)
            // and its parameter-port classification; fall back to a plain
            // python `forward` if the component isn't in the registry (keeps
            // hand-built/test graphs working).
            let desc = registry.get_by_name(&n.component);
            let (language, entry) = desc
                .and_then(|d| d.implementations.first())
                .map(|impl_| (impl_.language.clone(), impl_.entry.clone()))
                .unwrap_or_else(|| ("python".into(), "forward".into()));

            // Bare descriptor-declared port names, in descriptor order —
            // needed to call the Python function positionally in the right
            // order, and to classify parameter/trainable/zero-init ports.
            let bare_param_inputs = desc.map(|d| d.parameter_ports()).unwrap_or_default();
            let bare_trainable = desc.map(|d| d.trainable_parameter_ports()).unwrap_or_else(|| bare_param_inputs.clone());
            let bare_zero_init = desc.map(|d| d.zero_init_parameter_ports()).unwrap_or_default();

            // Resolve each of *this node's own* declared input ports (not
            // `n.ports.input_ports`, which is GUI-authored and only needs to
            // match in count/order — the descriptor is the source of truth
            // for names) to its real binding key.
            let resolved_inputs: Vec<String> = if let Some(d) = desc {
                d.inputs
                    .iter()
                    .map(|p| match edge_source.get(&(n.id.as_str(), p.name.as_str())) {
                        Some((from_node, from_port)) => namespaced(from_node, from_port),
                        None => namespaced(&n.id, &p.name),
                    })
                    .collect()
            } else {
                // No descriptor available (hand-built/fixture component name
                // not in the registry): preserve the old bare-name behavior
                // rather than guessing — these are only ever used by tests
                // that bypass `compile()`'s descriptor path anyway.
                n.ports.input_ports.clone()
            };
            let resolved_outputs: Vec<String> = if let Some(d) = desc {
                d.outputs.iter().map(|p| namespaced(&n.id, &p.name)).collect()
            } else {
                n.ports.output_ports.iter().map(|p| namespaced(&n.id, p)).collect()
            };
            let param_inputs: Vec<String> = bare_param_inputs.iter().map(|p| namespaced(&n.id, p)).collect();
            let trainable_inputs: Vec<String> = bare_trainable.iter().map(|p| namespaced(&n.id, p)).collect();
            let zero_init_inputs: Vec<String> = bare_zero_init.iter().map(|p| namespaced(&n.id, p)).collect();
            let resolved_param_shapes = bare_param_inputs
                .iter()
                .filter_map(|port| {
                    let shape = &desc?.inputs.iter().find(|p| &p.name == port)?.tensor.shape;
                    let dims = resolve_shape_expr(shape, &n.hyperparams)?;
                    Some((namespaced(&n.id, port), dims))
                })
                .collect();

            ExecutableOp {
                node_id: n.id.clone(),
                component: n.component.clone(),
                language,
                entry,
                inputs: resolved_inputs,
                outputs: resolved_outputs,
                param_inputs,
                trainable_inputs,
                zero_init_inputs,
                hyperparams: n.hyperparams.clone(),
                resolved_param_shapes,
            }
        })
        .collect();

    let epochs = graph
        .training
        .as_ref()
        .and_then(|t| t.hyperparams.get("epochs"))
        .and_then(|v| v.as_u64())
        .map(|e| e as usize)
        .unwrap_or(10);

    let preset_weights = load_preset_weights(graph, &ops, ctx.arena.as_ref())?;

    Ok(ExecutionPlan {
        epochs,
        graph: graph.clone(),
        operations: ops,
        device_assignments: std::collections::HashMap::new(),
        preset_weights,
        arena: ctx.arena.clone(),
    })
}

/// Load any pretrained weights a node asked for into the plan's
/// `preset_weights`, keyed by the namespaced parameter port the trainer binds.
/// A node opts in with a `pretrained` hyperparameter:
///
/// ```json
/// "pretrained": { "file": "model.safetensors", "tensor": "encoder.weight", "port": "weight" }
/// ```
///
/// `port` defaults to `"weight"`. This is the transfer-learning seam: pair it
/// with a `lora_linear` node (whose base `weight` is non-trainable) and the
/// loaded tensor becomes a real frozen pretrained layer, adapted by the LoRA
/// parameters the optimizer *does* train. The loaded tensor's shape is checked
/// against the port's descriptor-resolved shape (when known) so a mismatched
/// pretrained tensor fails loudly at compile time, not deep inside the worker.
///
/// A node can also spell this out as three flat hyperparameters instead —
/// `pretrained_file`/`pretrained_tensor`/(optional) `pretrained_port` — which
/// is the form `lora_linear.edn` declares, so the Inspector's generic
/// `SchemaForm` (which only knows how to render flat scalar hyperparameters,
/// never a nested object) can set them directly on a hand-dragged node. The
/// nested `pretrained` object stays the primary format — it's what the Intent
/// transfer-learning flow (`intent::propose_transfer_model`) authors — and
/// takes priority if both happen to be present.
fn load_preset_weights(
    graph: &BBIRGraph,
    ops: &[ExecutableOp],
    arena: &SharedArena,
) -> Result<std::collections::HashMap<String, Tensor>> {
    let err = |msg: String| crate::interop::protocol::BrainBuilderError::ConfigError(msg);
    let mut preset = std::collections::HashMap::new();

    for node in &graph.nodes {
        let (file, tensor_name, port): (String, String, String) =
            if let Some(pretrained) = node.hyperparams.get("pretrained") {
                // Ignore a `null`/absent value gracefully (a GUI may
                // serialize the key with no value); only act on a real
                // object.
                let Some(spec) = pretrained.as_object() else {
                    continue;
                };
                let file = spec.get("file").and_then(|v| v.as_str()).ok_or_else(|| {
                    err(format!("node `{}` has a `pretrained` block without a string `file`", node.id))
                })?;
                let tensor_name = spec.get("tensor").and_then(|v| v.as_str()).ok_or_else(|| {
                    err(format!("node `{}` has a `pretrained` block without a string `tensor`", node.id))
                })?;
                let port = spec.get("port").and_then(|v| v.as_str()).unwrap_or("weight");
                (file.to_string(), tensor_name.to_string(), port.to_string())
            } else if let (Some(file), Some(tensor_name)) = (
                node.hyperparams.get("pretrained_file").and_then(|v| v.as_str()).filter(|s| !s.is_empty()),
                node.hyperparams.get("pretrained_tensor").and_then(|v| v.as_str()).filter(|s| !s.is_empty()),
            ) {
                let port = node.hyperparams.get("pretrained_port").and_then(|v| v.as_str()).unwrap_or("weight");
                (file.to_string(), tensor_name.to_string(), port.to_string())
            } else {
                continue;
            };

        let loaded = crate::models::safetensors_loader::load_named_tensors(
            std::path::Path::new(&file),
            &[tensor_name.as_str()],
            arena,
        )?;
        let loaded_tensor = loaded
            .get(tensor_name.as_str())
            .ok_or_else(|| err(format!("pretrained tensor `{tensor_name}` not found in `{file}`")))?;

        let key = format!("{}:{}", node.id, port);

        // If the descriptor + hyperparameters pinned an exact shape for this
        // port, the pretrained tensor must match it.
        if let Some(op) = ops.iter().find(|o| o.node_id == node.id) {
            if let Some(expected) = op.resolved_param_shapes.get(&key) {
                if expected != &loaded_tensor.shape {
                    return Err(err(format!(
                        "pretrained tensor `{tensor_name}` has shape {:?}, but node `{}` port `{port}` expects {:?}",
                        loaded_tensor.shape, node.id, expected
                    )));
                }
            }
        }

        preset.insert(key, loaded_tensor.tensor.clone());
    }

    Ok(preset)
}
