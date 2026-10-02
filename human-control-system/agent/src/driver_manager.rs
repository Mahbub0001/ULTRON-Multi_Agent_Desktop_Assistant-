//! Driver manager - manages input driver lifecycle

use crate::driver::{
    DriverConfig, DriverError, DriverResult, InputDriver, InputEvent,
    KeyboardEvent, MouseEvent, KeyState, MouseButton, DeviceInfo, BackendType
};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};

pub struct DriverManager {
    config: DriverConfig,
    driver: Arc<RwLock<Option<Box<dyn InputDriver>>>>,
    stats: Arc<DriverStats>,
    macros: Arc<DashMap<String, MacroDefinition>>,
}

#[derive(Debug, Default)]
pub struct DriverStats {
    events_injected: std::sync::atomic::AtomicU64,
    events_failed: std::sync::atomic::AtomicU64,
    total_latency_us: std::sync::atomic::AtomicU64,
    last_error: parking_lot::RwLock<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub name: String,
    pub description: String,
    pub events: Vec<InputEvent>,
    pub variance_ms: u32,
}

impl DriverManager {
    pub fn new(config: DriverConfig) -> Self {
        Self {
            config,
            driver: Arc::new(RwLock::new(None)),
            stats: Arc::new(DriverStats::default()),
            macros: Arc::new(DashMap::new()),
        }
    }
    
    pub async fn initialize(&self) -> Result<()> {
        let mut driver_guard = self.driver.write().await;
        
        if driver_guard.is_some() {
            return Err(anyhow::anyhow!("Driver already initialized"));
        }
        
        info!("Initializing driver with backend: {:?}", self.config.backend);
        
        let driver = crate::driver::DriverFactory::create_default(self.config.clone()).await?;
        driver.initialize().await?;
        
        *driver_guard = Some(driver);
        
        // Register built-in macros
        self.register_builtin_macros().await;
        
        info!("Driver initialized successfully");
        Ok(())
    }
    
    pub async fn shutdown(&self) -> Result<()> {
        let mut driver_guard = self.driver.write().await;
        
        if let Some(mut driver) = driver_guard.take() {
            driver.shutdown().await?;
        }
        
        info!("Driver shutdown complete");
        Ok(())
    }
    
    pub async fn is_ready(&self) -> bool {
        let driver_guard = self.driver.read().await;
        driver_guard.as_ref().map(|d| d.is_ready()).unwrap_or(false)
    }
    
    pub async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        let driver_guard = self.driver.read().await;
        let driver = driver_guard.as_ref().ok_or(DriverError::NotInitialized)?;
        
