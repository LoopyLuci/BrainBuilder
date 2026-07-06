use crate::interop::edn_value::{edn_to_json, json_to_edn};
// `serde::{Serialize, Deserialize}` are intentionally *not* `use`d by name here:
// every struct below derives both serde's and edn_rs's traits, and both define
// an inherent-looking `serialize`/`deserialize` method — if both traits were
// in scope, `x.serialize()` would be ambiguous (E0034). Referencing the derive
// macros by full path below avoids bringing the trait names into scope, so
// only `edn_rs::Serialize`/`Deserialize` (imported below, used for the manual
// impls and the `.serialize()` calls) resolve for method calls.
use edn_rs::{Deserialize as EdnDeserializeTrait, Edn, EdnError, Serialize as EdnSerializeTrait};

/// Bumped whenever a BBIR format change would break parsing an older saved
/// `.bbir.edn` graph without an explicit migration (e.g. a field becoming
/// required, or changing meaning) — not for purely additive, optional
/// changes, which don't need a version bump at all. There's been exactly one
/// format so far, so this is 1; `BBIRGraph::from_edn` treats a graph saved
/// before this field existed (`:schema-version` absent) as version 1 too,
/// since that's the only format that predates it.
pub const CURRENT_BBIR_SCHEMA_VERSION: u32 = 1;

fn current_bbir_schema_version() -> u32 {
    CURRENT_BBIR_SCHEMA_VERSION
}

/// BrainBuilder Intermediate Representation – the canonical graph format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BBIRGraph {
    #[serde(default = "current_bbir_schema_version")]
    pub schema_version: u32,
    pub graph_id: String,
    pub name: String,
    pub nodes: Vec<BBIRNode>,
    pub edges: Vec<BBIREdge>,
    pub training: Option<TrainingConfig>,
}

impl EdnDeserializeTrait for BBIRGraph {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        let schema_version = match &edn[":schema-version"] {
            // A graph saved before this field existed is real BBIR — the
            // only format that ever existed pre-versioning — so it's
            // version 1, not an error and not silently "current" (which
            // would hide a real future migration need once version 2 exists).
            Edn::Nil => 1,
            e => edn_rs::from_edn(e)?,
        };
        Ok(Self {
            schema_version,
            graph_id: edn_rs::from_edn(&edn[":graph-id"])?,
            name: edn_rs::from_edn(&edn[":name"])?,
            nodes: edn_rs::from_edn(&edn[":nodes"])?,
            edges: edn_rs::from_edn(&edn[":edges"])?,
            training: edn_rs::from_edn(&edn[":training"])?,
        })
    }
}

