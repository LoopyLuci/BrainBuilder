use crate::bbir::{BBIRGraph, BBIRNode};
use crate::component::registry::ComponentRegistry;
use crate::component::descriptor::{ComponentDescriptor, ShapeExpr};
use crate::runtime::scheduler::resolve_shape_dims_partial;
use crate::Result;
use crate::interop::protocol::BrainBuilderError;

/// Self-consistency checks for a single component descriptor, independent of
/// any graph — the structural gate a *synthesized* component must clear before
/// its kernel is ever run (see `crate::synthesis`). Confirms it declares a
/// name, at least one output, and unique, non-empty port names. Deliberately
/// conservative: it rejects obviously-malformed descriptors without second-
/// guessing legitimate exotic shapes.
pub fn validate_descriptor_self(descriptor: &ComponentDescriptor) -> Result<()> {
    if descriptor.name.trim().is_empty() {
        return Err(BrainBuilderError::ConfigError("component descriptor has no name".into()));
    }
    if descriptor.outputs.is_empty() {
        return Err(BrainBuilderError::ConfigError(format!(
            "component `{}` declares no outputs",
            descriptor.name
        )));
    }
    for (side, ports) in [("input", &descriptor.inputs), ("output", &descriptor.outputs)] {
        let mut seen = std::collections::HashSet::new();
        for port in ports.iter() {
            if port.name.trim().is_empty() {
                return Err(BrainBuilderError::ConfigError(format!(
                    "component `{}` has an {side} port with an empty name",
                    descriptor.name
                )));
            }
            if !seen.insert(port.name.as_str()) {
                return Err(BrainBuilderError::ConfigError(format!(
                    "component `{}` has a duplicate {side} port name `{}`",
                    descriptor.name, port.name
                )));
            }
        }
    }
    Ok(())
}

/// Validate shape and type compatibility across every edge in the graph.
pub fn validate_graph(graph: &BBIRGraph, registry: &ComponentRegistry) -> Result<()> {
    let node_map: std::collections::HashMap<&str, &BBIRNode> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Real, independent check that every node's component exists — the
    // edge-driven loop below only ever looks up a node's component when
    // that node is touched by an edge, so a hallucinated/misspelled
    // component name on an unconnected node (or a single-node graph with no
    // edges at all — every graph before today's multi-node work) silently
    // passed validation entirely. Caught by testing LLM-authored graphs
    // against this function, not by inspection.
    for node in &graph.nodes {
        registry
            .get_by_name(&node.component)
            .ok_or_else(|| BrainBuilderError::ComponentNotFound(node.component.clone()))?;
    }

    for edge in &graph.edges {
        let src_node = node_map
            .get(edge.from_node.as_str())
            .ok_or_else(|| BrainBuilderError::GraphError(format!("missing node {}", edge.from_node)))?;
        let tgt_node = node_map
            .get(edge.to_node.as_str())
            .ok_or_else(|| BrainBuilderError::GraphError(format!("missing node {}", edge.to_node)))?;

        let src_comp = registry
            .get_by_name(&src_node.component)
            .ok_or_else(|| BrainBuilderError::ComponentNotFound(src_node.component.clone()))?;
        let tgt_comp = registry
            .get_by_name(&tgt_node.component)
            .ok_or_else(|| BrainBuilderError::ComponentNotFound(tgt_node.component.clone()))?;

        let src_port = src_comp
            .outputs
            .iter()
            .find(|p| p.name == edge.from_port)
            .ok_or_else(|| BrainBuilderError::GraphError(format!("port {} not on {}", edge.from_port, src_node.component)))?;
        let tgt_port = tgt_comp
            .inputs
            .iter()
            .find(|p| p.name == edge.to_port)
            .ok_or_else(|| BrainBuilderError::GraphError(format!("port {} not on {}", edge.to_port, tgt_node.component)))?;

        if src_port.tensor.dtype != tgt_port.tensor.dtype {
            return Err(BrainBuilderError::TypeMismatch(format!(
                "{} -> {}: dtype {:?} vs {:?}",
                src_node.component, tgt_node.component, src_port.tensor.dtype, tgt_port.tensor.dtype
            )));
        }

        // Try real numeric resolution first, per dimension, against each
        // side's *own node* hyperparameters (the same resolver
        // `scheduler::compile` uses to size weight tensors before
        // training). This catches concrete mismatches a purely structural
        // check can't — e.g. `linear(out_features=64)` feeding
        // `layernorm(dims=32)` — even though the shapes both start with an
        // unresolvable `:batch` dim that can only be known at runtime:
        // resolution is independent per-dimension, so `:batch` failing to
        // resolve doesn't block checking the dimensions that do.
        let src_dims = resolve_shape_dims_partial(&src_port.tensor.shape, &src_node.hyperparams);
        let tgt_dims = resolve_shape_dims_partial(&tgt_port.tensor.shape, &tgt_node.hyperparams);
        if let (Some(src_dims), Some(tgt_dims)) = (&src_dims, &tgt_dims) {
            if src_dims.len() == tgt_dims.len() {
                for (i, (a, b)) in src_dims.iter().zip(tgt_dims.iter()).enumerate() {
                    if let (Some(a), Some(b)) = (a, b) {
                        if a != b {
                            return Err(BrainBuilderError::ShapeInference(format!(
                                "{} -> {}: dim {i} mismatch — {} produces {a} but {} expects {b} (resolved from hyperparameters)",
                                src_node.component, tgt_node.component, src_node.component, tgt_node.component
                            )));
                        }
                    }
                }
            }
        }

        match (&src_port.tensor.shape, &tgt_port.tensor.shape) {
            (ShapeExpr::Fixed(a), ShapeExpr::Fixed(b)) if a == b => {}
            (ShapeExpr::Fixed(_), ShapeExpr::Fixed(_)) => {
                return Err(BrainBuilderError::ShapeInference(format!(
                    "{} -> {}: fixed shape mismatch", src_node.component, tgt_node.component
                )));
            }
            (ShapeExpr::Symbolic(a), ShapeExpr::Symbolic(b)) => {
                check_symbolic_shapes(a, b).map_err(|msg| {
                    BrainBuilderError::ShapeInference(format!(
                        "{} -> {}: {msg}",
                        src_node.component, tgt_node.component
                    ))
                })?;
            }
            // One side fixed, one symbolic (e.g. `[1 16 30 30]` vs
            // `[:batch :out-ch (:- :h :kh 1)]`) and neither side resolved
            // numerically above: real unification would need concrete
            // runtime values for whatever didn't resolve. Left permissive
            // rather than fake-checking it.
            _ => {}
        }
    }
    Ok(())
}

