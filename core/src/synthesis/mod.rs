//! Component **synthesis**: turning a plain-English description into a brand-new
//! BrainBuilder component (an EDN descriptor + a Python kernel), rather than
//! merely assembling existing ones. This is what makes the component library
//! open-ended — a user can ask for an architecture no one hand-wrote.
//!
//! Generated code is **untrusted by default**. Nothing synthesized is ever
//! trusted or installed until it clears a validation gauntlet:
//!
//! 1. **Parse** — the descriptor must deserialize as a real
//!    [`ComponentDescriptor`], its name must be fresh (no clobbering a
//!    built-in), and the kernel must define the declared entry function.
//! 2. **Structure** — at least one output, well-formed ports, and a smoke test
//!    whose declared shapes line up with the interface.
//! 3. **Sandboxed smoke test** — the kernel actually runs, on tiny synthetic
//!    tensors, **inside the nervous-system sandbox** ([`Supervisor::run_checked`]
//!    + [`Capabilities`]: read-only temp dir, no network, a wall-clock + memory
//!    ceiling), and its real output shape must match what the descriptor
//!    promises. Only on green is the component written to disk + hot-registered.
//!
//! The parse/structure half is fully unit-tested here; the smoke-test half
//! reuses the exact sandbox every other runtime goes through, so a synthesized
//! kernel is contained the same way Racket/Clojure/Python component code is.
use crate::component::descriptor::{ComponentDescriptor, DataType};
use crate::component::registry::ComponentRegistry;
use crate::component::validation::validate_descriptor_self;
use crate::interop::protocol::BrainBuilderError;
use crate::llm::provider::LlmProvider;
use crate::runtime::nervous_system::{Capabilities, Supervisor};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// The model's raw synthesis output, once parsed. Holds both the on-disk
/// artifacts (EDN + Python) and the already-parsed descriptor so callers don't
/// re-parse.
#[derive(Debug, Clone, Serialize)]
pub struct SynthesizedComponent {
    pub name: String,
    pub descriptor_edn: String,
    pub python_code: String,
    #[serde(skip)]
    pub descriptor: ComponentDescriptor,
    pub smoke_test: SmokeTest,
}

/// Concrete tiny inputs for the sandboxed smoke test, one shape per forward
/// argument (data + parameter ports, in declared order), plus the output shape
/// the kernel must actually produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmokeTest {
    pub input_shapes: Vec<Vec<i64>>,
    pub expected_shape: Vec<i64>,
}

/// What the model is asked to emit — kept as a private DTO so the public
/// `SynthesizedComponent` can carry the parsed descriptor too.
#[derive(Debug, Deserialize)]
struct RawSynthesis {
    name: String,
    descriptor_edn: String,
    python_code: String,
    smoke_test: SmokeTest,
}

/// The entry function name we require the kernel to define. Matches the
/// convention every shipped component uses (`:entry "forward"`).
const DEFAULT_ENTRY: &str = "forward";

/// Build the synthesis system prompt. Shows the model a couple of *real*
/// shipped components as worked examples (so it matches the exact EDN dialect
/// and kernel shape) and forbids reusing an existing component name.
pub fn build_synthesis_prompt(existing_names: &[String]) -> String {
    let taken = existing_names.join(", ");
    format!(
        "You are a component-synthesis assistant for BrainBuilder. Given a plain-English \
description of ONE neural-network component, output ONLY a single JSON object (no markdown, no \
prose) with exactly these keys:\n\
{{\n\
  \"name\": \"<short snake_case name, NOT one of the existing names>\",\n\
  \"descriptor_edn\": \"<an EDN component descriptor, see format below>\",\n\
  \"python_code\": \"<a Python module defining `def forward(...)` using torch>\",\n\
  \"smoke_test\": {{\"input_shapes\": [[...], ...], \"expected_shape\": [...]}}\n\
}}\n\n\
The EDN descriptor format (match it EXACTLY, including the `component/` and `interface/` key \
prefixes):\n\
{{:component/id \"\"\n \
 :component/name \"<same as name>\"\n \
 :meta-type \"pure-function\"\n \
 :version \"1.0.0\"\n \
 :tags [\"...\"]\n \
 :interface/inputs [{{:name \"input\" :tensor {{:shape [:batch :features] :dtype \"float32\"}}}}]\n \
 :interface/outputs [{{:name \"output\" :tensor {{:shape [:batch :features] :dtype \"float32\"}}}}]\n \
 :hyperparameters {{}}\n \
 :implementation [{{:language \"python\" :entry \"forward\"}}]\n \
 :compatibility {{:devices [\"cpu\"] :dtypes [\"float32\"] :autograd true}}}}\n\n\
Rules:\n\
- `python_code` MUST define `def forward(...)` whose positional args are the interface inputs in \
order, and MUST return a torch tensor. Use only `torch` (already importable).\n\
- `smoke_test.input_shapes` gives one concrete small shape per forward argument, in order; \
`expected_shape` is the exact shape `forward` returns for those inputs.\n\
- Do not reuse any of these existing component names: {taken}.\n\
- Keep it a single self-contained component. No file I/O, no network, no side effects.\n"
    )
}

