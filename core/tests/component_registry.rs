use brainbuilder_core::component::registry::ComponentRegistry;
use std::path::Path;

#[test]
fn loads_all_shipped_components() {
    let mut registry = ComponentRegistry::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    registry
        .load_from_dir(&dir)
        .expect("failed to parse one of the shipped .edn component files");

    let names = registry.list_names();
    for expected in ["conv2d", "linear", "relu", "adam", "image_classifier_pipeline"] {
        assert!(
            names.contains(&expected.to_string()),
            "expected component `{expected}` to be loaded, got: {names:?}"
        );
    }

    let conv2d = registry.get_by_name("conv2d").expect("conv2d missing");
    assert_eq!(conv2d.inputs.len(), 2);
    assert_eq!(conv2d.inputs[0].name, "x");
    assert_eq!(conv2d.hyperparameters.len(), 3);
    assert!(conv2d.compatibility.autograd);
    assert_eq!(conv2d.tests.len(), 1);
}
