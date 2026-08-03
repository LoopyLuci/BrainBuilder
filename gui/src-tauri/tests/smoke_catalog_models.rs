#[cfg(test)]
mod catalog_tests {
    #[test]
    fn catalog_domain_files_exist() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/model_catalog");
        let expected = [
            "domain_01.rs",
            "domain_02.rs",
            "domain_03.rs",
            "domain_04.rs",
            "domain_05.rs",
            "domain_06.rs",
            "domain_07.rs",
            "domain_08.rs",
            "domain_09.rs",
            "domain_10.rs",
            "domain_11.rs",
            "domain_12.rs",
            "domain_13.rs",
            "domain_14.rs",
            "domain_15.rs",
            "domain_16.rs",
            "domain_17.rs",
            "domain_18.rs",
            "domain_19.rs",
            "domain_20.rs",
            "domain_21.rs",
        ];
        for name in &expected {
            assert!(root.join(name).exists(), "missing {}", name);
        }
    }

    #[test]
    fn catalog_total_model_count() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/model_catalog");
        let mut total = 0usize;
        for entry in std::fs::read_dir(root).unwrap() {
            let p = entry.unwrap().path();
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("domain_") && name.ends_with(".rs") {
                    let content = std::fs::read_to_string(&p).unwrap();
                    total += content.matches("pub struct ").count();
                }
            }
        }
        assert!(total >= 1050, "expected >=1050 catalog models, got {}", total);
    }
}
