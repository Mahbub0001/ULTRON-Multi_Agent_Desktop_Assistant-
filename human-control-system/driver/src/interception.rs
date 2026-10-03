//! Windows Interception Driver Implementation
//! 
//! Uses the Interception driver (https://github.com/oblita/Interception)
//! Must be installed separately: https://github.com/oblita/Interception/releases

use crate::{
    DriverConfig, DriverError, DriverResult, InputDriver, InputEvent,
    KeyboardEvent, MouseEvent, KeyState, MouseButton, DeviceInfo, DeviceType,
    BackendType, types::{VirtualKey, ScanCode}
};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;
use tracing::{info, warn, error, debug, instrument};
use windows::Win32::Foundation::*;
use std::ffi::c_void;
use windows::Win32::System::LibraryLoader::*;
use windows::core::*;

// Interception DLL function signatures
type InterceptionCreateContext = unsafe extern "system" fn() -> *mut c_void;
type InterceptionDestroyContext = unsafe extern "system" fn(context: *mut c_void);
type InterceptionGetFilter = unsafe extern "system" fn(context: *mut c_void, predicate: IsPredicate) -> u32;
type InterceptionSetFilter = unsafe extern "system" fn(context: *mut c_void, predicate: IsPredicate, filter: u32) -> i32;
type InterceptionReceive = unsafe extern "system" fn(context: *mut c_void, device: i32, stroke: *mut c_void, nstroke: u32) -> i32;
type InterceptionSend = unsafe extern "system" fn(context: *mut c_void, device: i32, stroke: *const c_void, nstroke: u32) -> i32;
type InterceptionIsKeyboard = unsafe extern "system" fn(device: i32) -> i32;
type InterceptionIsMouse = unsafe extern "system" fn(device: i32) -> i32;
type InterceptionGetHardwareID = unsafe extern "system" fn(context: *mut c_void, device: i32, buffer: *mut u16, size: u32) -> u32;

// Predicate types
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsPredicate {
    Keyboard = 0x0001,
    Mouse = 0x0002,
    MouseLeftButtonDown = 0x0004,
    MouseLeftButtonUp = 0x0008,
    MouseRightButtonDown = 0x0010,
    MouseRightButtonUp = 0x0020,
    MouseMiddleButtonDown = 0x0040,
    MouseMiddleButtonUp = 0x0080,
    MouseButton4Down = 0x0100,
    MouseButton4Up = 0x0200,
    MouseButton5Down = 0x0400,
    MouseButton5Up = 0x0800,
    MouseWheel = 0x1000,
    MouseHWheel = 0x2000,
    MouseMoveRelative = 0x4000,
    MouseMoveAbsolute = 0x8000,
    KeyDown = 0x10000,
    KeyUp = 0x20000,
    KeyE0 = 0x40000,
    KeyE1 = 0x80000,
}

