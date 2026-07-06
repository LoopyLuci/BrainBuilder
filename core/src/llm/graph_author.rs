// Turns a plain-English model description into a real BBIR graph via a
// local Ollama model, constrained to the live component registry so the
// model can only ever propose components/ports that genuinely exist —
// and validated through the exact same `validate_graph` the GUI's own
// "Validate" button and `execute_graph` use, so a hallucinated port or
// shape mismatch is caught here, not silently handed to the canvas.
//
// This can't be live-verified end-to-end in this environment: the sandbox
// this was built in cannot bind Ollama's local port (a real OS-level socket
// permission restriction specific to this tool sandbox — a normal user
// running the real desktop app won't hit this). What *is* verified: the
// HTTP client's request/response shapes match Ollama's real documented API
// (`ollama.rs`'s tests), the prompt-building logic (tested here against the
// real component registry), and the parse-then-validate pipeline (tested
// against hand-written "model output" JSON standing in for a real
// response) — the only unverified link is the network hop itself.
use crate::bbir::BBIRGraph;
use crate::component::descriptor::ComponentSummary;
use crate::component::registry::ComponentRegistry;
use crate::component::validation::validate_graph;
use crate::interop::protocol::BrainBuilderError;
use crate::llm::ollama::OllamaClient;
use crate::Result;

pub async fn generate_graph_from_description(
    description: &str,
    model: &str,
    registry: &ComponentRegistry,
) -> Result<BBIRGraph> {
    let system = build_system_prompt(&registry.summaries());
    let client = OllamaClient::new();
    let raw = client.generate_json(model, &system, description).await?;
    parse_and_validate(&raw, registry)
}

/// Split out from `generate_graph_from_description` so the parse/validate
/// half — the part that doesn't need a live network call — is directly
/// testable against hand-written stand-in model output, and so callers
/// holding a non-`Send` lock guard on the registry (e.g. a Tauri command
/// backed by `std::sync::RwLock`) can build the prompt and validate the
/// result *outside* the network `.await`, only ever touching the registry
/// synchronously (see `gui/src-tauri/src/main.rs`'s `generate_graph`
/// command for exactly that pattern).
pub fn parse_and_validate(raw_model_output: &str, registry: &ComponentRegistry) -> Result<BBIRGraph> {
    let graph: BBIRGraph = serde_json::from_str(raw_model_output).map_err(|e| {
        BrainBuilderError::Parse(format!(
            "the model's output wasn't a valid BBIR graph ({e}) — raw output:\n{raw_model_output}"
        ))
    })?;
    validate_graph(&graph, registry)?;
    Ok(graph)
}

/// Builds the system prompt from the *live* component registry (not a
/// hardcoded list that could drift from whatever's actually installed), so
/// the model is only ever offered components/ports that genuinely exist —
/// the single biggest lever against hallucinated graphs.
pub fn build_system_prompt(components: &[ComponentSummary]) -> String {
    let component_list: String = components
        .iter()
        .map(|c| {
            let inputs: Vec<String> = c.inputs.iter().map(|p| format!("{}[{}:{}]", p.name, p.role, p.dtype)).collect();
            let outputs: Vec<String> = c.outputs.iter().map(|p| format!("{}[{}]", p.name, p.dtype)).collect();
            let hp: Vec<String> = c
                .hyperparameters
                .iter()
                .map(|h| format!("{}:{}(default {})", h.name, h.param_type, h.default))
                .collect();
            format!(
                "- `{}` ({}): inputs=[{}] outputs=[{}] hyperparameters=[{}]",
                c.name,
                c.meta_type,
                inputs.join(", "),
                outputs.join(", "),
                hp.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "You are a graph-authoring assistant for BrainBuilder, a visual neural-network builder. \
Given a plain-English description of a model, output ONLY a single JSON object matching this exact \
schema (BBIR — BrainBuilder Intermediate Representation). No explanation, no markdown fences — just \
the JSON object.\n\n\
Schema:\n\
{{\"graph_id\": \"<short unique id>\", \"name\": \"<short name>\", \"nodes\": [{{\"id\": \"<unique node id>\", \
\"component\": \"<a component name from the list below>\", \"label\": null, \"hyperparams\": {{...}}, \
\"ports\": {{\"input_ports\": [...], \"output_ports\": [...]}}, \"position\": null}}], \
\"edges\": [{{\"from_node\": \"<id>\", \"from_port\": \"<name>\", \"to_node\": \"<id>\", \"to_port\": \"<name>\"}}], \
\"training\": null}}\n\n\
Rules:\n\
- Only use component names from this exact list, with their real declared port names (a port marked \
`parameter` is auto-initialized by the trainer — never wire an edge to it, never treat it as a dataset input):\n\
{component_list}\n\
- `ports.input_ports`/`ports.output_ports` on a node must list that component's real port names, in order.\n\
- Connect nodes with real `edges` entries — never rely on port names coincidentally matching.\n\
- Every node id must be unique.\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_registry() -> ComponentRegistry {
        let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        let mut registry = ComponentRegistry::new();
        registry.load_from_dir(&components_dir).expect("failed to load real shipped components");
        registry
    }

    #[test]
    fn system_prompt_lists_real_registered_components() {
        let registry = test_registry();
        let prompt = build_system_prompt(&registry.summaries());
        assert!(prompt.contains("`linear`"), "prompt should mention the real `linear` component");
        assert!(prompt.contains("`attention`"), "prompt should mention the real `attention` component");
    }

    #[test]
    fn accepts_well_formed_model_output_and_returns_a_valid_graph() {
        let registry = test_registry();
        let model_output = r#"{
            "graph_id": "g1", "name": "test-graph",
            "nodes": [
                {"id": "n1", "component": "linear", "label": null,
                 "hyperparams": {"in_features": 4, "out_features": 1},
                 "ports": {"input_ports": ["input", "weight"], "output_ports": ["output"]},
                 "position": null}
            ],
            "edges": [], "training": null
        }"#;
        let graph = parse_and_validate(model_output, &registry).expect("should accept a valid graph");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].component, "linear");
    }

    #[test]
    fn rejects_a_hallucinated_component_name() {
        let registry = test_registry();
        let model_output = r#"{
            "graph_id": "g1", "name": "test-graph",
            "nodes": [
                {"id": "n1", "component": "quantum_flux_capacitor", "label": null,
                 "hyperparams": {}, "ports": {"input_ports": ["x"], "output_ports": ["y"]}, "position": null}
            ],
            "edges": [], "training": null
        }"#;
        let err = parse_and_validate(model_output, &registry).unwrap_err();
        assert!(err.to_string().contains("quantum_flux_capacitor"), "error should name the bad component: {err}");
    }

    #[test]
    fn rejects_malformed_json_with_a_clear_error() {
        let registry = test_registry();
        let err = parse_and_validate("not json at all", &registry).unwrap_err();
        assert!(err.to_string().contains("valid BBIR graph"));
    }
}
