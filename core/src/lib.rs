pub mod orchestrator;
pub mod bbir;
pub mod cluster;
pub mod component;
pub mod runtime;
pub mod interop;
pub mod data;
pub mod utils;
pub mod models;
pub mod llm;

use std::sync::Arc;

/// Owns a heap-allocated `dlpack::ManagedTensor` (a `#[repr(C)]` binding to
/// the raw DLPack ABI) by pointer, and calls its `deleter` on drop.
///
/// Storing a pointer rather than the struct by value matters once tensors
/// can be *imported* from a foreign producer (e.g. PyTorch, via
/// `interop::python`): DLPack's contract is that `deleter(self)` frees the
/// entire `ManagedTensor` allocation, including its own backing memory — so
/// the deleter must be called against the exact address the producer
/// allocated, never a Rust-side copy of the struct's bytes. For tensors we
/// allocate ourselves (`interop::dlpack_support`), the deleter frees a `Box`
/// we created; for imported ones, it's the foreign allocator's own free
/// function, called through the function pointer embedded in the struct.
///
/// The pointer is exclusively owned by this handle and never aliased
/// outside of it, so it's sound to mark it Send+Sync and share it as
/// `Arc<TensorHandle>` across the async runtime.
pub struct TensorHandle(pub *mut dlpack::ManagedTensor);

unsafe impl Send for TensorHandle {}
unsafe impl Sync for TensorHandle {}

impl Drop for TensorHandle {
    fn drop(&mut self) {
        unsafe { ((*self.0).deleter)(self.0) };
    }
}

/// Opaque tensor handle – zero-copy DLPack pointer inside Arc.
pub type Tensor = Arc<TensorHandle>;

/// Result alias for the whole crate.
pub type Result<T> = std::result::Result<T, interop::protocol::BrainBuilderError>;

/// Application state shared across modules (not used as global, passed where needed).
pub struct AppContext {
    pub registry: std::sync::RwLock<component::registry::ComponentRegistry>,
    pub arena: Arc<interop::arena::SharedArena>,
    pub provenance: utils::provenance::ProvenanceStore,
    /// Directory where trained-weight checkpoints (`<graph_id>.pt`) are saved
    /// after training and loaded for inference.
    pub checkpoints_dir: std::path::PathBuf,
}

impl AppContext {
    pub fn checkpoint_path(&self, graph_id: &str) -> std::path::PathBuf {
        self.checkpoints_dir.join(format!("{graph_id}.pt"))
    }
}
