use crate::interop::dlpack_support::alloc_managed_tensor;
use crate::TensorHandle;
use std::sync::{Arc, Mutex};

/// Shared memory arena for zero-copy tensor exchange. All runtimes use this.
pub struct SharedArena {
    tensors: Mutex<Vec<Arc<TensorHandle>>>,
}

impl SharedArena {
    pub fn new() -> Self {
        Self {
            tensors: Mutex::new(Vec::new()),
        }
    }

    pub fn allocate(
        &self,
        shape: &[i64],
        dtype: dlpack::DataType,
        ctx: dlpack::Context,
    ) -> Arc<TensorHandle> {
        let tensor_ptr = alloc_managed_tensor(shape, dtype, ctx);
        let arc = Arc::new(TensorHandle(tensor_ptr));
        // Recover from a poisoned lock (another thread panicked while holding
        // it) rather than propagating the panic here too — the arena's
        // bookkeeping Vec is still structurally valid, just possibly missing
        // an entry from whatever panicked mid-push.
        self.tensors
            .lock()
            .unwrap_or_else(|poisoned| {
                log::warn!("SharedArena tensor list lock was poisoned — recovering");
                poisoned.into_inner()
            })
            .push(arc.clone());
        arc
    }
}

impl Default for SharedArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod chaos_tests {
    //! Regression test for the poisoned-lock recovery in `allocate`: before
    //! the fix, any thread panicking while holding `tensors`'s lock poisoned
    //! it permanently, and every subsequent `allocate` call on the same
    //! arena would panic too (`.lock().unwrap()`). Now it recovers via
    //! `unwrap_or_else(|poisoned| poisoned.into_inner())`.
    use super::*;
    use std::panic::{self, AssertUnwindSafe};

    #[test]
    fn allocate_recovers_after_the_tensor_list_lock_is_poisoned() {
        let arena = Arc::new(SharedArena::new());

        // Poison the lock: hold it, then panic while it's held.
        let poison_arena = arena.clone();
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let _guard = poison_arena.tensors.lock().unwrap();
            panic!("deliberately poisoning the tensor list lock for the regression test");
        }));
        assert!(result.is_err(), "the poisoning panic should have been caught");
        assert!(
            arena.tensors.is_poisoned(),
            "the lock should now be reported as poisoned"
        );

        // Before the fix, this next call would itself panic (`.lock().unwrap()`
        // on an already-poisoned mutex). Now it must recover and succeed.
        let handle = arena.allocate(
            &[2, 2],
            dlpack::DataType { code: dlpack::data_type_codes::FLOAT, bits: 32, lanes: 1 },
            crate::interop::dlpack_support::cpu_context(),
        );
        assert!(!handle.0.is_null(), "allocate should have returned a real tensor handle");
    }
}
