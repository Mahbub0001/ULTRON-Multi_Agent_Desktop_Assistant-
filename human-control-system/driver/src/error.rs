//! Error types for the driver

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Driver not initialized")]
    NotInitialized,
    
    #[error("Driver already initialized")]
    AlreadyInitialized,
    
    #[error("Unsupported backend: {0:?}")]
    UnsupportedBackend(crate::BackendType),
    
    #[error("Platform error: {0}")]
    PlatformError(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    
    #[error("Invalid character: {0}")]
    InvalidCharacter(char),
    
    #[error("Injection failed: {0}")]
    InjectionFailed(String),
    
    #[error("Timeout: {0}")]
    Timeout(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serial port error: {0}")]
    SerialError(#[from] serialport::Error),
    
    #[error("Windows API error: {0}")]
    #[cfg(target_os = "windows")]
    WindowsError(#[from] windows::core::Error),
    
    #[error("Nix error: {0}")]
    #[cfg(target_os = "linux")]
    NixError(#[from] nix::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Hardware error: {0}")]
    HardwareError(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

pub type DriverResult<T> = Result<T, DriverError>;

impl DriverError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            DriverError::Timeout(_) |
            DriverError::InjectionFailed(_) |
            DriverError::IoError(_)
        )
    }
    
    pub fn is_permission_error(&self) -> bool {
        matches!(self, DriverError::PermissionDenied(_))
    }
    
    pub fn is_hardware_error(&self) -> bool {
        matches!(self, DriverError::HardwareError(_))
    }
}