/// Build the *repair* user-message for a second synthesis attempt: show the
/// model its own failed output and the exact reason it failed, and ask for a
/// corrected object. Paired with the same system prompt as the first try, this
/// turns most first-shot failures (a shape mismatch, a missing entry fn, a name
/// collision) into a usable component instead of a dead end — the difference
/// between a zero-knowledge user succeeding and giving up.
pub fn build_repair_request(description: &str, previous_json: &str, failure: &str) -> String {
    format!(
        "Your previous attempt to synthesize a component FAILED validation and was NOT accepted.\n\n\
Original request:\n{description}\n\n\
Your previous output:\n{previous_json}\n\n\
It failed with this error:\n{failure}\n\n\
Return a corrected single JSON object with the SAME keys, fixing exactly what the error \
describes (adjust the descriptor, kernel, or smoke_test shapes as needed so they agree). \
Output ONLY the JSON object, no prose."
    )
}

/// Parse + statically validate the model's output. This is the first two gates
/// of the gauntlet (parse + structure); it never runs code. `registry` is used
/// only to reject a name collision with an existing component.
pub fn parse_synthesis_output(raw: &str, registry: &ComponentRegistry) -> Result<SynthesizedComponent> {
    let parsed: RawSynthesis = serde_json::from_str(raw).map_err(|e| {
        BrainBuilderError::Parse(format!("synthesis output wasn't the expected JSON ({e}) — raw:\n{raw}"))
    })?;

    if parsed.name.trim().is_empty() {
        return Err(BrainBuilderError::ConfigError("synthesized component has an empty name".into()));
    }
    if registry.get_by_name(&parsed.name).is_some() {
        return Err(BrainBuilderError::ConfigError(format!(
            "a component named `{}` already exists — synthesis must produce a new name",
            parsed.name
        )));
    }

    let descriptor: ComponentDescriptor = edn_rs::from_str(&parsed.descriptor_edn)
        .map_err(|e| BrainBuilderError::Parse(format!("synthesized descriptor EDN didn't parse: {e:?}")))?;

    if descriptor.name != parsed.name {
        return Err(BrainBuilderError::ConfigError(format!(
            "descriptor name `{}` doesn't match component name `{}`",
            descriptor.name, parsed.name
        )));
    }

    // Reuse the same self-consistency checks the rest of the system trusts.
    validate_descriptor_self(&descriptor)?;

    // The kernel must actually define the entry function the descriptor points
    // at (default `forward`) — a cheap guard against an empty/garbage kernel
    // before we pay for a sandboxed run.
    let entry = descriptor
        .implementations
        .iter()
        .find(|i| i.language == "python")
        .map(|i| i.entry.as_str())
        .unwrap_or(DEFAULT_ENTRY);
    if !parsed.python_code.contains(&format!("def {entry}")) {
        return Err(BrainBuilderError::ConfigError(format!(
            "kernel doesn't define `def {entry}(...)` that the descriptor's implementation entry names"
        )));
    }

    // The smoke test must cover every forward argument (all input ports).
    if parsed.smoke_test.input_shapes.len() != descriptor.inputs.len() {
        return Err(BrainBuilderError::ConfigError(format!(
            "smoke test gives {} input shape(s) but the component declares {} input port(s)",
            parsed.smoke_test.input_shapes.len(),
            descriptor.inputs.len()
        )));
    }
    if parsed.smoke_test.expected_shape.is_empty() {
        return Err(BrainBuilderError::ConfigError("smoke test is missing an expected output shape".into()));
    }

    Ok(SynthesizedComponent {
        name: parsed.name,
        descriptor_edn: parsed.descriptor_edn,
        python_code: parsed.python_code,
        descriptor,
        smoke_test: parsed.smoke_test,
    })
}

