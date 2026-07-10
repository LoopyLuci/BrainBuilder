use std::sync::Arc;
use crate::bbir::BBIRGraph;
use crate::component::registry::ComponentRegistry;
use crate::component::validation::validate_graph;
use crate::runtime::scheduler::{self, ExecutionPlan};
use crate::runtime::trainer::TrainerController;
use crate::interop::arena::SharedArena;
use crate::interop::python::PythonBridge;
use crate::data::source::DataIterator;
use crate::Result;
use crate::AppContext;

/// The central orchestrator that receives BBIR graphs from the GUI
/// and manages the entire execution lifecycle.
pub struct Orchestrator {
    pub context: Arc<AppContext>,
    trainer_controller: TrainerController,
    python: PythonBridge,
    /// User-selected GPU for native (`rust`) component ops, by adapter-name
    /// substring (e.g. "7900 XTX"); `None` = auto/CPU. Set from the GUI's GPU
    /// picker and consumed when running a graph forward (see `predict`).
    preferred_gpu: std::sync::Mutex<Option<String>>,
}

impl Orchestrator {
    pub fn new(components_dir: &std::path::Path) -> Result<Self> {
        let arena = Arc::new(SharedArena::new());
        let mut registry = ComponentRegistry::new();
        registry.load_from_dir(components_dir)?;

        let project_root = components_dir.parent().unwrap_or(components_dir);
        let provenance_path = project_root.join("provenance.sqlite3");
        let provenance = crate::utils::provenance::ProvenanceStore::open(&provenance_path)?;

        let experiments_path = project_root.join("experiments.sqlite3");
        let experiments = crate::utils::experiment_log::ExperimentLog::open(&experiments_path)?;

        let checkpoints_dir = project_root.join("checkpoints");
        std::fs::create_dir_all(&checkpoints_dir)?;

        let context = Arc::new(AppContext {
            registry: std::sync::RwLock::new(registry),
            arena: arena.clone(),
            provenance,
            experiments,
            checkpoints_dir,
        });

        let python = PythonBridge::new(context.arena.clone())?;
        let trainer_controller = TrainerController::new(context.clone())?;

        Ok(Self {
            context,
            trainer_controller,
            python,
            preferred_gpu: std::sync::Mutex::new(None),
        })
    }

    /// Set (or clear, with `None`/empty) the preferred GPU for native ops. A
    /// name that doesn't match any adapter degrades to CPU at resolve time, so
    /// this never fails.
    pub fn set_preferred_gpu(&self, name: Option<String>) {
        let cleaned = name.and_then(|n| {
            let t = n.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        });
        match self.preferred_gpu.lock() {
            Ok(mut guard) => *guard = cleaned.clone(),
            Err(_) => {
                log::warn!("preferred_gpu lock poisoned — GPU preference not updated");
                return;
            }
        }

        // Extend the same choice to the torch training path. wgpu adapters are
        // named (e.g. "Radeon RX 7900 XTX") but torch selects by device *type*,
        // so a picked GPU maps to torch "cuda" (the API namespace ROCm/AMD and
        // NVIDIA both use); clearing maps to auto-detect. The worker validates
        // and falls back to CPU if that device isn't actually usable, so this
        // never wedges training.
        let torch_device = cleaned.map(|_| "cuda".to_string());
        if let Err(e) = self.python.set_device(torch_device) {
            log::warn!("couldn't apply torch device preference: {e}");
        }
    }

    /// Structural + shape validation only, no execution — lets the GUI catch
    /// port/dtype/shape errors (see `component::validation::validate_graph`)
    /// the instant a graph is edited, without waiting on data loading or a
    /// Python worker round trip.
    pub fn validate(&self, graph: &BBIRGraph) -> Result<()> {
        let registry = self.context.registry.read().map_err(|_| {
            crate::interop::protocol::BrainBuilderError::ConfigError("registry lock poisoned".into())
        })?;
        validate_graph(graph, &registry)
    }

    /// Validate and execute a BBIR graph. Called from a Tauri command.
    pub async fn execute_graph(&self, graph: BBIRGraph) -> Result<()> {
        log::debug!(
            "validating graph `{}`: {} nodes, {} edges",
            graph.name,
            graph.nodes.len(),
            graph.edges.len()
        );
        self.validate(&graph)?;

        if let Some(repro) = graph.training.as_ref().and_then(|t| t.reproducibility.as_ref()) {
            if repro.nix_expression.is_none() {
                log::info!(
                    "no nix_expression pinned for graph `{}` — captured environment:\n{}",
                    graph.name,
                    crate::utils::nix::capture_nix_environment()
                );
            }
        }

        let plan = self.compile(&graph)?;
        let trainer = self.trainer_controller.get_trainer(&graph.training)?;
        let mut data = self.load_data(&graph.training).await?;

        trainer.fit(plan, data.as_mut()).await
    }

    fn compile(&self, graph: &BBIRGraph) -> Result<ExecutionPlan> {
        scheduler::compile(graph, &self.context)
    }

