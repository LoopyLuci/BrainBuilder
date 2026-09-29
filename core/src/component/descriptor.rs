use crate::interop::edn_value::edn_to_json;
use edn_rs::{Deserialize as EdnDeserialize, Edn, EdnError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    #[serde(rename = "component/id")]
    pub id: String,
    #[serde(rename = "component/name")]
    pub name: String,
    #[serde(rename = "meta-type", default)]
    pub meta_type: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(rename = "version", default)]
    pub version: String,
    #[serde(rename = "interface/inputs", default)]
    pub inputs: Vec<PortSpec>,
    #[serde(rename = "interface/outputs", default)]
    pub outputs: Vec<PortSpec>,
    #[serde(rename = "hyperparameters", default)]
    pub hyperparameters: HashMap<String, HyperParameterDef>,
    #[serde(rename = "implementation", default)]
    pub implementations: Vec<Implementation>,
    #[serde(rename = "compatibility", default)]
    pub compatibility: Compatibility,
    #[serde(default)]
    pub tests: Vec<ComponentTest>,
}

impl EdnDeserialize for ComponentDescriptor {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            id: edn_rs::from_edn(&edn[":component/id"])?,
            name: edn_rs::from_edn(&edn[":component/name"])?,
            meta_type: opt_string(edn, ":meta-type"),
            tags: opt_vec(edn, ":tags")?,
            version: opt_string(edn, ":version"),
            inputs: opt_vec(edn, ":interface/inputs")?,
            outputs: opt_vec(edn, ":interface/outputs")?,
            hyperparameters: opt_map(edn, ":hyperparameters")?,
            implementations: opt_vec(edn, ":implementation")?,
            compatibility: match &edn[":compatibility"] {
                Edn::Nil => Compatibility::default(),
                e => edn_rs::from_edn(e)?,
            },
            tests: opt_vec(edn, ":tests")?,
        })
    }
}

fn opt_string(edn: &Edn, key: &str) -> String {
    match &edn[key] {
        Edn::Nil => String::new(),
        e => edn_rs::from_edn(e).unwrap_or_default(),
    }
}

fn opt_vec<T: EdnDeserialize>(edn: &Edn, key: &str) -> Result<Vec<T>, EdnError> {
    match &edn[key] {
        Edn::Nil => Ok(Vec::new()),
        e => edn_rs::from_edn(e),
    }
}

