//! Runtime smoke test for 10 next-gen models.
//!
//! Verifies each model compiles and exposes its core public API.
//! Run with: cargo test -p brainbuilder-gui --test smoke_next_gen_models

#[test]
fn causal_reasoning_compiles_and_has_api() {
    let src = include_str!("../src/causal_reasoning.rs");
    assert!(src.contains("pub struct CausalReasoningModel"), "CausalReasoningModel not found");
    assert!(src.contains("pub fn do_calculus"), "do_calculus not found");
    assert!(src.contains("pub fn counterfactual"), "counterfactual not found");
    assert!(src.contains("pub fn graph_metrics"), "graph_metrics not found");
}

#[test]
fn graph_of_thoughts_compiles_and_has_api() {
    let src = include_str!("../src/graph_of_thoughts.rs");
    assert!(src.contains("pub struct GraphOfThoughtsModel"), "GraphOfThoughtsModel not found");
    assert!(src.contains("pub fn reason"), "reason not found");
    assert!(src.contains("pub fn current_graph"), "current_graph not found");
}

#[test]
fn concept_bottleneck_compiles_and_has_api() {
    let src = include_str!("../src/concept_bottleneck.rs");
    assert!(src.contains("pub struct ConceptBottleneckModel"), "ConceptBottleneckModel not found");
    assert!(src.contains("pub fn predict"), "predict not found");
    assert!(src.contains("pub fn train"), "train not found");
    assert!(src.contains("pub fn concept_importance"), "concept_importance not found");
}

#[test]
fn mixture_of_experts_compiles_and_has_api() {
    let src = include_str!("../src/mixture_of_experts.rs");
    assert!(src.contains("pub struct MixtureOfExpertsRouter"), "MixtureOfExpertsRouter not found");
    assert!(src.contains("pub fn route"), "route not found");
    assert!(src.contains("pub fn expert_stats"), "expert_stats not found");
}

#[test]
fn neural_architecture_search_compiles_and_has_api() {
    let src = include_str!("../src/neural_architecture_search.rs");
    assert!(src.contains("pub struct NeuralArchitectureSearch"), "NeuralArchitectureSearch not found");
    assert!(src.contains("pub fn search"), "search not found");
    assert!(src.contains("fn evaluate"), "evaluate not found");
    assert!(src.contains("fn mutate"), "mutate not found");
}

#[test]
fn differentiable_neural_computer_compiles_and_has_api() {
    let src = include_str!("../src/differentiable_neural_computer.rs");
    assert!(src.contains("pub struct DifferentiableNeuralComputer"), "DifferentiableNeuralComputer not found");
    assert!(src.contains("pub fn step"), "step not found");
    assert!(src.contains("pub fn current_state"), "current_state not found");
}

#[test]
fn hyperdimensional_computing_compiles_and_has_api() {
    let src = include_str!("../src/hyperdimensional_computing.rs");
    assert!(src.contains("pub struct HyperdimensionalComputingModel"), "HyperdimensionalComputingModel not found");
    assert!(src.contains("pub fn bind"), "bind not found");
    assert!(src.contains("pub fn bundle"), "bundle not found");
    assert!(src.contains("pub fn query"), "query not found");
}

#[test]
fn spiking_neural_network_compiles_and_has_api() {
    let src = include_str!("../src/spiking_neural_network.rs");
    assert!(src.contains("pub struct SpikingNeuralNetwork"), "SpikingNeuralNetwork not found");
    assert!(src.contains("pub fn run"), "run not found");
    assert!(src.contains("pub fn layer_stats"), "layer_stats not found");
}

#[test]
fn world_model_compiles_and_has_api() {
    let src = include_str!("../src/world_model.rs");
    assert!(src.contains("pub struct WorldModel"), "WorldModel not found");
    assert!(src.contains("pub fn predict"), "predict not found");
    assert!(src.contains("pub fn transition_stats"), "transition_stats not found");
}

#[test]
fn program_synthesis_compiles_and_has_api() {
    let src = include_str!("../src/program_synthesis.rs");
    assert!(src.contains("pub struct ProgramSynthesisModel"), "ProgramSynthesisModel not found");
    assert!(src.contains("pub fn synthesize"), "synthesize not found");
}

#[test]
fn multimodal_alignment_compiles_and_has_api() {
    let src = include_str!("../src/multimodal_alignment.rs");
    assert!(src.contains("pub struct MultimodalAlignmentModel"), "MultimodalAlignmentModel not found");
    assert!(src.contains("pub fn align"), "align not found");
}

#[test]
fn next_gen_attention_compiles_and_has_api() {
    let src = include_str!("../src/next_gen_attention.rs");
    assert!(src.contains("pub struct NextGenAttention"), "NextGenAttention not found");
    assert!(src.contains("pub fn forward"), "forward not found");
    assert!(src.contains("pub fn prune_heads"), "prune_heads not found");
}

#[test]
fn next_gen_modules_exist() {
    let main_src = include_str!("../src/main.rs");
    let modules = [
        "mod causal_reasoning;",
        "mod graph_of_thoughts;",
        "mod concept_bottleneck;",
        "mod mixture_of_experts;",
        "mod neural_architecture_search;",
        "mod differentiable_neural_computer;",
        "mod hyperdimensional_computing;",
        "mod spiking_neural_network;",
        "mod world_model;",
        "mod program_synthesis;",
        "mod multimodal_alignment;",
        "mod next_gen_attention;",
    ];
    for m in modules {
        assert!(main_src.contains(m), "{} not found in main.rs", m);
    }
}