/// Structural unification for two symbolic shape arrays: same rank, and
/// every dimension must be `dims_compatible`. Real *numeric* evaluation of
/// an expression like `(:- :h :kh 1)` needs concrete values for `:h`/`:kh`,
/// which only exist once real data flows through the graph at runtime — not
/// at this static, pre-execution validation pass. What *is* soundly checkable
/// without runtime data is the expression's structure: two dimensions built
/// from different operators, different arity, or different nested literals
/// can never evaluate to the same value regardless of what the named symbols
/// bind to, so those are real, provable mismatches worth catching now.
fn check_symbolic_shapes(a: &serde_json::Value, b: &serde_json::Value) -> std::result::Result<(), String> {
    let (a, b) = match (a.as_array(), b.as_array()) {
        (Some(a), Some(b)) => (a, b),
        _ => return Ok(()), // not both arrays; nothing structural to check
    };
    if a.len() != b.len() {
        return Err(format!(
            "symbolic shape rank mismatch: {} dims vs {} dims",
            a.len(),
            b.len()
        ));
    }
    for (i, (da, db)) in a.iter().zip(b.iter()).enumerate() {
        if !dims_compatible(da, db) {
            return Err(format!("dim {i}: `{da}` is structurally incompatible with `{db}`"));
        }
    }
    Ok(())
}

