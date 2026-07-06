//! The Sandboxing Nervous System: a capability-based process fabric that all
//! component execution (local subprocess, and eventually remote peers) flows
//! through. Real capability enforcement (deny-by-default filesystem access,
//! a hard wall-clock timeout, a memory ceiling) for every subprocess-based
//! runtime: Racket and Clojure via `Supervisor::spawn_checked`, and Python via
//! `PythonBridge`'s persistent worker process (`interop/python.rs`) — the
//! former in-process pyo3 embedding (full host-process privileges for
//! arbitrary component code) has been fully migrated off. Tensors cross the
//! Python worker boundary via `memmap2`-backed shared memory files (both
//! sides `mmap` the same file rather than copying it through a `read`/
//! `write` syscall pair) — see `PythonBridge::{read_tensor,write_tensor}`
//! and the worker's own `mmap`-based counterparts in `_bb_worker.py`. Every
//! invocation across all three runtimes is recorded to `audit` (the
//! observability tap), queryable from the GUI's Console panel. Remaining
//! documented gap: OS-level process containment (Windows Job Objects, real;
//! Linux/macOS: `RLIMIT_AS` + process-group kill via `job_object.rs`'s Unix
//! path — written but unverified in this all-Windows dev environment,
//! cgroups/seccomp/App-Sandbox hardening beyond that is a real follow-up).
pub mod audit;
pub mod capability;
pub mod job_object;
pub mod runtime;
pub mod supervisor;
pub mod system_resources;

pub use audit::{AuditEvent, AuditRecord};
pub use capability::Capabilities;
pub use job_object::JobObject;
pub use runtime::ComponentRuntime;
pub use supervisor::Supervisor;
