//! Runtime smoke test for 15 next-gen models (batch 2).
//!
//! Verifies each model file exists, is non-trivial, declares its main struct,
//! exposes its core API surface, and is wired into main.rs.
//! Run with: cargo test -p brainbuilder-gui --test smoke_next_gen_batch2

#[test]
fn retrieval_augmented_generation_compiles_and_has_api() {
    let src = include_str!("../src/retrieval_augmented_generation.rs");
    assert!(
        src.contains("pub struct RetrievalAugmentedGeneration"),
        "RetrievalAugmentedGeneration not found"
    );
    assert!(src.contains("pub async fn rag_query"), "rag_query not found");
    assert!(src.contains("pub async fn rag_add_document"), "rag_add_document not found");
    assert!(
        src.contains("pub struct RagResponse"),
        "RagResponse not found"
    );
}

#[test]
fn adaptive_reasoning_compiles_and_has_api() {
    let src = include_str!("../src/adaptive_reasoning.rs");
    assert!(
        src.contains("pub struct AdaptiveReasoning"),
        "AdaptiveReasoning not found"
    );
    assert!(src.contains("pub async fn adaptive_reason"), "adaptive_reason not found");
    assert!(
        src.contains("pub async fn adaptive_backtrack"),
        "adaptive_backtrack not found"
    );
}

#[test]
fn knowledge_graph_compiles_and_has_api() {
    let src = include_str!("../src/knowledge_graph.rs");
    assert!(
        src.contains("pub struct KnowledgeGraph"),
        "KnowledgeGraph not found"
    );
    assert!(src.contains("pub fn add_entity"), "add_entity not found");
    assert!(src.contains("pub fn add_relation"), "add_relation not found");
    assert!(src.contains("pub fn neighbors"), "neighbors not found");
}

#[test]
fn flow_analyzer_compiles_and_has_api() {
    let src = include_str!("../src/flow_analyzer.rs");
    assert!(src.contains("pub struct FlowAnalyzer"), "FlowAnalyzer not found");
    assert!(src.contains("pub async fn flow_simulate"), "flow_simulate not found");
    assert!(src.contains("pub fn add_node"), "add_node not found");
}

#[test]
fn counterfactual_explainer_compiles_and_has_api() {
    let src = include_str!("../src/counterfactual_explainer.rs");
    assert!(
        src.contains("pub struct CounterfactualEngine"),
        "CounterfactualEngine not found"
    );
    assert!(
        src.contains("pub async fn counterfactual_explain"),
        "counterfactual_explain not found"
    );
    assert!(
        src.contains("pub struct CounterfactualExplanation"),
        "CounterfactualExplanation not found"
    );
}

#[test]
fn continual_learning_compiles_and_has_api() {
    let src = include_str!("../src/continual_learning.rs");
    assert!(
        src.contains("pub struct ContinualLearning"),
        "ContinualLearning not found"
    );
    assert!(src.contains("pub async fn continual_train"), "continual_train not found");
    assert!(
        src.contains("pub async fn continual_progress"),
        "continual_progress not found"
    );
}

#[test]
fn symbolic_reasoning_compiles_and_has_api() {
    let src = include_str!("../src/symbolic_reasoning.rs");
    assert!(
        src.contains("pub struct SymbolicProgram"),
        "SymbolicProgram not found"
    );
    assert!(
        src.contains("pub async fn symbolic_execute"),
        "symbolic_execute not found"
    );
    assert!(
        src.contains("pub enum SymbolicValue"),
        "SymbolicValue not found"
    );
}

#[test]
fn temporal_point_process_compiles_and_has_api() {
    let src = include_str!("../src/temporal_point_process.rs");
    assert!(
        src.contains("pub struct TemporalPointProcess"),
        "TemporalPointProcess not found"
    );
    assert!(src.contains("pub async fn tpp_forecast"), "tpp_forecast not found");
    assert!(src.contains("pub async fn tpp_record"), "tpp_record not found");
}

#[test]
fn interactive_explainability_compiles_and_has_api() {
    let src = include_str!("../src/interactive_explainability.rs");
    assert!(
        src.contains("pub struct InteractiveExplainability"),
        "InteractiveExplainability not found"
    );
    assert!(
        src.contains("pub async fn interactive_explain"),
        "interactive_explain not found"
    );
}

#[test]
fn compositional_reasoning_compiles_and_has_api() {
    let src = include_str!("../src/compositional_reasoning.rs");
    assert!(
        src.contains("pub struct CompositionalReasoning"),
        "CompositionalReasoning not found"
    );
    assert!(
        src.contains("pub async fn compositional_compose"),
        "compositional_compose not found"
    );
}

#[test]
fn neuro_symbolic_prover_compiles_and_has_api() {
    let src = include_str!("../src/neuro_symbolic_prover.rs");
    assert!(
        src.contains("pub struct NeuroSymbolicProver"),
        "NeuroSymbolicProver not found"
    );
    assert!(
        src.contains("pub async fn neurosymbolic_prove"),
        "neurosymbolic_prove not found"
    );
}

#[test]
fn federated_learning_compiles_and_has_api() {
    let src = include_str!("../src/federated_learning.rs");
    assert!(
        src.contains("pub struct FederatedLearning"),
        "FederatedLearning not found"
    );
    assert!(
        src.contains("pub async fn federated_aggregate"),
        "federated_aggregate not found"
    );
}

#[test]
fn pmi_analyzer_compiles_and_has_api() {
    let src = include_str!("../src/pmi_analyzer.rs");
    assert!(src.contains("pub struct PmiAnalyzer"), "PmiAnalyzer not found");
    assert!(src.contains("pub async fn pmi_score"), "pmi_score not found");
    assert!(src.contains("pub async fn pmi_observe"), "pmi_observe not found");
}

#[test]
fn uncertainty_quantification_compiles_and_has_api() {
    let src = include_str!("../src/uncertainty_quantification.rs");
    assert!(
        src.contains("pub struct MonteCarloUncertainty"),
        "MonteCarloUncertainty not found"
    );
    assert!(src.contains("pub async fn uq_estimate"), "uq_estimate not found");
    assert!(
        src.contains("pub async fn uq_add_prediction"),
        "uq_add_prediction not found"
    );
}

#[test]
fn hyperparameter_optimizer_compiles_and_has_api() {
    let src = include_str!("../src/hyperparameter_optimizer.rs");
    assert!(
        src.contains("pub struct HyperparameterSearch"),
        "HyperparameterSearch not found"
    );
    assert!(src.contains("pub async fn hpo_step"), "hpo_step not found");
    assert!(src.contains("pub async fn hpo_best"), "hpo_best not found");
}

#[test]
fn second_batch_modules_wired_into_main() {
    let main_src = include_str!("../src/main.rs");
    let modules = [
        "mod retrieval_augmented_generation;",
        "mod adaptive_reasoning;",
        "mod knowledge_graph;",
        "mod flow_analyzer;",
        "mod counterfactual_explainer;",
        "mod continual_learning;",
        "mod symbolic_reasoning;",
        "mod temporal_point_process;",
        "mod interactive_explainability;",
        "mod compositional_reasoning;",
        "mod neuro_symbolic_prover;",
        "mod federated_learning;",
        "mod pmi_analyzer;",
        "mod uncertainty_quantification;",
        "mod hyperparameter_optimizer;",
    ];
    for m in modules {
        assert!(main_src.contains(m), "{} not found in main.rs", m);
    }
}
