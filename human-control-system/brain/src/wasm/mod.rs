//! WASM Plugin System

pub mod runtime;
pub mod plugin_trait;
pub mod host_functions;

pub use runtime::{WasmtimeRuntime, WasmConfig};
pub use plugin_trait::{PluginExecutor, PluginMetadata, PluginInterface};
pub use host_functions::HostFunctions;