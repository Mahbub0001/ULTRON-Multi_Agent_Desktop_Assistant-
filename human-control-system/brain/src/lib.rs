//! Brain - Decision Engine with Behavior Tree and WASM Plugin System

pub mod bt;
pub mod wasm;
pub mod proto;
pub mod service;
pub mod config;

pub use bt::{Node, NodeId, NodeType, NodeStatus, NodeConfig, BehaviorTree, Blackboard};
pub use bt::executor::{Executor, ExecutorConfig, ExecutionContext, TickResult};
pub use wasm::{WasmtimeRuntime, WasmConfig, PluginExecutor, PluginMetadata};
pub use service::BrainServiceImpl;
pub use config::BrainConfig;

// Re-export proto types
pub use proto::brain_proto;