impl EdnSerializeTrait for BBIRGraph {
    fn serialize(&self) -> String {
        format!(
            "{{:schema-version {}, :graph-id {:?}, :name {:?}, :nodes {}, :edges {}, :training {}, }}",
            self.schema_version,
            self.graph_id,
            self.name,
            self.nodes.serialize(),
            self.edges.serialize(),
            self.training
                .as_ref()
                .map_or_else(|| "nil".to_string(), |t| t.serialize()),
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BBIRNode {
    pub id: String,
    pub component: String,
    pub label: Option<String>,
    pub hyperparams: serde_json::Value,
    pub ports: PortInfo,
    /// Canvas position, for round-tripping the GUI's visual layout through
    /// save/load — without this, reloading a saved graph would silently
    /// scatter every node back to a default position.
    #[serde(default)]
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

impl EdnDeserializeTrait for BBIRNode {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        let position = match &edn[":position"] {
            Edn::Nil => None,
            e => Some(NodePosition {
                x: edn_rs::from_edn(&e[":x"])?,
                y: edn_rs::from_edn(&e[":y"])?,
            }),
        };
        Ok(Self {
            id: edn_rs::from_edn(&edn[":id"])?,
            component: edn_rs::from_edn(&edn[":component"])?,
            label: edn_rs::from_edn(&edn[":label"])?,
            hyperparams: edn_to_json(&edn[":hyperparams"]),
            ports: edn_rs::from_edn(&edn[":ports"])?,
            position,
        })
    }
}

impl EdnSerializeTrait for BBIRNode {
    fn serialize(&self) -> String {
        let position = self
            .position
            .map_or_else(|| "nil".to_string(), |p| format!("{{:x {}, :y {}, }}", p.x, p.y));
        format!(
            "{{:id {:?}, :component {:?}, :label {}, :hyperparams {}, :ports {}, :position {}, }}",
            self.id,
            self.component,
            self.label
                .as_ref()
                .map_or_else(|| "nil".to_string(), |l| format!("{l:?}")),
            json_to_edn(&self.hyperparams).to_string(),
            self.ports.serialize(),
            position,
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, edn_derive::Serialize, edn_derive::Deserialize)]
pub struct PortInfo {
    pub input_ports: Vec<String>,
    pub output_ports: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, edn_derive::Serialize, edn_derive::Deserialize)]
pub struct BBIREdge {
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingConfig {
    pub loss: String,
    pub optimizer: String,
    pub trainer_type: String,
    pub hyperparams: serde_json::Value,
    pub data_source: DataSourceConfig,
    pub reproducibility: Option<ReproducibilityConfig>,
}

impl EdnDeserializeTrait for TrainingConfig {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            loss: edn_rs::from_edn(&edn[":loss"])?,
            optimizer: edn_rs::from_edn(&edn[":optimizer"])?,
            trainer_type: edn_rs::from_edn(&edn[":trainer_type"])?,
            hyperparams: edn_to_json(&edn[":hyperparams"]),
            data_source: edn_rs::from_edn(&edn[":data_source"])?,
            reproducibility: edn_rs::from_edn(&edn[":reproducibility"])?,
        })
    }
}

impl EdnSerializeTrait for TrainingConfig {
    fn serialize(&self) -> String {
        format!(
            "{{:loss {:?}, :optimizer {:?}, :trainer_type {:?}, :hyperparams {}, :data_source {}, :reproducibility {}, }}",
            self.loss,
            self.optimizer,
            self.trainer_type,
            json_to_edn(&self.hyperparams),
            self.data_source.serialize(),
            self.reproducibility
                .as_ref()
                .map_or_else(|| "nil".to_string(), |r| r.serialize()),
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, edn_derive::Serialize, edn_derive::Deserialize)]
pub struct DataSourceConfig {
    pub source_type: String,
    pub path_or_uri: String,
    pub batch_size: usize,
    pub preprocessing: Vec<PreprocStep>,
    /// Only meaningful when `source_type == "text_sequence"`: window length
    /// for next-token-prediction training (see `data::text`). `None` for
    /// every other source type.
    #[serde(default)]
    pub sequence_length: Option<usize>,
    /// Only meaningful when `source_type == "text_sequence"`: caps the
    /// word-level vocabulary built from the text file (most-frequent words
    /// win; anything else maps to the reserved `<unk>` id 0). Must match the
    /// `vocab_size` hyperparameter on the graph's `embedding` node — the
    /// engine doesn't yet auto-wire dataset vocab size into a node's
    /// hyperparameters, so this is a real, documented manual-coordination
    /// step, not a silent gap.
    #[serde(default)]
    pub vocab_size: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreprocStep {
    pub op: String,
    pub params: serde_json::Value,
}

impl EdnDeserializeTrait for PreprocStep {
    fn deserialize(edn: &Edn) -> Result<Self, EdnError> {
        Ok(Self {
            op: edn_rs::from_edn(&edn[":op"])?,
            params: edn_to_json(&edn[":params"]),
        })
    }
}

impl EdnSerializeTrait for PreprocStep {
    fn serialize(&self) -> String {
        format!(
            "{{:op {:?}, :params {}, }}",
            self.op,
            json_to_edn(&self.params),
        )
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, edn_derive::Serialize, edn_derive::Deserialize)]
pub struct ReproducibilityConfig {
    pub nix_expression: Option<String>,
    pub seed: Option<u64>,
}

impl BBIRGraph {
    /// Parse from an EDN string.
    pub fn from_edn(edn: &str) -> crate::Result<Self> {
        edn_rs::from_str(edn)
            .map_err(|e| crate::interop::protocol::BrainBuilderError::Parse(format!("{e:?}")))
    }

    /// Serialize to pretty-printed EDN (for display / diff).
    pub fn to_edn(&self) -> crate::Result<String> {
        Ok(edn_rs::to_string(self))
    }
}