/// Recursive structural compatibility check for one shape dimension —
/// concrete numbers must match exactly; an `(:op ...)` expression must match
/// another expression's operator, arity, and (recursively) every operand;
/// anything involving a bare named symbol (`:batch`) is permissively
/// accepted, since there's no cross-component dimension-name-aliasing
/// convention in this codebase to prove two differently-named symbols refer
/// to the same axis (or don't).
fn dims_compatible(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(x), serde_json::Value::Number(y)) => x.as_i64() == y.as_i64(),
        (serde_json::Value::Array(xs), serde_json::Value::Array(ys)) => {
            xs.len() == ys.len()
                && xs.first().zip(ys.first()).is_none_or(|(op_a, op_b)| op_a == op_b)
                && xs.iter().zip(ys.iter()).all(|(x, y)| dims_compatible(x, y))
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{check_symbolic_shapes, validate_graph};
    use crate::bbir::{BBIRGraph, BBIRNode};
    use crate::component::registry::ComponentRegistry;
    use serde_json::json;

    #[test]
    fn accepts_matching_named_dims() {
        assert!(check_symbolic_shapes(&json!([":batch", ":features"]), &json!([":batch", ":features"])).is_ok());
    }

    #[test]
    fn rejects_rank_mismatch() {
        assert!(check_symbolic_shapes(&json!([":batch"]), &json!([":batch", ":features"])).is_err());
    }

    #[test]
    fn rejects_conflicting_concrete_dims() {
        assert!(check_symbolic_shapes(&json!([":batch", 16]), &json!([":batch", 32])).is_err());
    }

    #[test]
    fn accepts_concrete_dim_matching_across_expressions() {
        assert!(check_symbolic_shapes(&json!([":batch", 16]), &json!([":batch", 16])).is_ok());
    }

    #[test]
    fn accepts_matching_expressions_regardless_of_symbol_names() {
        // `(:- :h :kh 1)` vs `(:- :height :kernel-h 1)`: same operator/arity/
        // literal, different symbol names — can't prove incompatible.
        assert!(check_symbolic_shapes(
            &json!([":batch", ":out-ch", [":-", ":h", ":kh", 1]]),
            &json!([":batch", ":out-ch", [":-", ":height", ":kernel-h", 1]]),
        )
        .is_ok());
    }

    #[test]
    fn rejects_expressions_with_different_operators() {
        assert!(check_symbolic_shapes(
            &json!([[":-", ":h", ":kh", 1]]),
            &json!([[":*", ":h", ":kh", 1]]),
        )
        .is_err());
    }

    #[test]
    fn rejects_expressions_with_different_literal_operands() {
        assert!(check_symbolic_shapes(
            &json!([[":-", ":h", ":kh", 1]]),
            &json!([[":-", ":h", ":kh", 2]]),
        )
        .is_err());
    }

    #[test]
    fn accepts_bare_symbol_against_an_expression() {
        // Can't prove these incompatible without runtime values.
        assert!(check_symbolic_shapes(&json!([":some-dim"]), &json!([[":-", ":h", ":kh", 1]])).is_ok());
    }

    fn real_registry() -> ComponentRegistry {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        let mut registry = ComponentRegistry::new();
        registry.load_from_dir(&dir).expect("real components directory must load");
        registry
    }

    fn node(id: &str, component: &str, hyperparams: serde_json::Value) -> BBIRNode {
        BBIRNode {
            id: id.to_string(),
            component: component.to_string(),
            label: None,
            hyperparams,
            ports: crate::bbir::PortInfo { input_ports: vec![], output_ports: vec![] },
            position: None,
        }
    }

    fn linear_to_layernorm_graph(out_features: i64, features: i64) -> BBIRGraph {
        BBIRGraph {
            schema_version: 1,
            graph_id: "g".to_string(),
            name: "linear-to-layernorm".to_string(),
            nodes: vec![
                node("n1", "linear", json!({"in_features": 16, "out_features": out_features})),
                node("n2", "layernorm", json!({"features": features, "eps": 1e-5})),
            ],
            edges: vec![crate::bbir::BBIREdge {
                from_node: "n1".to_string(),
                from_port: "output".to_string(),
                to_node: "n2".to_string(),
                to_port: "input".to_string(),
            }],
            training: None,
        }
    }

    /// The real regression case symbolic-only structural checking cannot
    /// catch: `linear`'s `:out-features` and `layernorm`'s `:features` are
    /// different symbol names, so `check_symbolic_shapes` alone permissively
    /// accepts them regardless of value. Resolving each side against its
    /// own node's real hyperparameters (`resolve_shape_dims_partial`) is
    /// what actually proves 64 != 32 here, before ever spawning a training
    /// run that would fail deep inside the Python worker instead.
    #[test]
    fn catches_a_real_hyperparameter_driven_shape_mismatch_across_differently_named_dims() {
        let registry = real_registry();
        let graph = linear_to_layernorm_graph(64, 32);
        let err = validate_graph(&graph, &registry).unwrap_err().to_string();
        assert!(err.contains("mismatch"), "expected a shape mismatch error, got: {err}");
        assert!(err.contains("64") && err.contains("32"), "error should name the conflicting values: {err}");
    }

    #[test]
    fn accepts_matching_hyperparameter_driven_dims_across_differently_named_symbols() {
        let registry = real_registry();
        let graph = linear_to_layernorm_graph(64, 64);
        assert!(validate_graph(&graph, &registry).is_ok());
    }
}
