#!/usr/bin/env python3
"""Generate Rust facade modules for ALL model catalog domains including meta-models."""
import re
import json
from pathlib import Path
from collections import defaultdict

CATALOG = Path("Z:/Projects/BrainBuilder/docs/model-catalog")
MC_DIR = Path("Z:/Projects/BrainBuilder/gui/src-tauri/src/model_catalog")
REACT_SRC = Path("Z:/Projects/BrainBuilder/gui/src/next-gen")
SMOKE = Path("Z:/Projects/BrainBuilder/gui/src-tauri/tests/smoke_catalog_models.rs")


def parse():
    models = []
    seen_ids = {}
    for f in sorted(CATALOG.glob("[0-9][0-9]-*.md")):
        if f.name == "README.md":
            continue
        dc = f.name[:2]
        for m in re.finditer(r"\*\*(\d+)\. ([^*]+)\*\* — (.+?)(?:\n|$)", f.read_text(encoding="utf-8")):
            idx, name, desc = m.groups()
            name = name.strip()
            desc = desc.strip()[:220]
            words = re.split(r'[^a-z0-9]+', name.lower())
            words = [w for w in words if w]
            if words[0][0].isdigit():
                words[0] = 'n' + words[0]
            snake = '_'.join(words)
            if snake in seen_ids:
                seen_ids[snake] += 1
                snake = f"{snake}_{seen_ids[snake]}"
            else:
                seen_ids[snake] = 0
            pascal = ''.join(w.capitalize() for w in words)
            models.append({"domain": dc, "snake": snake, "pascal": pascal, "name": name, "desc": desc, "id": snake})
    return models


def registry_rs():
    return '''use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

#[async_trait::async_trait]
pub trait Model: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn execute(&self, _params: HashMap<String, Value>) -> Result<Value, String>;
}

pub struct ModelRegistry {
    models: HashMap<String, Arc<dyn Model>>,
}

impl ModelRegistry {
    pub fn new() -> Self { Self { models: HashMap::new() } }
    pub fn register(&mut self, model: Arc<dyn Model>) { self.models.insert(model.id().to_string(), model); }
    pub async fn execute(&self, id: &str, _params: HashMap<String, Value>) -> Result<Value, String> {
        match self.models.get(id) { Some(model) => model.execute(_params).await, None => Err(format!("Model '{}' not found", id)) }
    }
    pub fn list(&self) -> Vec<(&str, &str)> { self.models.values().map(|m| (m.id(), m.name())).collect() }
}
'''


def domain_rs(dc, models):
    lines = [
        "use crate::model_catalog::registry::{Model, ModelRegistry};",
        "use std::sync::Arc;",
        "use std::collections::HashMap;",
        "use serde_json::Value;",
        "use async_trait::async_trait;",
        "",
    ]
    for m in models:
        p, s, name, desc = m["pascal"], m["snake"], m["name"], m["desc"]
        name = name.replace('"', '\\"')[:90]
        desc = desc.replace('"', '\\"')[:120]
        lines.append(f"pub struct {p};")
        lines.append("#[async_trait::async_trait]")
        lines.append(f"impl Model for {p} {{")
        lines.append(f"    fn id(&self) -> &'static str {{ \"{s}\" }}")
        lines.append(f"    fn name(&self) -> &'static str {{ \"{name}\" }}")
        lines.append("    async fn execute(&self, _params: HashMap<String, Value>) -> Result<Value, String> {")
        lines.append(f"        Ok(Value::String(format!(\"{{}} executed\", self.name())))")
        lines.append("    }")
        lines.append("}")
        lines.append("")
    lines.append("pub fn register(registry: &mut ModelRegistry) {")
    for m in models:
        lines.append(f"    registry.register(Arc::new({m['pascal']}));")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def mod_rs(domains):
    lines = ["pub mod registry;"]
    for d in sorted(domains):
        lines.append(f"pub mod domain_{d};")
    lines += ["", "pub fn register_all(registry: &mut ModelRegistry) {"]
    for d in sorted(domains):
        lines.append(f"    domain_{d}::register(registry);")
    lines += ["}", ""]
    lines += [
        "use std::collections::HashMap;",
        "use serde_json::Value;",
        "use crate::model_catalog::registry::ModelRegistry;",
        "use crate::self_improving_commands::AppStateExt;",
        "use tokio::sync::Mutex;",
        "",
        "#[tauri::command]",
        "pub async fn catalog_model_run(",
        "    state: tauri::State<'_, std::sync::Arc<Mutex<AppStateExt>>>,",
        "    model_id: String,",
        "    params: HashMap<String, Value>,",
        ") -> Result<Value, String> {",
        "    let ext = state.lock().await;",
        "    let registry = ext.catalog_registry.read().await;",
        "    registry.execute(&model_id, params).await",
        "}",
        "",
    ]
    return "\n".join(lines)


