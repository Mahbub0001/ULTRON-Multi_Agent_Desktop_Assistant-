//! Platform abstraction module

pub mod common;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use crate::{DriverConfig, DriverError, DriverResult, InputDriver, InputEvent, DeviceInfo, BackendType};
use async_trait::async_trait;
use tracing::{info, warn, error, debug};

/// Synthetic driver for testing/fallback
pub struct SyntheticDriver {
    config: DriverConfig,
    initialized: bool,
}

impl SyntheticDriver {
    pub async fn new(config: DriverConfig) -> DriverResult<Self> {
        info!("Creating synthetic driver (no-op)");
        Ok(Self {
            config,
            initialized: false,
        })
    }
}

#[async_trait]
impl InputDriver for SyntheticDriver {
    async fn initialize(&mut self) -> DriverResult<()> {
        self.initialized = true;
        warn!("Synthetic driver initialized - NO INPUT WILL BE INJECTED");
        Ok(())
    }
    
    fn is_ready(&self) -> bool {
        self.initialized
    }
    
    async fn inject_keyboard(&self, event: crate::KeyboardEvent) -> DriverResult<()> {
        debug!("SYNTHETIC: Key {:04X} {:?}", event.code, event.state);
        Ok(())
    }
    
    async fn inject_mouse(&self, event: crate::MouseEvent) -> DriverResult<()> {
        debug!("SYNTHETIC: Mouse ({}, {}) btn={:?}", event.x, event.y, event.button);
        Ok(())
    }
    
    async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()> {
        debug!("SYNTHETIC: Batch of {} events", events.len());
        Ok(())
    }
    
    async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>> {
        Ok(vec![
            DeviceInfo {
                id: "synthetic-kbd".into(),
                name: "Synthetic Keyboard".into(),
                device_type: crate::DeviceType::Keyboard,
                vendor_id: 0x1234,
                product_id: 0x5678,
                is_keyboard: true,
                is_mouse: false,
                is_touch: false,
            },
            DeviceInfo {
                id: "synthetic-mouse".into(),
                name: "Synthetic Mouse".into(),
                device_type: crate::DeviceType::Mouse,
                vendor_id: 0x1234,
                product_id: 0x5679,
                is_keyboard: false,
                is_mouse: true,
                is_touch: false,
            },
        ])
    }
    
    async fn set_exclusive(&self, _exclusive: bool) -> DriverResult<()> {
        warn!("Exclusive mode not supported in synthetic driver");
        Ok(())
    }
    
    async fn shutdown(&mut self) -> DriverResult<()> {
        self.initialized = false;
        Ok(())
    }
}

/// Get the default driver for current platform
pub async fn create_default_driver(config: DriverConfig) -> DriverResult<Box<dyn InputDriver>> {
    let backend = match config.backend {
        BackendType::Auto => detect_best_backend(),
        b => b,
    };
    
    create_driver(backend, config).await
}

fn detect_best_backend() -> BackendType {
    #[cfg(target_os = "windows")]
    {
        if is_interception_available() {
            BackendType::Interception
        } else {
            BackendType::Synthetic
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if is_uinput_available() {
            BackendType::UInput
        } else {
            BackendType::Synthetic
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        BackendType::Synthetic
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        BackendType::Synthetic
    }
}

#[cfg(target_os = "windows")]
fn is_interception_available() -> bool {
    use std::path::Path;
    Path::new(r"C:\Windows\System32\drivers\interception.sys").exists() ||
    Path::new(r"C:\Windows\SysWOW64\drivers\interception.sys").exists()
}

#[cfg(target_os = "linux")]
fn is_uinput_available() -> bool {
    use std::path::Path;
    Path::new("/dev/uinput").exists() || Path::new("/dev/input/uinput").exists()
}

async fn create_driver(backend: BackendType, config: DriverConfig) -> DriverResult<Box<dyn InputDriver>> {
    match backend {
        #[cfg(target_os = "windows")]
        BackendType::Interception => {
            let driver = crate::interception::InterceptionDriver::new(config).await?;
            Ok(Box::new(driver))
        }
        
        #[cfg(target_os = "linux")]
        BackendType::UInput => {
            let driver = crate::uinput::UInputDriver::new(config).await?;
            Ok(Box::new(driver))
        }
        
        BackendType::HID => {
            let driver = crate::hid::HIDDriver::new(config).await?;
            Ok(Box::new(driver))
        }
        
        BackendType::Synthetic => {
            let driver = SyntheticDriver::new(config).await?;
            Ok(Box::new(driver))
        }
        
        _ => Err(DriverError::UnsupportedBackend(backend)),
    }
}