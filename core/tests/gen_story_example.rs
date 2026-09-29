// Not a real test — a one-off generator for gui/examples/story.txt and
// gui/examples/story.bbir.edn, run manually (`cargo test --test
// gen_story_example -- --ignored --nocapture`) whenever the bundled example
// needs regenerating.
//
// A third bundled example, and the first to prove the `attention` component
// trains through the real production path (BBIRGraph -> Orchestrator ->
// text_sequence dataset), not just the synthetic hand-built-ops tests in
// transformer_components.rs. The task: predict the next word of a tiny
// four-sentence story cycle. Crucially the graph includes real self-attention
// over the whole context window (not just the immediately preceding word),
// because "the" alone precedes four different possible next words across the
// four sentences — a bigram model (bare embedding -> select_last -> linear,
// as in sequence_training.rs) cannot disambiguate that, but attending over
// the full seq_len window can, and does (see story_teller.rs's accuracy
// assertion).
//
// `embedding_dim` on the attention node must be an exact multiple of
// `num_heads`, and the embedding/linear-head `vocab_size` must exactly equal
// the real realized vocabulary width for the corpus — computed here via the
// same real `data::text::Vocab::build` call the training path uses, so it
// can never drift out of sync with the corpus.
use brainbuilder_core::bbir::{
    BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, NodePosition, PortInfo, TrainingConfig,
};
use brainbuilder_core::data::text::Vocab;

fn node(id: &str, component: &str, label: &str, inputs: &[&str], outputs: &[&str], hp: serde_json::Value, x: f64) -> BBIRNode {
    BBIRNode {
        id: id.to_string(),
        component: component.to_string(),
        label: Some(label.to_string()),
        hyperparams: hp,
        ports: PortInfo {
            input_ports: inputs.iter().map(|s| s.to_string()).collect(),
            output_ports: outputs.iter().map(|s| s.to_string()).collect(),
        },
        position: Some(NodePosition { x, y: 150.0 }),
    }
}

fn edge(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> BBIREdge {
    BBIREdge {
        from_node: from_node.to_string(),
        from_port: from_port.to_string(),
        to_node: to_node.to_string(),
        to_port: to_port.to_string(),
    }
}

#[test]
#[ignore]
fn generate_story_example() {
    let gui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui");

    // A 4-sentence cycle, repeated many times so there's enough real
    // next-token windows to train on. Every sentence shares filler words
    // ("the", "and", "into") but diverges quickly, so a genuinely
    // context-aware model (not a bigram) can learn to tell them apart.
    let cycle = "the wizard cast a spell and vanished into the night \
the dragon woke and breathed fire across the sky \
the knight raised his sword and charged into the fray \
the princess opened the door and smiled at the guard ";
    // Repeating the cycle doesn't add real information beyond a few reps —
    // it's a strictly periodic 39-token pattern, so windows from one
    // repetition are structurally identical to the next. Kept modest (not
    // e.g. 40x) so a full training run doesn't create an excessive number of
    // Python-bridge tensor scratch files (each op call round-trips through
    // one) — a real, if unrelated, Windows NTFS limitation this repo's own
    // `PythonBridge::write_tensor` scratch-file protocol can hit under heavy
    // sustained IO.
    let corpus = cycle.repeat(15);
    let text_path = gui_dir.join("examples/story.txt");
    std::fs::write(&text_path, &corpus).expect("write story.txt");

    // Comfortably above the real distinct-word count so the frequency cap
    // never actually kicks in (same reasoning as gen_sentiment_example.rs).
    let vocab_cap = 60usize;
    let vocab = Vocab::build(&corpus, vocab_cap);
    let vocab_size = vocab.len();
    println!("real corpus vocabulary width: {vocab_size}");

    let embedding_dim = 32;
    let num_heads = 4;
    let seq_len = 8;

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "00000000-0000-0000-0000-000000000003".to_string(),
        name: "story-teller-example".to_string(),
        nodes: vec![
            node(
                "embed",
                "embedding",
                "Embedding",
                &["ids", "weight"],
                &["output"],
                serde_json::json!({"vocab_size": vocab_size, "embedding_dim": embedding_dim}),
                80.0,
            ),
            node(
                "attn",
                "attention",
                "Self-Attention",
                &["input", "q_weight", "k_weight", "v_weight", "out_weight"],
                &["output"],
                serde_json::json!({"features": embedding_dim, "num_heads": num_heads}),
                320.0,
            ),
            node("pool", "select_last", "Select Last", &["input"], &["output"], serde_json::json!({}), 560.0),
            node(
                "proj",
                "linear",
                "Next-Word Head",
                &["input", "weight"],
                &["output"],
                serde_json::json!({"in_features": embedding_dim, "out_features": vocab_size}),
                800.0,
            ),
        ],
        edges: vec![
            edge("embed", "output", "attn", "input"),
            edge("attn", "output", "pool", "input"),
            edge("pool", "output", "proj", "input"),
        ],
        training: Some(TrainingConfig {
            loss: "cross_entropy".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.02, "epochs": 300}),
            data_source: DataSourceConfig {
                source_type: "text_sequence".to_string(),
                // Relative to the Tauri backend's working directory at
                // runtime (cwd = gui/), same convention `first_run`/
                // `sentiment` use — resolves to gui/examples/story.txt.
                path_or_uri: "examples/story.txt".to_string(),
                // Larger than the corpus's ~570 real windows -> exactly one
                // full-batch gradient step per epoch, trading batch-count
                // (and so scratch-file volume — see the note on `corpus`
                // above) for more epochs of a cleaner, less noisy gradient.
                batch_size: 1000,
                preprocessing: Vec::new(),
                sequence_length: Some(seq_len),
                vocab_size: Some(vocab_cap),
                image_size: None,
                grayscale: None,
                text_column: None,
                label_column: None,
            },
            reproducibility: None,
        }),
    };

    let edn = graph.to_edn().expect("serialize story example graph");
    let out = gui_dir.join("examples/story.bbir.edn");
    std::fs::write(&out, edn).expect("write example graph");
    println!("wrote {}", out.display());
}