def react_panel():
    return '''import { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { logError, logInfo } from '../console/logStore';

interface Props { modelId: string; title: string; description?: string; }

export default function CatalogModelPanel({ modelId, title, description }: Props) {
  const [output, setOutput] = useState('');
  const [loading, setLoading] = useState(false);
  const run = async () => {
    setLoading(true); setOutput('');
    try {
      const res = await invoke<Record<string, unknown>>('catalog_model_run', { modelId, params: {} });
      setOutput(JSON.stringify(res, null, 2));
      logInfo(`${title}: ${JSON.stringify(res)}`);
    } catch (e) { logError(`${title}: ${String(e)}`); setOutput(String(e)); }
    finally { setLoading(false); }
  };
  return (<div className="panel"><h3>{title}</h3>{description && <p className="text-xs text-gray-500 mb-2">{description}</p>}<button onClick={run} disabled={loading}>Run Model</button><pre className="mt-2">{output}</pre></div>);
}
'''


def _escape_title(name: str) -> str:
    return name.replace("'", "\\'")


def builtins_registrations(models):
    header = "// Auto-generated catalog registrations — do not edit by hand.\nimport CatalogModelPanel from '../next-gen/CatalogModelPanel';\nimport { registerWidget } from './registry';\nimport type { WidgetDef } from './types';\n\n"
    lines = []
    for i, m in enumerate(models):
        wid = m["id"]
        title = m["name"]
        name = f"W_{wid}"
        lines.append(f"const {name} = () => <CatalogModelPanel modelId='{wid}' title={json.dumps(title)} description={json.dumps('')} />;")
        lines.append(f"registerWidget({{ id: '{wid}', title: {json.dumps(title)}, slot: 'side', component: {name}, order: {200 + i} }} as WidgetDef);")
    return header + "\n".join(lines)


def smoke_test(models):
    lines = [
        "#[cfg(test)]",
        "mod catalog_tests {",
        "    #[test]",
        "    fn catalog_domain_files_exist() {",
        "        let root = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src/model_catalog\");",
        "        let expected = [",
    ]
    for d in sorted(set(m["domain"] for m in models)):
        lines.append(f'            "domain_{d}.rs",')
    lines += [
        "        ];",
        "        for name in &expected {",
        "            assert!(root.join(name).exists(), \"missing {}\", name);",
        "        }",
        "    }",
        "",
        "    #[test]",
        "    fn catalog_total_model_count() {",
        "        let root = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\")).join(\"src/model_catalog\");",
        "        let mut total = 0usize;",
        "        for entry in std::fs::read_dir(root).unwrap() {",
        "            let p = entry.unwrap().path();",
        "            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {",
        "                if name.starts_with(\"domain_\") && name.ends_with(\".rs\") {",
        "                    let content = std::fs::read_to_string(&p).unwrap();",
        "                    total += content.matches(\"pub struct \").count();",
        "                }",
        "            }",
        "        }",
        '        assert!(total >= 1050, "expected >=1050 catalog models, got {}", total);',
        "    }",
        "}",
        "",
    ]
    return "\n".join(lines)


def main():
    models = parse()
    print(f"Parsed {len(models)} catalog models")
    MC_DIR.mkdir(parents=True, exist_ok=True)
    REACT_SRC.mkdir(parents=True, exist_ok=True)
    SMOKE.parent.mkdir(parents=True, exist_ok=True)
    (MC_DIR / "registry.rs").write_text(registry_rs(), encoding="utf-8")
    print("Wrote registry.rs")
    by_domain = defaultdict(list)
    for m in models:
        by_domain[m["domain"]].append(m)
    for dc in sorted(by_domain):
        (MC_DIR / f"domain_{dc}.rs").write_text(domain_rs(dc, by_domain[dc]), encoding="utf-8")
        print(f"Wrote domain_{dc}.rs ({len(by_domain[dc])} models)")
    (MC_DIR / "mod.rs").write_text(mod_rs(set(m["domain"] for m in models)), encoding="utf-8")
    (REACT_SRC / "CatalogModelPanel.tsx").write_text(react_panel(), encoding="utf-8")
    Path("Z:/Projects/BrainBuilder/gui/src/widgets/builtins_catalog.tsx").write_text(builtins_registrations(models), encoding="utf-8")
    (SMOKE).write_text(smoke_test(models), encoding="utf-8")
    print("All files regenerated.")


if __name__ == "__main__":
    main()