        let start = std::time::Instant::now();
        let result = driver.inject_keyboard(event).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    pub async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()> {
        let driver_guard = self.driver.read().await;
        let driver = driver_guard.as_ref().ok_or(DriverError::NotInitialized)?;
        
        let start = std::time::Instant::now();
        let result = driver.inject_mouse(event).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    pub async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()> {
        let driver_guard = self.driver.read().await;
        let driver = driver_guard.as_ref().ok_or(DriverError::NotInitialized)?;
        
        let start = std::time::Instant::now();
        let result = driver.inject_batch(events).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    pub async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>> {
        let driver_guard = self.driver.read().await;
        let driver = driver_guard.as_ref().ok_or(DriverError::NotInitialized)?;
        driver.get_devices().await
    }
    
    pub async fn set_exclusive(&self, exclusive: bool) -> DriverResult<()> {
        let driver_guard = self.driver.read().await;
        let driver = driver_guard.as_ref().ok_or(DriverError::NotInitialized)?;
        driver.set_exclusive(exclusive).await
    }
    
    pub async fn reconfigure(&self, config: DriverConfig) -> Result<()> {
        // Shutdown current driver
        self.shutdown().await?;
        
        // Create new driver with new config
        self.config = config;
        self.initialize().await
    }
    
    pub fn get_stats(&self) -> DriverStatsSnapshot {
        DriverStatsSnapshot {
            events_injected: self.stats.events_injected.load(std::sync::atomic::Ordering::Relaxed),
            events_failed: self.stats.events_failed.load(std::sync::atomic::Ordering::Relaxed),
            avg_latency_us: {
                let total = self.stats.total_latency_us.load(std::sync::atomic::Ordering::Relaxed);
                let count = self.stats.events_injected.load(std::sync::atomic::Ordering::Relaxed);
                if count > 0 { total / count } else { 0 }
            },
            last_error: self.stats.last_error.read().clone(),
        }
    }
    
    fn record_stats(&self, latency_us: u64, failed: bool) {
        self.stats.total_latency_us.fetch_add(latency_us, std::sync::atomic::Ordering::Relaxed);
        if failed {
            self.stats.events_failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        } else {
            self.stats.events_injected.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    
    // Macro management
    pub async fn register_macro(&self, macro_def: MacroDefinition) -> Result<()> {
        if macro_def.events.len() > 1000 {
            return Err(anyhow::anyhow!("Macro too large: max 1000 events"));
        }
        
        self.macros.insert(macro_def.name.clone(), macro_def);
        Ok(())
    }
    
    pub async fn unregister_macro(&self, name: &str) -> Result<()> {
        self.macros.remove(name);
        Ok(())
    }
    
    pub async fn execute_macro(&self, name: &str, variance_ms: u32) -> Result<()> {
        let macro_def = self.macros.get(name)
            .ok_or_else(|| anyhow::anyhow!("Macro not found: {}", name))?;
        
        // Apply variance to delays
        let mut events = macro_def.events.clone();
        if variance_ms > 0 {
            for event in &mut events {
                if let InputEvent::Delay(delay) = event {
                    let variance = (rand::random::<u64>() % (variance_ms as u64 * 1000));
                    delay.microseconds = delay.microseconds.saturating_add(variance);
                }
            }
        }
        
        self.inject_batch(events).await?;
        Ok(())
    }
    
    pub async fn list_macros(&self) -> Vec<MacroInfo> {
        self.macros.iter()
            .map(|m| MacroInfo {
                name: m.name.clone(),
                description: m.description.clone(),
                event_count: m.events.len(),
            })
            .collect()
    }
    
    async fn register_builtin_macros(&self) {
        use crate::driver::macros::*;
        
        // Register all built-in shortcuts
        let shortcuts = vec![
            ("copy", shortcuts::copy(), "Copy (Ctrl+C)"),
            ("paste", shortcuts::paste(), "Paste (Ctrl+V)"),
            ("cut", shortcuts::cut(), "Cut (Ctrl+X)"),
            ("select_all", shortcuts::select_all(), "Select All (Ctrl+A)"),
            ("undo", shortcuts::undo(), "Undo (Ctrl+Z)"),
            ("redo", shortcuts::redo(), "Redo (Ctrl+Y)"),
            ("save", shortcuts::save(), "Save (Ctrl+S)"),
            ("new", shortcuts::new(), "New (Ctrl+N)"),
            ("open", shortcuts::open(), "Open (Ctrl+O)"),
            ("find", shortcuts::find(), "Find (Ctrl+F)"),
            ("close_tab", shortcuts::close_tab(), "Close Tab (Ctrl+W)"),
            ("next_tab", shortcuts::next_tab(), "Next Tab (Ctrl+Tab)"),
            ("prev_tab", shortcuts::prev_tab(), "Previous Tab (Ctrl+Shift+Tab)"),
            ("alt_tab", shortcuts::alt_tab(), "Alt+Tab"),
            ("show_desktop", shortcuts::show_desktop(), "Show Desktop (Win+D)"),
            ("file_explorer", shortcuts::file_explorer(), "File Explorer (Win+E)"),
            ("run_dialog", shortcuts::run_dialog(), "Run Dialog (Win+R)"),
            ("task_manager", shortcuts::task_manager(), "Task Manager (Ctrl+Shift+Esc)"),
            ("print_screen", shortcuts::print_screen(), "Print Screen"),
            ("alt_print_screen", shortcuts::alt_print_screen(), "Alt+PrintScreen"),
        ];
        
        for (name, events, desc) in shortcuts {
            let macro_def = MacroDefinition {
                name: name.to_string(),
                description: desc.to_string(),
                events,
                variance_ms: self.config.injection_delay_us as u32 / 1000,
            };
            let _ = self.register_macro(macro_def).await;
        }
        
        // Gaming macros
        let gaming_macros = vec![
            ("jump", gaming::jump(), "Jump"),
            ("crouch", gaming::crouch(), "Crouch"),
            ("uncrouch", gaming::uncrouch(), "Uncrouch"),
            ("sprint", gaming::sprint(), "Sprint"),
            ("unsprint", gaming::unsprint(), "Stop Sprint"),
            ("reload", gaming::reload(), "Reload"),
            ("interact", gaming::interact(), "Interact"),
        ];
        
        for (name, events, desc) in gaming_macros {
            let macro_def = MacroDefinition {
                name: name.to_string(),
                description: desc.to_string(),
                events,
                variance_ms: 20,
            };
            let _ = self.register_macro(macro_def).await;
        }
        
        // Movement macros
        let move_macros = vec![
            ("move_forward", gaming::move_forward(), "Move Forward"),
            ("move_backward", gaming::move_backward(), "Move Backward"),
            ("move_left", gaming::move_left(), "Move Left"),
            ("move_right", gaming::move_right(), "Move Right"),
            ("stop_movement", gaming::stop_movement(), "Stop Movement"),
        ];
        
        for (name, events, desc) in move_macros {
            let macro_def = MacroDefinition {
                name: name.to_string(),
                description: desc.to_string(),
                events,
                variance_ms: 10,
            };
            let _ = self.register_macro(macro_def).await;
        }
        
        info!("Registered {} built-in macros", self.macros.len());
    }
}

#[derive(Debug, Clone)]
pub struct DriverStatsSnapshot {
    pub events_injected: u64,
    pub events_failed: u64,
    pub avg_latency_us: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MacroInfo {
    pub name: String,
    pub description: String,
    pub event_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::DriverConfig;
    
    #[tokio::test]
    async fn test_driver_manager_synthetic() {
        let config = DriverConfig {
            backend: BackendType::Synthetic,
            ..Default::default()
        };
        
        let manager = DriverManager::new(config);
        manager.initialize().await.unwrap();
        
        assert!(manager.is_ready().await);
        
        // Test keyboard injection
        let event = KeyboardEvent {
            code: 0x41, // A
            state: KeyState::Down,
            scan_code: 0x1E,
            extended: false,
            timestamp: 0,
        };
        manager.inject_keyboard(event).await.unwrap();
        
        let stats = manager.get_stats();
        assert_eq!(stats.events_injected, 1);
        
        manager.shutdown().await.unwrap();
    }
}