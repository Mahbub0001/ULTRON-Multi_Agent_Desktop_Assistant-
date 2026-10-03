//! Configuration for Adapters Service

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AdaptersConfig {
    pub server: ServerConfig,
    pub photoshop: PhotoshopConfig,
    pub chrome: ChromeConfig,
    pub game: GameConfig,
    pub window: WindowConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_concurrent_requests: usize,
    pub request_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhotoshopConfig {
    pub enabled: bool,
    pub uxp_enabled: bool,
    pub uxp_port: u16,
    pub uxp_host: String,
    pub cep_enabled: bool,
    pub cep_port: u16,
    pub cep_host: String,
    pub photoshop_path: String,
    pub default_width: u32,
    pub default_height: u32,
    pub default_resolution: f32,
    pub default_color_mode: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ChromeConfig {
    pub enabled: bool,
    pub cdp_endpoint: String,
    pub auto_attach: bool,
    pub chrome_path: String,
    pub launch_args: Vec<String>,
    pub navigation_timeout_ms: u64,
    pub evaluation_timeout_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GameConfig {
    pub enabled: bool,
    pub default_access: String,
    pub scan_max_results: usize,
    pub scan_chunk_size: usize,
    pub alloc_protection: String,
    pub alloc_type: String,
    pub require_admin: bool,
    pub validate_pointers: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowConfig {
    pub enabled: bool,
    pub windows: WindowsConfig,
    pub linux: LinuxConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WindowsConfig {
    pub use_dwm: bool,
    pub capture_cursor: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinuxConfig {
    pub use_x11: bool,
    pub use_wayland: bool,
    pub capture_cursor: bool,
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

impl Default for AdaptersConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 50054,
                max_concurrent_requests: 100,
                request_timeout_ms: 30000,
            },
            photoshop: PhotoshopConfig {
                enabled: true,
                uxp_enabled: true,
                uxp_port: 8080,
                uxp_host: "localhost".to_string(),
                cep_enabled: false,
                cep_port: 8888,
                cep_host: "localhost".to_string(),
                photoshop_path: "".to_string(),
                default_width: 1920,
                default_height: 1080,
                default_resolution: 72.0,
                default_color_mode: "RGB".to_string(),
            },
            chrome: ChromeConfig {
                enabled: true,
                cdp_endpoint: "http://localhost:9222".to_string(),
                auto_attach: true,
                chrome_path: "".to_string(),
                launch_args: vec![
                    "--remote-debugging-port=9222".to_string(),
                    "--no-first-run".to_string(),
                    "--no-default-browser-check".to_string(),
                    "--disable-extensions".to_string(),
                    "--disable-popup-blocking".to_string(),
                ],
                navigation_timeout_ms: 30000,
                evaluation_timeout_ms: 10000,
            },
            game: GameConfig {
                enabled: true,
                default_access: "READ_WRITE".to_string(),
                scan_max_results: 1000,
                scan_chunk_size: 65536,
                alloc_protection: "READ_WRITE_EXECUTE".to_string(),
                alloc_type: "COMMIT_RESERVE".to_string(),
                require_admin: true,
                validate_pointers: true,
            },
            window: WindowConfig {
                enabled: true,
                windows: WindowsConfig {
                    use_dwm: true,
                    capture_cursor: true,
                },
                linux: LinuxConfig {
                    use_x11: true,
                    use_wayland: false,
                    capture_cursor: true,
                },
            },
            metrics: MetricsConfig {
                enabled: true,
                port: 9092,
                path: "/metrics".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
            },
        }
    }
}

impl AdaptersConfig {
    pub fn load() -> anyhow::Result<Self> {
        let config = config::Config::builder()
            .add_source(config::File::with_name("config/adapters").required(false))
            .add_source(config::Environment::with_prefix("ADAPTERS").separator("__"))
            .build()?;

        Ok(config.try_deserialize()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AdaptersConfig::default();
        assert_eq!(config.server.port, 50054);
        assert!(config.photoshop.enabled);
        assert!(config.chrome.enabled);
        assert!(config.game.enabled);
        assert!(config.window.enabled);
    }
}