// Captures the *exact* pinned versions of the current build environment as a
// syntactically valid Nix expression (an attribute set of version strings),
// for storing in `ReproducibilityConfig.nix_expression`. This deliberately
// does not attempt to produce a buildable `mkShell`/derivation — mapping an
// arbitrary pip/cargo package to the matching nixpkgs derivation name isn't
// something that can be done generically/correctly. What's real here is
// the version data itself: read from `Cargo.lock` (a simple, stable format —
// parsed directly rather than pulling in a TOML dependency for it) and from
// `pip freeze` (run as a real subprocess, not shelled out to a fictional
// tool).
use std::path::Path;
use std::process::Command;

pub fn capture_nix_environment() -> String {
    let cargo_lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.lock");
    let rust_crates = read_cargo_lock(&cargo_lock_path).unwrap_or_default();
    let python_packages = run_pip_freeze().unwrap_or_default();

    format_nix_expression(&rust_crates, &python_packages)
}

fn read_cargo_lock(path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut crates = Vec::new();
    let mut name: Option<String> = None;
    for line in content.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            name = None;
        } else if let Some(value) = line.strip_prefix("name = ") {
            name = Some(value.trim_matches('"').to_string());
        } else if let Some(value) = line.strip_prefix("version = ") {
            if let Some(n) = name.take() {
                crates.push(format!("{n}-{}", value.trim_matches('"')));
            }
        }
    }
    Some(crates)
}

fn run_pip_freeze() -> Option<Vec<String>> {
    let output = Command::new("python").args(["-m", "pip", "freeze"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
    )
}

fn format_nix_expression(rust_crates: &[String], python_packages: &[String]) -> String {
    let format_list = |items: &[String]| {
        items
            .iter()
            .map(|item| format!("    {item:?}"))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "{{\n  # Exact pinned versions captured from Cargo.lock and `pip freeze`.\n  # Not a buildable derivation — see core/src/utils/nix.rs for why.\n  rustCrates = [\n{}\n  ];\n  pythonPackages = [\n{}\n  ];\n}}\n",
        format_list(rust_crates),
        format_list(python_packages),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_a_valid_nix_attrset_shape() {
        let expr = format_nix_expression(
            &["serde-1.0.228".to_string()],
            &["torch==2.0.0".to_string()],
        );
        assert!(expr.starts_with('{'));
        assert!(expr.trim_end().ends_with('}'));
        assert!(expr.contains("\"serde-1.0.228\""));
        assert!(expr.contains("\"torch==2.0.0\""));
    }

    #[test]
    fn reads_real_names_and_versions_from_a_cargo_lock() {
        let dir = std::env::temp_dir().join("bb_nix_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Cargo.lock");
        std::fs::write(
            &path,
            "[[package]]\nname = \"serde\"\nversion = \"1.0.228\"\n\n[[package]]\nname = \"tokio\"\nversion = \"1.52.3\"\n",
        )
        .unwrap();

        let crates = read_cargo_lock(&path).unwrap();
        assert_eq!(crates, vec!["serde-1.0.228", "tokio-1.52.3"]);
    }

    #[test]
    fn capture_nix_environment_reads_this_workspaces_real_cargo_lock() {
        let expr = capture_nix_environment();
        // This repo's own Cargo.lock genuinely has these — proves the
        // parser is reading real data, not returning a canned string.
        assert!(expr.contains("\"tokio-"));
        assert!(expr.contains("\"serde-"));
    }
}
