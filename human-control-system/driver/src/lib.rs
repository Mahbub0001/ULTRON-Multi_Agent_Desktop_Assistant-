//! Human Control System - Kernel Input Driver
//! 
//! Cross-platform low-level input injection library.
//! Windows: Interception driver | Linux: uinput | macOS: CGEvent (planned)

pub mod error;
pub mod types;
pub mod platform;
#[cfg(target_os = "windows")]
pub mod interception;
#[cfg(target_os = "linux")]
pub mod uinput;
pub mod hid;
pub mod macros;

pub use error::{DriverError, DriverResult};
pub use types::*;
pub use platform::*;

use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, warn, error, debug, instrument};

/// Trait for platform-specific input drivers
#[async_trait]
pub trait InputDriver: Send + Sync {
    /// Initialize the driver
    async fn initialize(&mut self) -> DriverResult<()>;
    
    /// Check if driver is ready
    fn is_ready(&self) -> bool;
    
    /// Inject keyboard event
    async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()>;
    
    /// Inject mouse event
    async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()>;
    
    /// Inject multiple events atomically
    async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()>;
    
    /// Get device info
    async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>>;
    
    /// Set exclusive mode (block other input)
    async fn set_exclusive(&self, exclusive: bool) -> DriverResult<()>;
    
    /// Shutdown driver
    async fn shutdown(&mut self) -> DriverResult<()>;
}

/// Keyboard event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyState {
    Down,
    Up,
}

/// Mouse event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

/// Mouse event
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub button: Option<MouseButton>,
    pub button_state: Option<KeyState>,
    pub absolute: bool,
    pub timestamp: u64,
}

/// Keyboard event
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyboardEvent {
    pub code: u16,
    pub state: KeyState,
    pub scan_code: u16,
    pub extended: bool,
    pub timestamp: u64,
}

/// Unified input event
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Mouse(MouseEvent),
    Delay(u64), // microseconds
}

impl InputEvent {
    pub fn key_down(code: u16, scan_code: u16) -> Self {
        Self::Keyboard(KeyboardEvent { code, scan_code, state: KeyState::Down, extended: false, timestamp: 0 })
    }

    pub fn key_up(code: u16, scan_code: u16) -> Self {
        Self::Keyboard(KeyboardEvent { code, scan_code, state: KeyState::Up, extended: false, timestamp: 0 })
    }
}

/// Device information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub vendor_id: u16,
    pub product_id: u16,
    pub is_keyboard: bool,
    pub is_mouse: bool,
    pub is_touch: bool,
}

/// Device types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeviceType {
    Keyboard,
    Mouse,
    Touchscreen,
    Gamepad,
    Tablet,
    Unknown,
}

/// Driver configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DriverConfig {
    pub backend: BackendType,
    pub exclusive_mode: bool,
    pub injection_delay_us: u64,
    pub max_batch_size: usize,
    pub device_filter: Option<DeviceFilter>,
}

/// Backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BackendType {
    Auto,
    Interception,
    UInput,
    HID,
    Synthetic,
}

/// Device filter for exclusive mode
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceFilter {
    pub vendor_ids: Option<Vec<u16>>,
    pub product_ids: Option<Vec<u16>>,
    pub device_names: Option<Vec<String>>,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            backend: BackendType::Auto,
            exclusive_mode: false,
            injection_delay_us: 1000, // 1ms default
            max_batch_size: 64,
            device_filter: None,
        }
    }
}

/// Driver factory
pub struct DriverFactory;

impl DriverFactory {
    /// Create the best available driver for the current platform
    pub async fn create_default(config: DriverConfig) -> DriverResult<Box<dyn InputDriver>> {
        let backend = match config.backend {
            BackendType::Auto => Self::detect_best_backend(),
            b => b,
        };
        
        info!("Creating driver with backend: {:?}", backend);
        Self::create(backend, config).await
    }
    
