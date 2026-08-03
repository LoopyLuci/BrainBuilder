//! Tool subsystem: schemas, registry, and concrete implementations.

pub mod schemas;
pub mod registry;
pub mod implementations;

pub use schemas::get_all_tools;
pub use registry::ToolResult;
pub use implementations::{PlatformHost, MockPlatformHost, ToolExecutor};