/// Full synthesis: prompt a provider, then run the static half of the gauntlet.
/// The sandboxed smoke test ([`run_smoke_test`]) and install
/// ([`install_synthesized`]) are separate so a caller (the Tauri command) can
/// surface the smoke result to the user before anything is written to disk.
pub async fn synthesize_component(
    description: &str,
    provider: &dyn LlmProvider,
    model: &str,
    registry: &ComponentRegistry,
) -> Result<SynthesizedComponent> {
    let system = build_synthesis_prompt(&registry.list_names());
    let raw = provider.generate_json(model, &system, description).await?;
    parse_synthesis_output(&raw, registry)
}

/// Outcome of the sandboxed smoke test.
#[derive(Debug, Clone, Serialize)]
pub struct SmokeReport {
    pub passed: bool,
    pub actual_shape: Option<Vec<i64>>,
    pub detail: String,
}

/// Third gate: actually run the kernel on synthetic tensors **inside the
/// nervous-system sandbox**, and confirm it produces the promised output shape.
/// The kernel is written to a throwaway temp dir the sandbox is granted
/// read-only; network is denied and a wall-clock + memory ceiling apply, so a
/// pathological generated kernel can't escape, hang, or exhaust the machine.
pub fn run_smoke_test(component: &SynthesizedComponent) -> Result<SmokeReport> {
    let dir = std::env::temp_dir().join(format!("bb_synth_{}_{}", std::process::id(), sanitize(&component.name)));
    std::fs::create_dir_all(&dir)
        .map_err(|e| BrainBuilderError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    let kernel_path = dir.join(format!("{}.py", sanitize(&component.name)));
    std::fs::write(&kernel_path, &component.python_code)?;

    let driver = build_smoke_driver(component);
    let driver_path = dir.join("_bb_smoke_driver.py");
    std::fs::write(&driver_path, &driver)?;

    let mut caps = Capabilities::none()
        .allow_read(&dir)
        .with_timeout(Duration::from_secs(30))
        .with_memory_limit(1024 * 1024 * 1024); // 1 GiB ceiling for a smoke run
    // The kernel imports torch — grant read of whatever's already on PYTHONPATH
    // so the interpreter can find it, exactly like PythonBridge does.
    if let Ok(pythonpath) = std::env::var("PYTHONPATH") {
        for entry in std::env::split_paths(&pythonpath) {
            caps = caps.allow_read(entry);
        }
    }

    let mut command = Command::new("python");
    command.arg(&driver_path);
    // The kernel module lives beside the driver; put its dir on the child's
    // import path.
    command.env("PYTHONPATH", pythonpath_with(&dir));

    let output = Supervisor::run_checked_named("synthesis-smoke", &mut command, &caps, &[&kernel_path], None)?;

    let report = interpret_smoke_output(&output, &component.smoke_test.expected_shape);
    // Best-effort cleanup — a leftover temp dir is harmless, a panic isn't.
    std::fs::remove_dir_all(&dir).ok();
    Ok(report)
}

/// Writes the validated component to `components_dir` (descriptor `.edn`) and
/// `components_dir/python/<name>.py` (kernel). Call this ONLY after
/// [`run_smoke_test`] passes. Returns the descriptor path so the caller can
/// hand it to `Orchestrator::install_component` for live registration.
pub fn install_synthesized(component: &SynthesizedComponent, components_dir: &Path) -> Result<std::path::PathBuf> {
    let name = sanitize(&component.name);
    let python_dir = components_dir.join("python");
    std::fs::create_dir_all(&python_dir)?;
    std::fs::write(python_dir.join(format!("{name}.py")), &component.python_code)?;
    let edn_path = components_dir.join(format!("{name}.edn"));
    std::fs::write(&edn_path, &component.descriptor_edn)?;
    Ok(edn_path)
}

// --- helpers ---

fn sanitize(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect()
}

fn pythonpath_with(dir: &Path) -> String {
    match std::env::var("PYTHONPATH") {
        Ok(existing) => format!("{}{}{}", dir.display(), path_sep(), existing),
        Err(_) => dir.display().to_string(),
    }
}

fn path_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

/// Builds the tiny Python driver that constructs synthetic inputs of the
/// declared shapes/dtypes, calls the kernel's entry function, and prints the
/// real output shape as JSON on the last line.
fn build_smoke_driver(component: &SynthesizedComponent) -> String {
    let module = sanitize(&component.name);
    let entry = component
        .descriptor
        .implementations
        .iter()
        .find(|i| i.language == "python")
        .map(|i| i.entry.clone())
        .unwrap_or_else(|| DEFAULT_ENTRY.to_string());

    // Per-arg dtype so integer ports (e.g. embedding indices) get randint, not
    // randn. Falls back to float for anything we don't special-case.
    let dtypes: Vec<&str> = component
        .descriptor
        .inputs
        .iter()
        .map(|p| match p.tensor.dtype {
            DataType::Int32 | DataType::Int64 => "int",
            _ => "float",
        })
        .collect();

    let shapes_json = serde_json::to_string(&component.smoke_test.input_shapes).unwrap_or_else(|_| "[]".into());
    let dtypes_json = serde_json::to_string(&dtypes).unwrap_or_else(|_| "[]".into());

    format!(
        "import json\n\
import torch\n\
import {module} as kernel\n\
shapes = json.loads('{shapes_json}')\n\
dtypes = json.loads('{dtypes_json}')\n\
args = []\n\
for shape, dt in zip(shapes, dtypes):\n\
    if dt == 'int':\n\
        args.append(torch.randint(0, 8, tuple(shape)))\n\
    else:\n\
        args.append(torch.randn(*shape))\n\
out = kernel.{entry}(*args)\n\
print(json.dumps(list(out.shape)))\n"
    )
}

fn interpret_smoke_output(output: &std::process::Output, expected: &[i64]) -> SmokeReport {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return SmokeReport {
            passed: false,
            actual_shape: None,
            detail: format!("kernel crashed in the sandbox: {}", stderr.trim()),
        };
    }
    let last = stdout.lines().last().unwrap_or("").trim();
    match serde_json::from_str::<Vec<i64>>(last) {
        Ok(actual) => {
            let passed = actual == expected;
            SmokeReport {
                passed,
                actual_shape: Some(actual.clone()),
                detail: if passed {
                    "output shape matches the declared interface".into()
                } else {
                    format!("shape mismatch: kernel produced {actual:?}, descriptor promised {expected:?}")
                },
            }
        }
        Err(_) => SmokeReport {
            passed: false,
            actual_shape: None,
            detail: format!("couldn't read an output shape from the kernel; stdout was: {}", stdout.trim()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_gelu() -> ComponentRegistry {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        let mut r = ComponentRegistry::new();
        r.load_from_dir(&dir).expect("load real components");
        r
    }

    const GOOD: &str = r#"{
        "name": "scaled_tanh",
        "descriptor_edn": "{:component/id \"\" :component/name \"scaled_tanh\" :meta-type \"pure-function\" :version \"1.0.0\" :tags [\"activation\"] :interface/inputs [{:name \"input\" :tensor {:shape [:batch :features] :dtype \"float32\"}}] :interface/outputs [{:name \"output\" :tensor {:shape [:batch :features] :dtype \"float32\"}}] :hyperparameters {} :implementation [{:language \"python\" :entry \"forward\"}] :compatibility {:devices [\"cpu\"] :dtypes [\"float32\"] :autograd true}}",
        "python_code": "import torch\n\ndef forward(input):\n    return torch.tanh(input) * 2.0\n",
        "smoke_test": {"input_shapes": [[2, 4]], "expected_shape": [2, 4]}
    }"#;

    #[test]
    fn repair_request_carries_the_failure_and_prior_output() {
        let msg = build_repair_request("a swish activation", "{\"name\":\"x\"}", "shape mismatch: got [2,3] expected [2,4]");
        assert!(msg.contains("a swish activation"), "must restate the original request");
        assert!(msg.contains("{\"name\":\"x\"}"), "must echo the failed output");
        assert!(msg.contains("shape mismatch"), "must include the failure detail");
    }

    #[test]
    fn accepts_a_well_formed_synthesis() {
        let reg = registry_with_gelu();
        let c = parse_synthesis_output(GOOD, &reg).expect("should accept");
        assert_eq!(c.name, "scaled_tanh");
        assert_eq!(c.descriptor.inputs.len(), 1);
        assert_eq!(c.smoke_test.expected_shape, vec![2, 4]);
    }

    #[test]
    fn rejects_a_name_collision_with_an_existing_component() {
        let reg = registry_with_gelu();
        let collide = GOOD.replace("scaled_tanh", "gelu");
        let err = parse_synthesis_output(&collide, &reg).unwrap_err();
        assert!(err.to_string().contains("already exists"), "{err}");
    }

    #[test]
    fn rejects_a_kernel_missing_its_entry_function() {
        let reg = registry_with_gelu();
        let bad = GOOD.replace("def forward(input):", "def not_forward(input):");
        let err = parse_synthesis_output(&bad, &reg).unwrap_err();
        assert!(err.to_string().contains("def forward"), "{err}");
    }

    #[test]
    fn rejects_a_smoke_test_that_doesnt_cover_every_input() {
        let reg = registry_with_gelu();
        let bad = GOOD.replace(r#""input_shapes": [[2, 4]]"#, r#""input_shapes": []"#);
        let err = parse_synthesis_output(&bad, &reg).unwrap_err();
        assert!(err.to_string().contains("input port"), "{err}");
    }

    #[test]
    fn rejects_a_descriptor_name_mismatch() {
        let reg = registry_with_gelu();
        // Change only the JSON name, leaving the EDN descriptor name as scaled_tanh.
        let bad = GOOD.replacen("\"name\": \"scaled_tanh\"", "\"name\": \"other_name\"", 1);
        let err = parse_synthesis_output(&bad, &reg).unwrap_err();
        assert!(err.to_string().contains("doesn't match"), "{err}");
    }

    #[test]
    fn smoke_driver_calls_the_entry_and_prints_a_shape() {
        let reg = registry_with_gelu();
        let c = parse_synthesis_output(GOOD, &reg).unwrap();
        let driver = build_smoke_driver(&c);
        assert!(driver.contains("kernel.forward(*args)"));
        assert!(driver.contains("print(json.dumps(list(out.shape)))"));
    }

    #[cfg(unix)]
    #[test]
    fn interpret_flags_a_shape_mismatch() {
        use std::os::unix::process::ExitStatusExt;
        let status = std::process::ExitStatus::from_raw(0);
        let output = std::process::Output { status, stdout: b"[3, 4]\n".to_vec(), stderr: Vec::new() };
        let report = interpret_smoke_output(&output, &[2, 4]);
        assert!(!report.passed);
        assert_eq!(report.actual_shape, Some(vec![3, 4]));
    }

    #[cfg(windows)]
    #[test]
    fn interpret_flags_a_shape_mismatch() {
        use std::os::windows::process::ExitStatusExt;
        let status = std::process::ExitStatus::from_raw(0);
        let output = std::process::Output { status, stdout: b"[3, 4]\n".to_vec(), stderr: Vec::new() };
        let report = interpret_smoke_output(&output, &[2, 4]);
        assert!(!report.passed);
        assert_eq!(report.actual_shape, Some(vec![3, 4]));
    }
}