// Interception stroke structures
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterceptionKeyStroke {
    pub code: u16,
    pub state: u16,
    pub information: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InterceptionMouseStroke {
    pub state: u16,
    pub flags: u16,
    pub rolling: i16,
    pub x: i32,
    pub y: i32,
    pub information: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union InterceptionStroke {
    pub key: InterceptionKeyStroke,
    pub mouse: InterceptionMouseStroke,
}

// Mouse state flags
const INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN: u16 = 0x0001;
const INTERCEPTION_MOUSE_LEFT_BUTTON_UP: u16 = 0x0002;
const INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN: u16 = 0x0004;
const INTERCEPTION_MOUSE_RIGHT_BUTTON_UP: u16 = 0x0008;
const INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN: u16 = 0x0010;
const INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP: u16 = 0x0020;
const INTERCEPTION_MOUSE_BUTTON_1_DOWN: u16 = INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN;
const INTERCEPTION_MOUSE_BUTTON_1_UP: u16 = INTERCEPTION_MOUSE_LEFT_BUTTON_UP;
const INTERCEPTION_MOUSE_BUTTON_2_DOWN: u16 = INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN;
const INTERCEPTION_MOUSE_BUTTON_2_UP: u16 = INTERCEPTION_MOUSE_RIGHT_BUTTON_UP;
const INTERCEPTION_MOUSE_BUTTON_3_DOWN: u16 = INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN;
const INTERCEPTION_MOUSE_BUTTON_3_UP: u16 = INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP;
const INTERCEPTION_MOUSE_BUTTON_4_DOWN: u16 = 0x0040;
const INTERCEPTION_MOUSE_BUTTON_4_UP: u16 = 0x0080;
const INTERCEPTION_MOUSE_BUTTON_5_DOWN: u16 = 0x0100;
const INTERCEPTION_MOUSE_BUTTON_5_UP: u16 = 0x0200;
const INTERCEPTION_MOUSE_WHEEL: u16 = 0x0400;
const INTERCEPTION_MOUSE_HWHEEL: u16 = 0x0800;
const INTERCEPTION_MOUSE_MOVE_RELATIVE: u16 = 0x0000;
const INTERCEPTION_MOUSE_MOVE_ABSOLUTE: u16 = 0x1000;
const INTERCEPTION_MOUSE_VIRTUAL_DESKTOP: u16 = 0x0200;

// Key state flags
const INTERCEPTION_KEY_DOWN: u16 = 0x0000;
const INTERCEPTION_KEY_UP: u16 = 0x0001;
const INTERCEPTION_KEY_E0: u16 = 0x0002;
const INTERCEPTION_KEY_E1: u16 = 0x0004;

/// Interception driver wrapper
pub struct InterceptionDriver {
    config: DriverConfig,
    dll_handle: Option<HMODULE>,
    context: *mut c_void,
    initialized: bool,
    device_cache: Arc<Mutex<Vec<CachedDevice>>>,
}

#[derive(Debug, Clone)]
struct CachedDevice {
    id: i32,
    name: String,
    device_type: DeviceType,
    vendor_id: u16,
    product_id: u16,
    is_keyboard: bool,
    is_mouse: bool,
}

impl InterceptionDriver {
    /// Create new Interception driver
    pub async fn new(config: DriverConfig) -> DriverResult<Self> {
        info!("Initializing Interception driver...");
        
        let driver = Self {
            config,
            dll_handle: None,
            context: std::ptr::null_mut(),
            initialized: false,
            device_cache: Arc::new(Mutex::new(Vec::new())),
        };
        
        Ok(driver)
    }
    
    /// Load Interception DLL and initialize
    #[instrument(skip(self))]
    pub async fn initialize(&mut self) -> DriverResult<()> {
        if self.initialized {
            return Err(DriverError::AlreadyInitialized);
        }
        
        // Load interception.dll
        let dll_path = self.find_interception_dll()?;
        info!("Loading Interception from: {:?}", dll_path);
        
        unsafe {
            self.dll_handle = Some(LoadLibraryW(&HSTRING::from(dll_path))?);
        }
        
        let dll = self.dll_handle.unwrap();
        
        // Get function pointers
        let create_context: InterceptionCreateContext = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_create_context")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let destroy_context: InterceptionDestroyContext = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_destroy_context")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let set_filter: InterceptionSetFilter = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_set_filter")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let send: InterceptionSend = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_send")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let is_keyboard: InterceptionIsKeyboard = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_is_keyboard")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let is_mouse: InterceptionIsMouse = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_is_mouse")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        let get_hardware_id: InterceptionGetHardwareID = unsafe {
            std::mem::transmute(GetProcAddress(dll, s!("interception_get_hardware_id")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        
        // Create context
        self.context = unsafe { create_context() };
        if self.context.is_null() {
            return Err(DriverError::PlatformError("Failed to create interception context".into()));
        }
        
        // Set filters for keyboard and mouse
        let filter = IsPredicate::KeyDown as u32 
            | IsPredicate::KeyUp as u32 
            | IsPredicate::KeyE0 as u32 
            | IsPredicate::KeyE1 as u32
            | IsPredicate::MouseMoveRelative as u32
            | IsPredicate::MouseMoveAbsolute as u32
            | IsPredicate::MouseLeftButtonDown as u32
            | IsPredicate::MouseLeftButtonUp as u32
            | IsPredicate::MouseRightButtonDown as u32
            | IsPredicate::MouseRightButtonUp as u32
            | IsPredicate::MouseMiddleButtonDown as u32
            | IsPredicate::MouseMiddleButtonUp as u32
            | IsPredicate::MouseButton4Down as u32
            | IsPredicate::MouseButton4Up as u32
            | IsPredicate::MouseButton5Down as u32
            | IsPredicate::MouseButton5Up as u32
            | IsPredicate::MouseWheel as u32
            | IsPredicate::MouseHWheel as u32;
        
        unsafe {
            set_filter(self.context, IsPredicate::Keyboard, filter);
            set_filter(self.context, IsPredicate::Mouse, filter);
        }
        
        // Cache devices
        self.cache_devices(is_keyboard, is_mouse, get_hardware_id).await?;
        
        self.initialized = true;
        info!("Interception driver initialized successfully");
        Ok(())
    }
    
    fn find_interception_dll(&self) -> DriverResult<std::ffi::OsString> {
        let paths = vec![
            r"C:\Windows\System32\interception.dll",
            r"C:\Windows\SysWOW64\interception.dll",
            r".\interception.dll",
            r"..\interception.dll",
        ];
        
        for path in paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.into());
            }
        }
        
        // Check PATH
        if let Ok(output) = std::process::Command::new("where").arg("interception.dll").output() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path.into());
            }
        }
        
        Err(DriverError::PlatformError(
            "Interception DLL not found. Install from https://github.com/oblita/Interception/releases".into()
        ))
    }
    
    async fn cache_devices(
        &self,
        is_keyboard: InterceptionIsKeyboard,
        is_mouse: InterceptionIsMouse,
        get_hardware_id: InterceptionGetHardwareID,
    ) -> DriverResult<()> {
        let mut cache = self.device_cache.lock();
        cache.clear();
        
        for device_id in 1..=20 {
            let is_kbd = unsafe { is_keyboard(device_id) != 0 };
            let is_mse = unsafe { is_mouse(device_id) != 0 };
            
            if !is_kbd && !is_mse {
                continue;
            }
            
            let mut buffer = [0u16; 500];
            let size = unsafe {
                get_hardware_id(self.context, device_id, buffer.as_mut_ptr(), buffer.len() as u32)
            };
            
            let hw_id = if size > 0 && size <= buffer.len() as u32 {
                String::from_utf16_lossy(&buffer[..size as usize])
            } else {
                format!("Device_{}", device_id)
            };
            
            let (vendor_id, product_id) = Self::parse_hardware_id(&hw_id);
            
            cache.push(CachedDevice {
                id: device_id,
                name: hw_id.clone(),
                device_type: if is_kbd { DeviceType::Keyboard } else { DeviceType::Mouse },
                vendor_id,
                product_id,
                is_keyboard: is_kbd,
                is_mouse: is_mse,
            });
        }
        
        info!("Cached {} interception devices", cache.len());
        Ok(())
    }
    
    fn parse_hardware_id(hw_id: &str) -> (u16, u16) {
        // Parse VID_XXXX&PID_YYYY format
        let mut vid = 0u16;
        let mut pid = 0u16;
        
        for part in hw_id.split('&') {
            if let Some(v) = part.strip_prefix("VID_") {
                vid = u16::from_str_radix(v, 16).unwrap_or(0);
            } else if let Some(p) = part.strip_prefix("PID_") {
                pid = u16::from_str_radix(p, 16).unwrap_or(0);
            }
        }
        
        (vid, pid)
    }
    
    fn get_keyboard_device(&self) -> Option<i32> {
        let cache = self.device_cache.lock();
        cache.iter().find(|d| d.is_keyboard).map(|d| d.id)
    }
    
    fn get_mouse_device(&self) -> Option<i32> {
        let cache = self.device_cache.lock();
        cache.iter().find(|d| d.is_mouse).map(|d| d.id)
    }
}

