use std::collections::HashMap;
use std::fs;
use std::path::Path;
use crate::Result;
use crate::interop::protocol::BrainBuilderError;
use super::descriptor::ComponentDescriptor;

/// In-memory registry of all available components. Backed by disk.
pub struct ComponentRegistry {
    by_hash: HashMap<String, ComponentDescriptor>,
    by_name: HashMap<String, String>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            by_hash: HashMap::new(),
            by_name: HashMap::new(),
        }
    }

    /// Load every `.edn` file in the given directory.
    pub fn load_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Err(BrainBuilderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Components directory missing: {}", dir.display()),
            )));
        }
        let mut loaded = 0;
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().map_or(false, |e| e == "edn") {
                let content = fs::read_to_string(&path)?;
                let desc: ComponentDescriptor = edn_rs::from_str(&content)
                    .map_err(|e| BrainBuilderError::Parse(format!("{}: {:?}", path.display(), e)))?;
                log::debug!("loaded component `{}` from {}", desc.name, path.display());
                self.insert(desc);
                loaded += 1;
            }
        }
        log::info!("loaded {loaded} component(s) from {}", dir.display());
        Ok(())
    }

    /// Insert a descriptor, returning its content-addressed hash.
    pub fn insert(&mut self, mut desc: ComponentDescriptor) -> String {
        if desc.id.is_empty() || desc.id.len() != 64 {
            let canonical = serde_json::to_string(&desc).unwrap();
            desc.id = blake3::hash(canonical.as_bytes()).to_hex().to_string();
        }
        let hash = desc.id.clone();
        self.by_name.insert(desc.name.clone(), hash.clone());
        self.by_hash.insert(hash.clone(), desc);
        hash
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ComponentDescriptor> {
        self.by_name.get(name).and_then(|h| self.by_hash.get(h))
    }

    pub fn get_by_hash(&self, hash: &str) -> Option<&ComponentDescriptor> {
        self.by_hash.get(hash)
    }

    pub fn list_names(&self) -> Vec<String> {
        self.by_name.keys().cloned().collect()
    }

    /// Clean, frontend-facing summaries of every registered component
    /// (ports with roles + hyperparameter schema), sorted by name for stable
    /// palette ordering.
    pub fn summaries(&self) -> Vec<super::descriptor::ComponentSummary> {
        let mut summaries: Vec<_> = self.by_name.values().filter_map(|hash| {
            self.by_hash.get(hash).map(|d| d.summary())
        }).collect();
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