    /// Detect the best available backend
    fn detect_best_backend() -> BackendType {
        #[cfg(target_os = "windows")]
        {
            if Self::is_interception_available() {
                BackendType::Interception
            } else {
                BackendType::Synthetic
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if Self::is_uinput_available() {
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
        // Check if Interception driver is installed
        use std::path::Path;
        Path::new(r"C:\Windows\System32\drivers\interception.sys").exists() ||
        Path::new(r"C:\Windows\SysWOW64\drivers\interception.sys").exists()
    }
    
    #[cfg(target_os = "linux")]
    fn is_uinput_available() -> bool {
        use std::path::Path;
        Path::new("/dev/uinput").exists() || Path::new("/dev/input/uinput").exists()
    }
    
    /// Create driver for specific backend
    pub async fn create(backend: BackendType, config: DriverConfig) -> DriverResult<Box<dyn InputDriver>> {
        match backend {
            #[cfg(target_os = "windows")]
            BackendType::Interception => {
                let driver = interception::InterceptionDriver::new(config).await?;
                Ok(Box::new(driver))
            }
            
            #[cfg(target_os = "linux")]
            BackendType::UInput => {
                let driver = uinput::UInputDriver::new(config).await?;
                Ok(Box::new(driver))
            }
            
            BackendType::HID => {
                let driver = hid::HIDDriver::new(config).await?;
                Ok(Box::new(driver))
            }
            
            BackendType::Synthetic => {
                let driver = platform::SyntheticDriver::new(config).await?;
                Ok(Box::new(driver))
            }
            
            _ => Err(DriverError::UnsupportedBackend(backend)),
        }
    }
}

/// High-level input controller
pub struct InputController {
    driver: Arc<dyn InputDriver>,
    config: DriverConfig,
    stats: Arc<parking_lot::RwLock<ControllerStats>>,
}

#[derive(Debug, Default)]
struct ControllerStats {
    events_injected: u64,
    events_failed: u64,
    total_latency_us: u64,
    last_error: Option<String>,
}

impl InputController {
    /// Create new input controller
    pub async fn new(config: DriverConfig) -> DriverResult<Self> {
        let mut driver = DriverFactory::create_default(config.clone()).await?;
        driver.initialize().await?;
        Ok(Self {
            driver: Arc::from(driver),
            config,
            stats: Arc::new(parking_lot::RwLock::new(ControllerStats::default())),
        })
    }
    
    /// Type text with human-like timing
    #[instrument(skip(self))]
    pub async fn type_text(&self, text: &str, wpm: u32) -> DriverResult<()> {
        let delay_ms = (60_000 / (wpm * 5)) as u64; // Average 5 chars per word
        
        for ch in text.chars() {
            let events = Self::char_to_events(ch)?;
            self.inject_batch(events).await?;
            
            // Human-like variation
            let variation = (rand::random::<u64>() % (delay_ms / 2)) + (delay_ms / 2);
            tokio::time::sleep(tokio::time::Duration::from_millis(variation)).await;
        }
        
        Ok(())
    }
    
    /// Convert character to keyboard events
    fn char_to_events(ch: char) -> DriverResult<Vec<InputEvent>> {
        // Simplified - real implementation would use keyboard layout
        let vk = Self::char_to_virtual_key(ch)?;
        let scan = Self::virtual_key_to_scan_code(vk)?;
        
        Ok(vec![
            InputEvent::Keyboard(KeyboardEvent {
                code: vk,
                state: KeyState::Down,
                scan_code: scan,
                extended: false,
                timestamp: Self::current_timestamp(),
            }),
            InputEvent::Keyboard(KeyboardEvent {
                code: vk,
                state: KeyState::Up,
                scan_code: scan,
                extended: false,
                timestamp: Self::current_timestamp(),
            }),
        ])
    }
    
    #[cfg(target_os = "windows")]
    fn char_to_virtual_key(ch: char) -> DriverResult<u16> {
        use windows::Win32::UI::Input::KeyboardAndMouse::VkKeyScanW;
        let result = unsafe { VkKeyScanW(ch as u16) };
        if result == -1 {
            return Err(DriverError::InvalidCharacter(ch));
        }
        Ok(result as u16 & 0xFF)
    }
    
    #[cfg(not(target_os = "windows"))]
    fn char_to_virtual_key(ch: char) -> DriverResult<u16> {
        // Linux/macOS keycode mapping - simplified
        match ch {
            'a'..='z' => Ok((ch as u8 - b'a' + 4) as u16), // Linux keycodes
            'A'..='Z' => Ok((ch as u8 - b'A' + 4) as u16),
            '0'..='9' => Ok((ch as u8 - b'0' + 39) as u16),
            ' ' => Ok(44), // Space
            '\n' => Ok(40), // Enter
            '\t' => Ok(43), // Tab
            _ => Err(DriverError::InvalidCharacter(ch)),
        }
    }
    
    #[cfg(target_os = "windows")]
    fn virtual_key_to_scan_code(vk: u16) -> DriverResult<u16> {
        use windows::Win32::UI::Input::KeyboardAndMouse::MapVirtualKeyW;
        use windows::Win32::UI::Input::KeyboardAndMouse::MAPVK_VK_TO_VSC;
        let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) };
        Ok(scan as u16)
    }
    
    #[cfg(not(target_os = "windows"))]
    fn virtual_key_to_scan_code(vk: u16) -> DriverResult<u16> {
        Ok(vk) // Linux uses same codes
    }
    
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
    
    /// Move mouse to absolute position
    #[instrument(skip(self))]
    pub async fn move_mouse_absolute(&self, x: i32, y: i32, duration_ms: u64) -> DriverResult<()> {
        let start = self.get_mouse_position().await?;
        let steps = (duration_ms / 16).max(1) as i32; // ~60fps
        
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let eased = Self::ease_out_quad(t);
            
            let curr_x = start.0 + ((x - start.0) as f32 * eased) as i32;
            let curr_y = start.1 + ((y - start.1) as f32 * eased) as i32;
            
            self.inject_mouse(MouseEvent {
                x: curr_x,
                y: curr_y,
                dx: 0,
                dy: 0,
                button: None,
                button_state: None,
                absolute: true,
                timestamp: Self::current_timestamp(),
            }).await?;
            
            tokio::time::sleep(tokio::time::Duration::from_millis(16)).await;
        }
        
        Ok(())
    }
    
