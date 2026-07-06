// Bridge between `edn_rs::Edn` and `serde_json::Value` for the small number of
// fields (hyperparameter defaults, test fixtures, symbolic shapes) that are
// intentionally free-form rather than typed BBIR/component-descriptor fields.
use edn_rs::edn::{Map as EdnMap, Vector as EdnVector};
use edn_rs::Edn;
use std::collections::BTreeMap;

/// Convert a leaf `Edn` value into a `serde_json::Value`, for fields whose
/// EDN shape is not fixed ahead of time (hyperparameter defaults/constraints,
/// test fixtures, symbolic tensor-shape expressions).
pub fn edn_to_json(edn: &Edn) -> serde_json::Value {
    serde_json::from_str(&edn.to_json()).unwrap_or(serde_json::Value::Null)
}

/// Convert a `serde_json::Value` back into `Edn`, the inverse of
/// [`edn_to_json`]. Hand-written rather than routed through
/// `edn_rs::json_to_edn` (that function is genuinely broken for numeric
/// values: `{"lr": 0.01, "epochs": 30}` comes out as `{:epochs30,:lr0.01}` —
/// key and value glued together with no separator, verified directly against
/// edn-rs 0.18's real behavior, not a usage mistake on our part).
pub fn json_to_edn(value: &serde_json::Value) -> Edn {
    match value {
        serde_json::Value::Null => Edn::Nil,
        serde_json::Value::Bool(b) => Edn::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Edn::Int(i)
            } else {
                Edn::Double(n.as_f64().unwrap_or(0.0).into())
            }
        }
        serde_json::Value::String(s) => Edn::Str(s.clone()),
        serde_json::Value::Array(items) => {
            Edn::Vector(EdnVector::new(items.iter().map(json_to_edn).collect()))
        }
        serde_json::Value::Object(map) => {
            let entries: BTreeMap<String, Edn> = map
                .iter()
                .map(|(k, v)| (format!(":{k}"), json_to_edn(v)))
                .collect();
            Edn::Map(EdnMap::new(entries))
        }
    }
}