    /// Run a compiled graph forward only (no loss/backward/optimizer step),
    /// loading the graph's trained-weight checkpoint if one exists (saved
    /// automatically at the end of `execute_graph`'s training run). Errors
    /// clearly (via `ExecutionPlan::forward`) if the graph has parameter
    /// ports but no checkpoint has been trained yet.
    pub fn predict(&self, graph: BBIRGraph, batch: arrow::record_batch::RecordBatch) -> Result<Vec<crate::Tensor>> {
        self.validate(&graph)?;
        let plan = self.compile(&graph)?;
        let inputs = plan.prepare_inputs(batch)?;
        let checkpoint_path = self.context.checkpoint_path(&graph.graph_id);
        let weights = if checkpoint_path.exists() {
            self.python.load_state_dict(&checkpoint_path)?
        } else {
            std::collections::HashMap::new()
        };
        // Run native (`rust`) ops on the user's preferred GPU when one is set +
        // available, else CPU. Python/torch components use torch's own device.
        let preferred = self.preferred_gpu.lock().map_err(|_| {
            crate::interop::protocol::BrainBuilderError::ConfigError("preferred_gpu lock poisoned".into())
        })?.clone();
        let device = crate::runtime::device_select::resolve_device(preferred.as_deref());
        plan.forward(&inputs, &weights, &self.python, device.as_ref())
    }

    pub fn has_checkpoint(&self, graph_id: &str) -> bool {
        self.context.checkpoint_path(graph_id).exists()
    }

    async fn load_data(&self, train_cfg: &Option<crate::bbir::TrainingConfig>) -> Result<Box<dyn DataIterator>> {
        match train_cfg {
            Some(cfg) => crate::data::source::load_dataset(cfg).await,
            None => Err(crate::interop::protocol::BrainBuilderError::ConfigError(
                "No training config".into(),
            )),
        }
    }

    /// Install a component from a local file path (called via a Tauri command).
    pub fn install_component(&self, path: &str) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let desc: crate::component::descriptor::ComponentDescriptor = edn_rs::from_str(&content)
            .map_err(|e| crate::interop::protocol::BrainBuilderError::Parse(format!("{e:?}")))?;
        let mut registry = self.context.registry.write().map_err(|_| {
            crate::interop::protocol::BrainBuilderError::ConfigError("registry lock poisoned".into())
        })?;
        let hash = registry.insert(desc);
        log::info!("installed component `{path}` as {hash}");
        Ok(())
    }

    pub fn list_components(&self) -> Result<Vec<String>> {
        let registry = self.context.registry.read().map_err(|_| {
            crate::interop::protocol::BrainBuilderError::ConfigError("registry lock poisoned".into())
        })?;
        Ok(registry.list_names())
    }

    /// Full component summaries (ports + roles + hyperparameter schema) for the
    /// GUI palette / inspector / graph-conversion. Replaces `list_components`
    /// as the source of truth for building valid graphs.
    pub fn component_summaries(
        &self,
    ) -> Result<Vec<crate::component::descriptor::ComponentSummary>> {
        let registry = self.context.registry.read().map_err(|_| {
            crate::interop::protocol::BrainBuilderError::ConfigError("registry lock poisoned".into())
        })?;
        Ok(registry.summaries())
    }
}

#[cfg(test)]
mod chaos_tests {
    //! Regression test for the `preferred_gpu` poisoned-lock recovery.
    //! Needs a real `Orchestrator` (spawns the python worker), same as every
    //! other orchestrator-level integration test in `core/tests/` — not
    //! `#[ignore]`d because this crate's existing convention only ignores
    //! tests needing toolchains beyond python+torch (racket, clojure).
    use super::*;
    use std::panic::{self, AssertUnwindSafe};

    #[test]
    fn set_preferred_gpu_recovers_after_the_lock_is_poisoned() {
        let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        std::env::set_var("PYTHONPATH", components_dir.join("python"));
        let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");

        // Poison `preferred_gpu`'s lock: hold it, then panic while held.
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = orchestrator.preferred_gpu.lock().unwrap();
            panic!("deliberately poisoning preferred_gpu's lock for the regression test");
        }));
        assert!(result.is_err(), "the poisoning panic should have been caught");
        assert!(orchestrator.preferred_gpu.is_poisoned(), "lock should now report poisoned");

        // Before the fix, `set_preferred_gpu`'s `match self.preferred_gpu.lock()`
        // already handled the `Err` branch without panicking (it logs and
        // returns) — this asserts that behavior still holds and doesn't
        // regress into a panic.
        orchestrator.set_preferred_gpu(Some("test-gpu".to_string()));

        // `predict`'s lock use (`.map_err(...)`) must likewise return a clean
        // `Err`, not panic, when the lock is poisoned. A minimal empty graph
        // is enough to reach the lock before any real component execution.
        let graph = crate::bbir::BBIRGraph {
            schema_version: 1,
            graph_id: "chaos-test-graph".into(),
            name: "chaos-test-graph".into(),
            nodes: vec![],
            edges: vec![],
            training: None,
        };
        let batch = arrow::record_batch::RecordBatch::new_empty(std::sync::Arc::new(arrow::datatypes::Schema::empty()));
        let result = orchestrator.predict(graph, batch);
        assert!(result.is_err(), "predict should return a clean Err, not panic, past the poisoned lock");
    }
}
