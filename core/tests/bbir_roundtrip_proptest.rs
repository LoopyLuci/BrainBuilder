// Property-based counterpart to bbir_roundtrip.rs's hand-picked examples:
// generates many random graphs (node/edge counts, hyperparameter shapes,
// numeric ranges) and checks every one survives a BBIRGraph -> EDN -> BBIRGraph
// round trip exactly. bbir_roundtrip.rs already caught one real bug this way
// (edn_rs's own `json_to_edn` mangling numeric object values) — proptest is
// what actually explores the space of "which numbers/keys/strings" instead
// of relying on whichever cases a person happened to write by hand.
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, PortInfo};
use proptest::prelude::*;

/// Snake_case-shaped hyperparameter keys, matching real component
/// descriptors (`out_features`, `vocab_size`, ...) — this is the exact shape
/// `camel_to_snake`/`edn_to_json`'s camelCasing round trip has to preserve
/// correctly (see runtime/scheduler.rs's documented real bug history).
fn hyperparam_key() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{0,6}(_[a-z0-9]{1,6}){0,2}"
}

fn hyperparam_value() -> impl Strategy<Value = serde_json::Value> {
    prop_oneof![
        any::<i64>().prop_map(|n| serde_json::json!(n)),
        (-1_000_000f64..1_000_000f64).prop_map(|f| serde_json::json!(f)),
        "[a-zA-Z0-9 _-]{0,20}".prop_map(|s| serde_json::json!(s)),
        any::<bool>().prop_map(|b| serde_json::json!(b)),
    ]
}

fn hyperparams() -> impl Strategy<Value = serde_json::Map<String, serde_json::Value>> {
    prop::collection::hash_map(hyperparam_key(), hyperparam_value(), 0..6)
        .prop_map(|m| m.into_iter().collect::<serde_json::Map<_, _>>())
}

fn node_id() -> impl Strategy<Value = String> {
    "n[0-9]{1,3}"
}

fn component_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("linear".to_string()),
        Just("relu".to_string()),
        Just("layernorm".to_string()),
        Just("embedding".to_string()),
        Just("attention".to_string()),
    ]
}

fn arb_node() -> impl Strategy<Value = BBIRNode> {
    (node_id(), component_name(), hyperparams()).prop_map(|(id, component, hp)| BBIRNode {
        id,
        component,
        label: None,
        hyperparams: serde_json::Value::Object(hp),
        ports: PortInfo { input_ports: vec![], output_ports: vec![] },
        position: None,
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Every hyperparameter key/value survives the round trip with the
    /// *exact* same key spelling and value — the real regression class this
    /// guards against is `edn_to_json`'s camelCasing of dash-keywords and
    /// `json_to_edn`'s numeric-value serialization, both bugs that already
    /// bit this codebase once (see runtime/scheduler.rs, bbir_roundtrip.rs).
    #[test]
    fn hyperparams_round_trip_exactly(nodes in prop::collection::vec(arb_node(), 0..5)) {
        // Real components only ever appear once per graph in this test's
        // node-id scheme (n0, n1, ...), but two random draws could collide
        // on id — dedupe so we're testing the round trip, not a map-key clash.
        let mut seen = std::collections::HashSet::new();
        let nodes: Vec<BBIRNode> = nodes.into_iter().filter(|n| seen.insert(n.id.clone())).collect();

        let graph = BBIRGraph {
            schema_version: 1,
            graph_id: "prop-graph".to_string(),
            name: "prop-test".to_string(),
            nodes: nodes.clone(),
            edges: vec![],
            training: None,
        };

        let edn = graph.to_edn().expect("serialize to edn");
        let parsed = BBIRGraph::from_edn(&edn).expect("parse edn back");

        prop_assert_eq!(parsed.nodes.len(), nodes.len());
        for (original, roundtripped) in nodes.iter().zip(parsed.nodes.iter()) {
            prop_assert_eq!(&roundtripped.id, &original.id);
            prop_assert_eq!(&roundtripped.component, &original.component);
            let original_hp = original.hyperparams.as_object().unwrap();
            let roundtripped_hp = roundtripped.hyperparams.as_object().unwrap();
            prop_assert_eq!(roundtripped_hp.len(), original_hp.len());
            for (key, value) in original_hp {
                let got = roundtripped_hp.get(key);
                match value {
                    serde_json::Value::Number(n) if n.as_f64().map(|f| f.fract() != 0.0).unwrap_or(false) => {
                        // Floats: compare numerically (EDN's own text
                        // formatting may not be byte-identical) rather than
                        // requiring exact serde_json::Value equality.
                        let expected = n.as_f64().unwrap();
                        let got_f = got.and_then(|v| v.as_f64());
                        prop_assert!(
                            got_f.map(|g| (g - expected).abs() < 1e-6).unwrap_or(false),
                            "key `{key}`: expected ~{expected}, got {got:?}"
                        );
                    }
                    other => {
                        prop_assert_eq!(got, Some(other), "key `{}` mismatched after round trip", key);
                    }
                }
            }
        }
    }

    /// Edge wiring (which node/port connects to which) survives the round
    /// trip regardless of how many nodes/edges or what they're named.
    #[test]
    fn edges_round_trip_exactly(
        node_ids in prop::collection::vec(node_id(), 2..6),
        edge_indices in prop::collection::vec((0usize..5, 0usize..5), 0..8),
    ) {
        let mut seen = std::collections::HashSet::new();
        let node_ids: Vec<String> = node_ids.into_iter().filter(|id| seen.insert(id.clone())).collect();
        prop_assume!(node_ids.len() >= 2);

        let nodes: Vec<BBIRNode> = node_ids
            .iter()
            .map(|id| BBIRNode {
                id: id.clone(),
                component: "linear".to_string(),
                label: None,
                hyperparams: serde_json::json!({}),
                ports: PortInfo { input_ports: vec![], output_ports: vec![] },
                position: None,
            })
            .collect();

        let edges: Vec<BBIREdge> = edge_indices
            .into_iter()
            .filter(|(a, b)| *a < node_ids.len() && *b < node_ids.len())
            .map(|(a, b)| BBIREdge {
                from_node: node_ids[a].clone(),
                from_port: "output".to_string(),
                to_node: node_ids[b].clone(),
                to_port: "input".to_string(),
            })
            .collect();

        let graph = BBIRGraph {
            schema_version: 1,
            graph_id: "prop-edges".to_string(),
            name: "prop-edge-test".to_string(),
            nodes,
            edges: edges.clone(),
            training: None,
        };

        let edn = graph.to_edn().expect("serialize to edn");
        let parsed = BBIRGraph::from_edn(&edn).expect("parse edn back");

        prop_assert_eq!(parsed.edges.len(), edges.len());
        for (original, roundtripped) in edges.iter().zip(parsed.edges.iter()) {
            prop_assert_eq!(&roundtripped.from_node, &original.from_node);
            prop_assert_eq!(&roundtripped.from_port, &original.from_port);
            prop_assert_eq!(&roundtripped.to_node, &original.to_node);
            prop_assert_eq!(&roundtripped.to_port, &original.to_port);
        }
    }
}
