//! Agent configuration

use crate::driver::DriverConfig as DriverDriverConfig;
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub server: ServerConfig,
    pub driver: DriverDriverConfig,
    pub auth: AuthConfig,
    pub macros: MacrosConfig,
    pub tasks: TasksConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    pub sandbox: SandboxConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub grpc_addr: SocketAddr,
    pub http_addr: Option<SocketAddr>,
    pub max_concurrent_streams: usize,
    pub max_message_size: usize,
    pub keepalive_interval_secs: u64,
    pub keepalive_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub jwt_secret: Option<String>,
    pub token_ttl_seconds: u64,
    pub admin_tokens: Vec<String>,
    pub default_capabilities: Vec<CapabilityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    pub resource: String,
    pub actions: Vec<String>,
    pub constraints: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacrosConfig {
    pub builtin_enabled: bool,
    pub custom_dir: PathBuf,
    pub max_events_per_macro: usize,
    pub default_variance_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksConfig {
    pub max_concurrent: usize,
    pub default_timeout_ms: u64,
    pub history_size: usize,
    pub persistence_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub file: Option<PathBuf>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Json,
    Text,
    Compact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub addr: SocketAddr,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub enabled: bool,
    pub allowed_executables: Vec<String>,
    pub blocked_paths: Vec<PathBuf>,
    pub max_memory_mb: u64,
    pub max_cpu_percent: u32,
    pub network_allowed: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            driver: DriverDriverConfig::default(),
            auth: AuthConfig::default(),
            macros: MacrosConfig::default(),
            tasks: TasksConfig::default(),
            logging: LoggingConfig::default(),
            metrics: MetricsConfig::default(),
            sandbox: SandboxConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            grpc_addr: "0.0.0.0:50051".parse().unwrap(),
            http_addr: Some("0.0.0.0:8080".parse().unwrap()),
            max_concurrent_streams: 100,
            max_message_size: 4 * 1024 * 1024, // 4MB
            keepalive_interval_secs: 30,
            keepalive_timeout_secs: 10,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            jwt_secret: None,
            token_ttl_seconds: 3600,
            admin_tokens: vec![],
            default_capabilities: vec![
                CapabilityConfig {
                    resource: "input:*".to_string(),
                    actions: vec!["write".to_string(), "execute".to_string()],
                    constraints: HashMap::new(),
                },
                CapabilityConfig {
                    resource: "macro:*".to_string(),
                    actions: vec!["execute".to_string()],
                    constraints: HashMap::new(),
                },
                CapabilityConfig {
                    resource: "task:*".to_string(),
                    actions: vec!["read".to_string(), "write".to_string(), "execute".to_string()],
                    constraints: HashMap::new(),
                },
                CapabilityConfig {
                    resource: "system:info".to_string(),
                    actions: vec!["read".to_string()],
                    constraints: HashMap::new(),
                },
            ],
        }
    }
}

impl Default for MacrosConfig {
    fn default() -> Self {
        Self {
            builtin_enabled: true,
            custom_dir: PathBuf::from("/etc/hcs/macros"),
            max_events_per_macro: 1000,
            default_variance_ms: 50,
        }
    }
}

impl Default for TasksConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            default_timeout_ms: 300_000, // 5 minutes
            history_size: 1000,
            persistence_dir: PathBuf::from("/var/lib/hcs/tasks"),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            file: Some(PathBuf::from("/var/log/hcs/agent.log")),
            max_file_size_mb: 100,
            max_files: 10,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            addr: "0.0.0.0:9090".parse().unwrap(),
            path: "/metrics".to_string(),
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_executables: vec![
                "python3".to_string(),
                "node".to_string(),
                "bash".to_string(),
                "powershell".to_string(),
            ],
            blocked_paths: vec![
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/etc/shadow"),
                PathBuf::from("/root"),
                PathBuf::from("C:\\Windows\\System32"),
            ],
            max_memory_mb: 512,
            max_cpu_percent: 50,
            network_allowed: false,
        }
    }
}

impl AgentConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name("/etc/hcs/agent").required(false))
            .add_source(File::with_name("config/agent").required(false))
            .add_source(Environment::with_prefix("HCS").separator("_"))
            .build()?;
        
        config.try_deserialize()
    }
    
    pub fn load_from_file(path: &str) -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::with_name(path).required(true))
            .add_source(Environment::with_prefix("HCS").separator("_"))
            .build()?;
        
        config.try_deserialize()
    }
}