    /// Move mouse relative
    #[instrument(skip(self))]
    pub async fn move_mouse_relative(&self, dx: i32, dy: i32) -> DriverResult<()> {
        self.inject_mouse(MouseEvent {
            x: 0,
            y: 0,
            dx,
            dy,
            button: None,
            button_state: None,
            absolute: false,
            timestamp: Self::current_timestamp(),
        }).await
    }
    
    /// Click mouse button
    #[instrument(skip(self))]
    pub async fn click(&self, button: MouseButton, count: u32) -> DriverResult<()> {
        for _ in 0..count {
            self.inject_mouse(MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(button),
                button_state: Some(KeyState::Down),
                absolute: false,
                timestamp: Self::current_timestamp(),
            }).await?;
            
            // Human-like click duration (50-150ms)
            let duration = 50 + (rand::random::<u64>() % 100);
            tokio::time::sleep(tokio::time::Duration::from_millis(duration)).await;
            
            self.inject_mouse(MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(button),
                button_state: Some(KeyState::Up),
                absolute: false,
                timestamp: Self::current_timestamp(),
            }).await?;
            
            if count > 1 {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        Ok(())
    }
    
    /// Scroll mouse wheel
    #[instrument(skip(self))]
    pub async fn scroll(&self, delta_y: i32, delta_x: i32) -> DriverResult<()> {
        let button = if delta_y > 0 { MouseButton::WheelUp } 
                     else if delta_y < 0 { MouseButton::WheelDown }
                     else if delta_x > 0 { MouseButton::WheelRight }
                     else { MouseButton::WheelLeft };
        
        let clicks = delta_y.abs().max(delta_x.abs());
        for _ in 0..clicks {
            self.inject_mouse(MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(button),
                button_state: Some(KeyState::Down),
                absolute: false,
                timestamp: Self::current_timestamp(),
            }).await?;
            
            self.inject_mouse(MouseEvent {
                x: 0, y: 0, dx: 0, dy: 0,
                button: Some(button),
                button_state: Some(KeyState::Up),
                absolute: false,
                timestamp: Self::current_timestamp(),
            }).await?;
        }
        Ok(())
    }
    
    /// Press and hold key
    #[instrument(skip(self))]
    pub async fn key_down(&self, code: u16) -> DriverResult<()> {
        self.inject_keyboard(KeyboardEvent {
            code,
            state: KeyState::Down,
            scan_code: Self::virtual_key_to_scan_code(code)?,
            extended: false,
            timestamp: Self::current_timestamp(),
        }).await
    }
    
    /// Release key
    #[instrument(skip(self))]
    pub async fn key_up(&self, code: u16) -> DriverResult<()> {
        self.inject_keyboard(KeyboardEvent {
            code,
            state: KeyState::Up,
            scan_code: Self::virtual_key_to_scan_code(code)?,
            extended: false,
            timestamp: Self::current_timestamp(),
        }).await
    }
    
    /// Press key with duration
    #[instrument(skip(self))]
    pub async fn press_key(&self, code: u16, duration_ms: u64) -> DriverResult<()> {
        self.key_down(code).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        self.key_up(code).await
    }
    
    /// Hotkey combination (e.g., Ctrl+C)
    #[instrument(skip(self))]
    pub async fn hotkey(&self, modifiers: &[u16], key: u16) -> DriverResult<()> {
        // Press modifiers
        for &mod_code in modifiers {
            self.key_down(mod_code).await?;
        }
        
        // Press main key
        self.press_key(key, 50).await?;
        
        // Release modifiers in reverse order
        for &mod_code in modifiers.iter().rev() {
            self.key_up(mod_code).await?;
        }
        
        Ok(())
    }
    
    /// Get current mouse position (platform-specific)
    async fn get_mouse_position(&self) -> DriverResult<(i32, i32)> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use windows::Win32::Foundation::POINT;
            let mut pt = POINT::default();
            unsafe { GetCursorPos(&mut pt) };
            Ok((pt.x, pt.y))
        }
        
        #[cfg(target_os = "linux")]
        {
            // Use X11 or libinput - simplified
            Ok((0, 0))
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Ok((0, 0))
        }
    }
    
    /// Easing function for smooth movement
    fn ease_out_quad(t: f32) -> f32 {
        1.0 - (1.0 - t).powi(2)
    }
    
    /// Inject keyboard event
    #[instrument(skip(self))]
    pub async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        let start = std::time::Instant::now();
        let result = self.driver.inject_keyboard(event).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    /// Inject mouse event
    #[instrument(skip(self))]
    pub async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()> {
        let start = std::time::Instant::now();
        let result = self.driver.inject_mouse(event).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    /// Inject batch of events
    #[instrument(skip(self))]
    pub async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()> {
        let start = std::time::Instant::now();
        let result = self.driver.inject_batch(events).await;
        self.record_stats(start.elapsed().as_micros() as u64, result.is_err());
        result
    }
    
    fn record_stats(&self, latency_us: u64, failed: bool) {
        let mut stats = self.stats.write();
        stats.total_latency_us += latency_us;
        if failed {
            stats.events_failed += 1;
        } else {
            stats.events_injected += 1;
        }
    }
    
    /// Get controller statistics
    pub fn get_stats(&self) -> ControllerStatsSnapshot {
        let stats = self.stats.read();
        ControllerStatsSnapshot {
            events_injected: stats.events_injected,
            events_failed: stats.events_failed,
            avg_latency_us: if stats.events_injected > 0 {
                stats.total_latency_us / stats.events_injected
            } else { 0 },
            last_error: stats.last_error.clone(),
        }
    }
    
    /// Set exclusive mode
    pub async fn set_exclusive(&self, exclusive: bool) -> DriverResult<()> {
        self.driver.set_exclusive(exclusive).await
    }
    
    /// Get available devices
    pub async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>> {
        self.driver.get_devices().await
    }
    
    /// Shutdown controller
    pub async fn shutdown(&self) -> DriverResult<()> {
        // Driver shutdown handled by Drop
        Ok(())
    }
}

/// Snapshot of controller stats
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ControllerStatsSnapshot {
    pub events_injected: u64,
    pub events_failed: u64,
    pub avg_latency_us: u64,
    pub last_error: Option<String>,
}