#[async_trait]
impl InputDriver for InterceptionDriver {
    async fn initialize(&mut self) -> DriverResult<()> {
        self.initialize().await
    }
    
    fn is_ready(&self) -> bool {
        self.initialized && !self.context.is_null()
    }
    
    #[instrument(skip(self))]
    async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        if !self.is_ready() {
            return Err(DriverError::NotInitialized);
        }
        
        let device = self.get_keyboard_device()
            .ok_or_else(|| DriverError::DeviceNotFound("No keyboard device".into()))?;
        
        let mut stroke = InterceptionKeyStroke {
            code: event.code,
            state: match event.state {
                KeyState::Down => INTERCEPTION_KEY_DOWN,
                KeyState::Up => INTERCEPTION_KEY_UP,
            },
            information: 0,
        };
        
        if event.extended {
            stroke.state |= INTERCEPTION_KEY_E0;
        }
        
        let send_fn: InterceptionSend = unsafe {
            std::mem::transmute(GetProcAddress(self.dll_handle.unwrap(), s!("interception_send")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        
        let sent = unsafe {
            send_fn(self.context, device, &stroke as *const _ as *const c_void, 1)
        };
        
        if sent != 1 {
            return Err(DriverError::InjectionFailed("Failed to send keystroke".into()));
        }
        
        // Small delay for stability
        if self.config.injection_delay_us > 0 {
            tokio::time::sleep(tokio::time::Duration::from_micros(self.config.injection_delay_us)).await;
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()> {
        if !self.is_ready() {
            return Err(DriverError::NotInitialized);
        }
        
        let device = self.get_mouse_device()
            .ok_or_else(|| DriverError::DeviceNotFound("No mouse device".into()))?;
        
        let mut stroke = InterceptionMouseStroke {
            state: 0,
            flags: 0,
            rolling: 0,
            x: event.x,
            y: event.y,
            information: 0,
        };
        
        // Set flags
        if event.absolute {
            stroke.flags |= INTERCEPTION_MOUSE_MOVE_ABSOLUTE | INTERCEPTION_MOUSE_VIRTUAL_DESKTOP;
        } else {
            stroke.flags |= INTERCEPTION_MOUSE_MOVE_RELATIVE;
        }
        
        // Set button state
        if let (Some(button), Some(state)) = (event.button, event.button_state) {
            stroke.state = match (button, state) {
                (MouseButton::Left, KeyState::Down) => INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN,
                (MouseButton::Left, KeyState::Up) => INTERCEPTION_MOUSE_LEFT_BUTTON_UP,
                (MouseButton::Right, KeyState::Down) => INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN,
                (MouseButton::Right, KeyState::Up) => INTERCEPTION_MOUSE_RIGHT_BUTTON_UP,
                (MouseButton::Middle, KeyState::Down) => INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN,
                (MouseButton::Middle, KeyState::Up) => INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP,
                (MouseButton::X1, KeyState::Down) => INTERCEPTION_MOUSE_BUTTON_4_DOWN,
                (MouseButton::X1, KeyState::Up) => INTERCEPTION_MOUSE_BUTTON_4_UP,
                (MouseButton::X2, KeyState::Down) => INTERCEPTION_MOUSE_BUTTON_5_DOWN,
                (MouseButton::X2, KeyState::Up) => INTERCEPTION_MOUSE_BUTTON_5_UP,
                (MouseButton::WheelUp, _) => {
                    stroke.flags |= INTERCEPTION_MOUSE_WHEEL;
                    stroke.rolling = 120;
                    0
                }
                (MouseButton::WheelDown, _) => {
                    stroke.flags |= INTERCEPTION_MOUSE_WHEEL;
                    stroke.rolling = -120;
                    0
                }
                (MouseButton::WheelLeft, _) => {
                    stroke.flags |= INTERCEPTION_MOUSE_HWHEEL;
                    stroke.rolling = -120;
                    0
                }
                (MouseButton::WheelRight, _) => {
                    stroke.flags |= INTERCEPTION_MOUSE_HWHEEL;
                    stroke.rolling = 120;
                    0
                }
            };
        }
        
        let send_fn: InterceptionSend = unsafe {
            std::mem::transmute(GetProcAddress(self.dll_handle.unwrap(), s!("interception_send")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        
        let sent = unsafe {
            send_fn(self.context, device, &stroke as *const _ as *const c_void, 1)
        };
        
        if sent != 1 {
            return Err(DriverError::InjectionFailed("Failed to send mouse stroke".into()));
        }
        
        if self.config.injection_delay_us > 0 {
            tokio::time::sleep(tokio::time::Duration::from_micros(self.config.injection_delay_us)).await;
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()> {
        if events.len() > self.config.max_batch_size {
            // Split into chunks
            for chunk in events.chunks(self.config.max_batch_size) {
                self.inject_batch(chunk.to_vec()).await?;
            }
            return Ok(());
        }
        
        // Convert to interception strokes
        let mut strokes = Vec::with_capacity(events.len());
        
        for event in events {
            match event {
                InputEvent::Keyboard(k) => {
                    let device = self.get_keyboard_device().ok_or(DriverError::DeviceNotFound("Keyboard".into()))?;
                    let mut stroke = InterceptionKeyStroke {
                        code: k.code,
                        state: match k.state {
                            KeyState::Down => INTERCEPTION_KEY_DOWN,
                            KeyState::Up => INTERCEPTION_KEY_UP,
                        },
                        information: 0,
                    };
                    if k.extended {
                        stroke.state |= INTERCEPTION_KEY_E0;
                    }
                    strokes.push((device, InterceptionStroke { key: stroke }));
                }
                InputEvent::Mouse(m) => {
                    let device = self.get_mouse_device().ok_or(DriverError::DeviceNotFound("Mouse".into()))?;
                    let mut stroke = InterceptionMouseStroke {
                        state: 0,
                        flags: if m.absolute { 
                            INTERCEPTION_MOUSE_MOVE_ABSOLUTE | INTERCEPTION_MOUSE_VIRTUAL_DESKTOP 
                        } else { 
                            INTERCEPTION_MOUSE_MOVE_RELATIVE 
                        },
                        rolling: 0,
                        x: m.x,
                        y: m.y,
                        information: 0,
                    };
                    
                    if let (Some(btn), Some(state)) = (m.button, m.button_state) {
                        stroke.state = match (btn, state) {
                            (MouseButton::Left, KeyState::Down) => INTERCEPTION_MOUSE_LEFT_BUTTON_DOWN,
                            (MouseButton::Left, KeyState::Up) => INTERCEPTION_MOUSE_LEFT_BUTTON_UP,
                            (MouseButton::Right, KeyState::Down) => INTERCEPTION_MOUSE_RIGHT_BUTTON_DOWN,
                            (MouseButton::Right, KeyState::Up) => INTERCEPTION_MOUSE_RIGHT_BUTTON_UP,
                            (MouseButton::Middle, KeyState::Down) => INTERCEPTION_MOUSE_MIDDLE_BUTTON_DOWN,
                            (MouseButton::Middle, KeyState::Up) => INTERCEPTION_MOUSE_MIDDLE_BUTTON_UP,
                            _ => 0,
                        };
                    }
                    
                    strokes.push((device, InterceptionStroke { mouse: stroke }));
                }
                InputEvent::Delay(us) => {
                    tokio::time::sleep(tokio::time::Duration::from_micros(us)).await;
                }
            }
        }
        
        // Send all strokes
        let send_fn: InterceptionSend = unsafe {
            std::mem::transmute(GetProcAddress(self.dll_handle.unwrap(), s!("interception_send")).ok_or_else(|| DriverError::PlatformError("Missing Interception DLL export".into()))?)
        };
        
        for (device, stroke) in strokes {
            let sent = unsafe {
                send_fn(self.context, device, &stroke as *const _ as *const c_void, 1)
            };
            if sent != 1 {
                return Err(DriverError::InjectionFailed("Batch send failed".into()));
            }
        }
        
        Ok(())
    }
    
    async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>> {
        let cache = self.device_cache.lock();
        Ok(cache.iter().map(|d| DeviceInfo {
            id: d.id.to_string(),
            name: d.name.clone(),
            device_type: d.device_type,
            vendor_id: d.vendor_id,
            product_id: d.product_id,
            is_keyboard: d.is_keyboard,
            is_mouse: d.is_mouse,
            is_touch: false,
        }).collect())
    }
    
    async fn set_exclusive(&self, exclusive: bool) -> DriverResult<()> {
        // Interception doesn't support exclusive mode directly
        // Would need to set filter to block all input
        warn!("Exclusive mode not fully supported with Interception");
        Ok(())
    }
    
    async fn shutdown(&mut self) -> DriverResult<()> {
        if let Some(dll) = self.dll_handle {
            if let Some(destroy_context) = unsafe {
                GetProcAddress(dll, s!("interception_destroy_context"))
            } {
                let fn_ptr: InterceptionDestroyContext = unsafe { std::mem::transmute(destroy_context) };
                unsafe { fn_ptr(self.context) };
            }
            let _ = unsafe { FreeLibrary(dll) };
            self.dll_handle = None;
            self.context = std::ptr::null_mut();
        }
        self.initialized = false;
        info!("Interception driver shutdown");
        Ok(())
    }
}

impl Drop for InterceptionDriver {
    fn drop(&mut self) {
        if self.initialized {
            let _ = futures::executor::block_on(self.shutdown());
        }
    }
}

unsafe impl Send for InterceptionDriver {}
unsafe impl Sync for InterceptionDriver {}