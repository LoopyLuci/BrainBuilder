/// Generates a component EDN descriptor from a user-supplied input/output
/// shape example (the visual designer's "infer from example" flow). Real,
/// working code — `tests::round_trips_through_the_real_edn_parser` proves
/// the output actually parses back through `ComponentDescriptor`'s real
/// `edn_rs::Deserialize` impl, not just that this string formats without
/// panicking.
pub fn generate_from_example(input_shape: Vec<usize>, output_shape: Vec<usize>) -> String {
    format!(
        r#"{{:component/id ""
 :component/name "custom-{}"
 :meta-type "function"
 :interface/inputs [{{:name "x" :tensor {{:shape {} :dtype "float32"}}}}]
 :interface/outputs [{{:name "y" :tensor {{:shape {} :dtype "float32"}}}}]
 :hyperparameters {{}}
 :implementation [{{:language "rust" :entry "custom_forward"}}]
 :compatibility {{:devices ["cpu"] :dtypes ["float32"] :autograd false}}}}
"#,
        uuid::Uuid::new_v4(),
        serde_json::to_string(&input_shape).unwrap(),
        serde_json::to_string(&output_shape).unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::descriptor::ComponentDescriptor;

    #[test]
    fn round_trips_through_the_real_edn_parser() {
        let edn = generate_from_example(vec![1, 3, 32, 32], vec![1, 10]);
        let desc: ComponentDescriptor = edn_rs::from_str(&edn).expect("generated EDN failed to parse");
        assert!(desc.name.starts_with("custom-"));
        assert_eq!(desc.inputs.len(), 1);
        assert_eq!(desc.outputs.len(), 1);
        assert_eq!(desc.implementations[0].language, "rust");
        assert_eq!(desc.implementations[0].entry, "custom_forward");
    }
}
