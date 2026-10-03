//! Configuration for Brain Service

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BrainConfig {
    pub server: ServerConfig,
    pub behavior_tree: BehaviorTreeConfig,
    pub wasm: WasmConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_concurrent_trees: usize,
    pub tick_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BehaviorTreeConfig {
    pub max_depth: usize,
    pub max_nodes: usize,
    pub default_tick_rate_hz: u32,
    pub enable_visualization: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WasmConfig {
    pub plugin_dir: PathBuf,
    pub max_memory_mb: u32,
    pub max_fuel_per_tick: u64,
    pub fuel_per_instruction: u64,
    pub enable_hot_reload: bool,
    pub hot_reload_interval_ms: u64,
    pub allowed_host_functions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub port: u16,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 50053,
                max_concurrent_trees: 100,
                tick_timeout_ms: 1000,
            },
            behavior_tree: BehaviorTreeConfig {
                max_depth: 100,
                max_nodes: 10000,
                default_tick_rate_hz: 60,
                enable_visualization: true,
            },
            wasm: WasmConfig {
                plugin_dir: PathBuf::from("plugins"),
                max_memory_mb: 64,
                max_fuel_per_tick: 1_000_000,
                fuel_per_instruction: 1,
                enable_hot_reload: true,
                hot_reload_interval_ms: 5000,
                allowed_host_functions: vec![
                    "blackboard_get".to_string(),
                    "blackboard_set".to_string(),
                    "blackboard_has".to_string(),
                    "log_info".to_string(),
                    "log_warn".to_string(),
                    "log_error".to_string(),
                    "get_time_ms".to_string(),
                    "random_float".to_string(),
                    "random_int".to_string(),
                ],
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9091,
                path: "/metrics".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
        }
    }
}

impl BrainConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/brain").required(false))
            .add_source(config::Environment::with_prefix("BRAIN").separator("__"))
            .build()?;
        
        Ok(config.try_deserialize()?)
    }
}