fn opt_map<T: EdnDeserialize>(edn: &Edn, key: &str) -> Result<HashMap<String, T>, EdnError> {
    match &edn[key] {
        Edn::Nil => Ok(HashMap::new()),
        e => edn_rs::from_edn(e),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSpec {
    pub name: String,
    pub tensor: TensorSpec,
    /// Whether this port carries wired **data** (connected by an edge, or fed
    /// from the dataset) or is a learnable **parameter** the runtime manages
    /// (e.g. `linear`'s `weight`, `adam`'s `params`). Parameter ports are left
    /// unconnected in the graph and auto-initialized by the trainer. Defaults
    /// to `Data` when `:role` is omitted in the EDN descriptor.
    #[serde(default)]
    pub role: PortRole,
    /// For a `Parameter` port only: whether the optimizer updates it during
    /// training. `false` marks a **frozen** parameter — still auto-initialized
    /// and passed into every forward call, but excluded from the trainable
    /// set. This is what LoRA fine-tuning needs: `lora_linear`'s base `weight`
    /// is frozen (`:trainable false`) while its low-rank `lora_a`/`lora_b`
    /// adapters are the only parameters actually optimized. Defaults to
    /// `true`; meaningless (and ignored) on a `Data` port.
    #[serde(default = "default_trainable")]
    pub trainable: bool,
    /// For a `Parameter` port only: initialize with zeros instead of
    /// `torch.randn`. LoRA's `lora_b` needs this — standard practice zeroes
    /// the second low-rank factor so the adapter starts as a true no-op on
    /// the frozen base weight (`lora_a` stays randomly initialized, matching
    /// the reference LoRA paper). Defaults to `false`.
    #[serde(default)]
    pub zero_init: bool,
}

fn default_trainable() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PortRole {
    #[default]
    Data,
    Parameter,
}

impl PortRole {
    fn from_edn_str(s: &str) -> Self {
        match s {
            "parameter" | "param" => Self::Parameter,
            _ => Self::Data,
        }
    }
}

impl EdnDeserialize for PortSpec {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        let role = match &edn[":role"] {
            Edn::Nil => PortRole::Data,
            e => PortRole::from_edn_str(&edn_rs::from_edn::<String>(e)?),
        };
        let trainable = match &edn[":trainable"] {
            Edn::Nil => true,
            e => edn_rs::from_edn(e)?,
        };
        let zero_init = match &edn[":zero-init"] {
            Edn::Nil => false,
            e => edn_rs::from_edn(e)?,
        };
        Ok(Self {
            name: edn_rs::from_edn(&edn[":name"])?,
            tensor: edn_rs::from_edn(&edn[":tensor"])?,
            role,
            trainable,
            zero_init,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorSpec {
    pub shape: ShapeExpr,
    pub dtype: DataType,
    #[serde(default)]
    pub sparsity: Option<String>,
}

impl EdnDeserialize for TensorSpec {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            shape: edn_rs::from_edn(&edn[":shape"])?,
            dtype: edn_rs::from_edn(&edn[":dtype"])?,
            sparsity: edn_rs::from_edn(&edn[":sparsity"])?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ShapeExpr {
    Fixed(Vec<isize>),
    Symbolic(serde_json::Value),
}

impl EdnDeserialize for ShapeExpr {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        // Shapes are either all-integer (`[1 16 30 30]`) or contain symbolic
        // dimensions/expressions (`[:batch :out-ch (:- :h :kh 1)]`).
        if let Ok(fixed) = edn_rs::from_edn::<Vec<isize>>(edn) {
            return Ok(Self::Fixed(fixed));
        }
        Ok(Self::Symbolic(edn_to_json(edn)))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    Float32,
    Float16,
    Int32,
    Int64,
    Bool,
}

impl EdnDeserialize for DataType {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        let s: String = edn_rs::from_edn(edn)?;
        match s.as_str() {
            "float32" => Ok(Self::Float32),
            "float16" => Ok(Self::Float16),
            "int32" => Ok(Self::Int32),
            "int64" => Ok(Self::Int64),
            "bool" => Ok(Self::Bool),
            other => Err(EdnError::Deserialize(format!(
                "couldn't convert `{other}` into a DataType"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperParameterDef {
    #[serde(rename = "type")]
    pub param_type: String,
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default)]
    pub constraints: Option<serde_json::Value>,
}

impl EdnDeserialize for HyperParameterDef {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            param_type: edn_rs::from_edn(&edn[":type"])?,
            default: edn_to_json(&edn[":default"]),
            constraints: match &edn[":constraints"] {
                Edn::Nil => None,
                e => Some(edn_to_json(e)),
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub language: String,
    #[serde(default)]
    pub path: Option<String>,
    pub entry: String,
}

impl EdnDeserialize for Implementation {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            language: edn_rs::from_edn(&edn[":language"])?,
            path: edn_rs::from_edn(&edn[":path"])?,
            entry: edn_rs::from_edn(&edn[":entry"])?,
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Compatibility {
    #[serde(default)]
    pub devices: Vec<String>,
    #[serde(default)]
    pub dtypes: Vec<DataType>,
    #[serde(default)]
    pub autograd: bool,
}

impl EdnDeserialize for Compatibility {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            devices: opt_vec(edn, ":devices")?,
            dtypes: opt_vec(edn, ":dtypes")?,
            autograd: match &edn[":autograd"] {
                Edn::Nil => false,
                e => edn_rs::from_edn(e)?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTest {
    pub inputs: serde_json::Value,
    #[serde(default)]
    pub expected_shape: Option<ShapeExpr>,
}

impl EdnDeserialize for ComponentTest {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            inputs: edn_to_json(&edn[":inputs"]),
            expected_shape: match &edn[":expected-shape"] {
                Edn::Nil => None,
                e => Some(EdnDeserialize::deserialize(e)?),
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Frontend-facing summary DTO.
//
// `ComponentDescriptor` serializes with EDN-flavored serde renames
// (`component/name`, `interface/inputs`, ...), which are awkward to consume in
// TypeScript. `ComponentSummary` is the clean, camelCase-friendly projection
// the GUI actually needs to render the palette, wire ports, and edit
// hyperparameters — decoupling the frontend from the on-disk EDN key naming.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ComponentSummary {
    pub name: String,
    pub meta_type: String,
    pub inputs: Vec<PortSummary>,
    pub outputs: Vec<PortSummary>,
    pub hyperparameters: Vec<HyperParamSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortSummary {
    pub name: String,
    /// "data" or "parameter" — the GUI only renders connectable handles for
    /// data ports; parameter ports are managed by the runtime.
    pub role: String,
    pub dtype: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HyperParamSummary {
    pub name: String,
    pub param_type: String,
    pub default: serde_json::Value,
}

impl ComponentDescriptor {
    /// The port names on this component that are learnable parameters
    /// (role = Parameter) rather than wired data. Used by the scheduler to
    /// decide which inputs to auto-initialize vs. feed from edges/dataset.
    pub fn parameter_ports(&self) -> Vec<String> {
        self.inputs
            .iter()
            .filter(|p| p.role == PortRole::Parameter)
            .map(|p| p.name.clone())
            .collect()
    }

    /// Subset of `parameter_ports()` the optimizer actually updates — excludes
    /// ports marked `:trainable false` (frozen base weights, e.g.
    /// `lora_linear`'s `weight`).
    pub fn trainable_parameter_ports(&self) -> Vec<String> {
        self.inputs
            .iter()
            .filter(|p| p.role == PortRole::Parameter && p.trainable)
            .map(|p| p.name.clone())
            .collect()
    }

    /// Parameter ports that should be freshly initialized with zeros instead
    /// of `torch.randn` (e.g. LoRA's `lora_b`).
    pub fn zero_init_parameter_ports(&self) -> Vec<String> {
        self.inputs
            .iter()
            .filter(|p| p.role == PortRole::Parameter && p.zero_init)
            .map(|p| p.name.clone())
            .collect()
    }

    pub fn summary(&self) -> ComponentSummary {
        let port = |p: &PortSpec| PortSummary {
            name: p.name.clone(),
            role: match p.role {
                PortRole::Data => "data".to_string(),
                PortRole::Parameter => "parameter".to_string(),
            },
            dtype: format!("{:?}", p.tensor.dtype).to_lowercase(),
        };
        ComponentSummary {
            name: self.name.clone(),
            meta_type: self.meta_type.clone(),
            inputs: self.inputs.iter().map(port).collect(),
            outputs: self.outputs.iter().map(port).collect(),
            hyperparameters: self
                .hyperparameters
                .iter()
                .map(|(name, def)| HyperParamSummary {
                    name: name.clone(),
                    param_type: def.param_type.clone(),
                    default: def.default.clone(),
                })
                .collect(),
        }
    }
}
