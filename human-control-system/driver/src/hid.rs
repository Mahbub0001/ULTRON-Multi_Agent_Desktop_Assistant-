//! HID Driver for Hardware Input Injection
//! 
//! Communicates with Arduino/Teensy running QMK/Vial firmware
//! or custom HID firmware for undetectable input injection

use crate::{
    DriverConfig, DriverError, DriverResult, InputDriver, InputEvent,
    KeyboardEvent, MouseEvent, KeyState, MouseButton, DeviceInfo, DeviceType,
};
use async_trait::async_trait;
use serialport::{SerialPort, SerialPortType, UsbPortInfo};
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Duration;
use tracing::{info, warn, error, debug, instrument};
use bytes::{BufMut, BytesMut};

/// HID Report IDs
const REPORT_ID_KEYBOARD: u8 = 0x01;
const REPORT_ID_MOUSE: u8 = 0x02;
const REPORT_ID_CONSUMER: u8 = 0x03;
const REPORT_ID_SYSTEM: u8 = 0x04;
const REPORT_ID_NKRO: u8 = 0x05;

/// Keyboard report (8 bytes: modifier + reserved + 6 keys)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
struct KeyboardReport {
    modifier: u8,
    reserved: u8,
    keys: [u8; 6],
}

/// Mouse report (5 bytes: buttons + x + y + wheel + hwheel)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
struct MouseReport {
    buttons: u8,
    x: i16,
    y: i16,
    wheel: i8,
    hwheel: i8,
}

/// Consumer control report (media keys)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
struct ConsumerReport {
    usage: u16,
}

/// System control report (power, sleep, wake)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
struct SystemReport {
    usage: u8,
}

/// NKRO keyboard report (bitmap)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
struct NKROReport {
    keys: [u8; 32], // 256 bits
}

/// HID Driver for hardware injection
pub struct HIDDriver {
    config: DriverConfig,
    port: Mutex<Option<Box<dyn SerialPort>>>,
    port_path: Option<String>,
    initialized: bool,
    keyboard_state: Arc<Mutex<KeyboardReport>>,
    mouse_state: Arc<Mutex<MouseReport>>,
    vendor_id: u16,
    product_id: u16,
}

impl HIDDriver {
    /// Create new HID driver
    pub async fn new(config: DriverConfig) -> DriverResult<Self> {
        info!("Initializing HID driver...");
        
        Ok(Self {
            config,
            port: Mutex::new(None),
            port_path: None,
            initialized: false,
            keyboard_state: Arc::new(Mutex::new(KeyboardReport::default())),
            mouse_state: Arc::new(Mutex::new(MouseReport::default())),
            vendor_id: 0x16C0, // Default Teensy VID
            product_id: 0x0486, // Default Teensy PID
        })
    }
    
    /// Auto-detect HID device
    pub async fn auto_detect(&mut self) -> DriverResult<String> {
        let ports = serialport::available_ports()?;
        
        for port in ports {
            if let SerialPortType::UsbPort(info) = &port.port_type {
                // Check for known VID/PID combinations
                if Self::is_hid_device(info) {
                    info!("Found HID device: {} ({:04X}:{:04X})", 
                          port.port_name, info.vid, info.pid);
                    self.vendor_id = info.vid;
                    self.product_id = info.pid;
                    return Ok(port.port_name.clone());
                }
            }
        }
        
        Err(DriverError::DeviceNotFound("No HID device found".into()))
    }
    
    fn is_hid_device(info: &UsbPortInfo) -> bool {
        // Known VID/PID for common HID devices
        matches!(
            (info.vid, info.pid),
            // Teensy
            (0x16C0, 0x0486) | (0x16C0, 0x0487) | (0x16C0, 0x0488) |
            // Arduino Leonardo/Micro
            (0x2341, 0x8036) | (0x2341, 0x8037) | (0x2341, 0x0036) |
            // Arduino Pro Micro
            (0x2341, 0x0044) | (0x2341, 0x0045) |
            // QMK/Vial devices
            (0xFEED, _) | (0xBEEF, _) |
            // Generic HID
            (0x1D50, 0x606F) | // OpenMoko
            // Custom
            (0x1209, _) // pid.codes
        )
    }
    
    /// Connect to specific port
    pub async fn connect(&mut self, port_path: &str) -> DriverResult<()> {
        let mut port = serialport::new(port_path, 115200)
            .timeout(Duration::from_millis(100))
            .open()?;
        
        // Configure port
        port.set_data_bits(serialport::DataBits::Eight)?;
        port.set_parity(serialport::Parity::None)?;
        port.set_stop_bits(serialport::StopBits::One)?;
        port.set_flow_control(serialport::FlowControl::None)?;
        
        // Wait for device to settle
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Send handshake
        self.handshake(&mut port).await?;
        
        *self.port.lock() = Some(port);
        self.port_path = Some(port_path.to_string());
        self.initialized = true;
        
        info!("Connected to HID device at {}", port_path);
        Ok(())
    }
    
    async fn handshake(&self, port: &mut Box<dyn SerialPort>) -> DriverResult<()> {
        // Send magic bytes to identify protocol
        let handshake = b"HCS_HID_v1\n";
        port.write_all(handshake)?;
        port.flush()?;
        
        // Read response
        let mut buf = [0u8; 32];
        tokio::time::sleep(Duration::from_millis(50)).await;
        let n = port.read(&mut buf)?;
        
        let response = String::from_utf8_lossy(&buf[..n]);
        if !response.contains("HCS_ACK") {
            return Err(DriverError::HardwareError(
                format!("Handshake failed: {}", response)
            ));
        }
        
        debug!("Handshake successful: {}", response.trim());
        Ok(())
    }
    
    fn send_report(&self, report_id: u8, data: &[u8]) -> DriverResult<()> {
        let mut port_guard = self.port.lock();
        let port = port_guard.as_mut().ok_or(DriverError::NotInitialized)?;
        
        // Build packet: [report_id][data...]
        let mut packet = BytesMut::with_capacity(1 + data.len());
        packet.put_u8(report_id);
        packet.put_slice(data);
        
        port.write_all(&packet)?;
        port.flush()?;
        
        Ok(())
    }
}

#[async_trait]
impl InputDriver for HIDDriver {
    async fn initialize(&mut self) -> DriverResult<()> {
        if self.initialized {
            return Err(DriverError::AlreadyInitialized);
        }
        
        // Try auto-detect if no port specified
        if self.port_path.is_none() {
            let port_path = self.auto_detect().await?;
            self.connect(&port_path).await?;
        } else if let Some(path) = self.port_path.clone() {
            self.connect(&path).await?;
        } else {
            return Err(DriverError::ConfigError("No HID device specified".into()));
        }
        
        Ok(())
    }
    
    fn is_ready(&self) -> bool {
        self.initialized && self.port.lock().is_some()
    }
    
    #[instrument(skip(self))]
    async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        let mut state = self.keyboard_state.lock();
        
        let key_code = Self::virtual_key_to_hid(event.code);
        
        match event.state {
            KeyState::Down => {
                // Add key to first empty slot
                for slot in &mut state.keys {
                    if *slot == 0 {
                        *slot = key_code;
                        break;
                    }
                }
            }
            KeyState::Up => {
                // Remove key
                for slot in &mut state.keys {
                    if *slot == key_code {
                        *slot = 0;
                        break;
                    }
                }
            }
        }
        
        // Handle modifiers
        match event.code {
            0x10 | 0xA0 | 0xA1 => state.modifier |= 0x02, // Shift
            0x11 | 0xA2 | 0xA3 => state.modifier |= 0x01, // Ctrl
            0x12 | 0xA4 | 0xA5 => state.modifier |= 0x04, // Alt
            0x5B | 0x5C => state.modifier |= 0x08,        // GUI/Win
            _ => {}
        }
        
        if matches!(event.state, KeyState::Up) {
            match event.code {
                0x10 | 0xA0 | 0xA1 => state.modifier &= !0x02,
                0x11 | 0xA2 | 0xA3 => state.modifier &= !0x01,
                0x12 | 0xA4 | 0xA5 => state.modifier &= !0x04,
                0x5B | 0x5C => state.modifier &= !0x08,
                _ => {}
            }
        }
        
        let mut data = [0u8; 8];
        data[0] = state.modifier;
        data[2..].copy_from_slice(&state.keys);
        self.send_report(REPORT_ID_KEYBOARD, &data)

    }
    
    #[instrument(skip(self))]
    async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()> {
        let mut state = self.mouse_state.lock();
        
        if event.absolute {
            // Convert to relative for HID (most firmware expects relative)
            // Or send absolute if firmware supports it
            state.x = event.x as i16;
            state.y = event.y as i16;
        } else {
            state.x = event.dx as i16;
            state.y = event.dy as i16;
        }
        
        if let (Some(button), Some(key_state)) = (event.button, event.button_state) {
            let bit = match button {
                MouseButton::Left => 0x01,
                MouseButton::Right => 0x02,
                MouseButton::Middle => 0x04,
                MouseButton::X1 => 0x08,
                MouseButton::X2 => 0x10,
                _ => 0,
            };
            
            match key_state {
                KeyState::Down => state.buttons |= bit,
                KeyState::Up => state.buttons &= !bit,
            }
        }
        
        let mut data = Vec::with_capacity(7);
        data.push(state.buttons);
        data.extend_from_slice(&state.x.to_le_bytes());
        data.extend_from_slice(&state.y.to_le_bytes());
        data.push(state.wheel as u8);
        data.push(state.hwheel as u8);
        self.send_report(REPORT_ID_MOUSE, &data)

    }
    
    #[instrument(skip(self))]
    async fn inject_batch(&self, events: Vec<InputEvent>) -> DriverResult<()> {
        for event in events {
            match event {
                InputEvent::Keyboard(k) => self.inject_keyboard(k).await?,
                InputEvent::Mouse(m) => self.inject_mouse(m).await?,
                InputEvent::Delay(us) => {
                    tokio::time::sleep(tokio::time::Duration::from_micros(us)).await;
                }
            }
        }
        Ok(())
    }
    
    async fn get_devices(&self) -> DriverResult<Vec<DeviceInfo>> {
        Ok(vec![
            DeviceInfo {
                id: format!("hid-{:04X}:{:04X}-kbd", self.vendor_id, self.product_id),
                name: "HCS HID Keyboard".into(),
                device_type: DeviceType::Keyboard,
                vendor_id: self.vendor_id,
                product_id: self.product_id,
                is_keyboard: true,
                is_mouse: false,
                is_touch: false,
            },
            DeviceInfo {
                id: format!("hid-{:04X}:{:04X}-mouse", self.vendor_id, self.product_id),
                name: "HCS HID Mouse".into(),
                device_type: DeviceType::Mouse,
                vendor_id: self.vendor_id,
                product_id: self.product_id,
                is_keyboard: false,
                is_mouse: true,
                is_touch: false,
            },
        ])
    }
    
    async fn set_exclusive(&self, _exclusive: bool) -> DriverResult<()> {
        // HID devices are naturally exclusive
        Ok(())
    }
    
    async fn shutdown(&mut self) -> DriverResult<()> {
        // Release all keys
        {
            let mut state = self.keyboard_state.lock();
            state.keys = [0; 6];
            state.modifier = 0;
        }
        
        {
            let mut state = self.mouse_state.lock();
            state.buttons = 0;
            state.x = 0;
            state.y = 0;
            state.wheel = 0;
            state.hwheel = 0;
        }
        
        if self.port.lock().is_some() {
            self.send_report(REPORT_ID_KEYBOARD, &[0; 8])?;
            self.send_report(REPORT_ID_MOUSE, &[0; 7])?;
        }
        
        if let Some(mut port) = self.port.lock().take() {
            let _ = port.flush();
        }
        
        self.initialized = false;
        info!("HID driver shutdown");
        Ok(())
    }
}

impl HIDDriver {
    fn virtual_key_to_hid(vk: u16) -> u8 {
        // HID Usage IDs (USB HID Usage Tables)
        match vk {
            0x08 => 0x2A, // Backspace
            0x09 => 0x2B, // Tab
            0x0D => 0x28, // Enter
            0x10 => 0xE1, // Left Shift
            0x11 => 0xE0, // Left Control
            0x12 => 0xE2, // Left Alt
            0x13 => 0x48, // Pause
            0x14 => 0x39, // Caps Lock
            0x1B => 0x29, // Escape
            0x20 => 0x2C, // Space
            0x21 => 0x4B, // Page Up
            0x22 => 0x4E, // Page Down
            0x23 => 0x4D, // End
            0x24 => 0x4A, // Home
            0x25 => 0x50, // Left Arrow
            0x26 => 0x52, // Up Arrow
            0x27 => 0x4F, // Right Arrow
            0x28 => 0x51, // Down Arrow
            0x2C => 0x46, // Print Screen
            0x2D => 0x49, // Insert
            0x2E => 0x4C, // Delete
            0x30..=0x39 => (0x27 + (vk - 0x30)) as u8, // 0-9
            0x41..=0x5A => (0x04 + (vk - 0x41)) as u8, // A-Z
            0x5B => 0xE3, // Left GUI
            0x5C => 0xE7, // Right GUI
            0x5D => 0x65, // Application
            0x60..=0x69 => (0x59 + (vk - 0x60)) as u8, // Numpad 0-9
            0x6A => 0x55, // Numpad *
            0x6B => 0x57, // Numpad +
            0x6D => 0x56, // Numpad -
            0x6E => 0x63, // Numpad .
            0x6F => 0x54, // Numpad /
            0x70..=0x87 => (0x3A + (vk - 0x70)) as u8, // F1-F24
            0x90 => 0x53, // Num Lock
            0x91 => 0x47, // Scroll Lock
            0xA0 => 0xE1, // Left Shift
            0xA1 => 0xE5, // Right Shift
            0xA2 => 0xE0, // Left Control
            0xA3 => 0xE4, // Right Control
            0xA4 => 0xE2, // Left Alt
            0xA5 => 0xE6, // Right Alt
            0xBA => 0x33, // ;:
            0xBB => 0x30, // +
            0xBC => 0x36, // ,
            0xBD => 0x38, // -
            0xBE => 0x37, // .
            0xBF => 0x38, // /?
            0xC0 => 0x35, // `~
            0xDB => 0x2F, // [{
            0xDC => 0x31, // \|
            0xDD => 0x30, // ]}
            0xDE => 0x34, // '"
            _ => 0x00,
        }
    }
}

impl Drop for HIDDriver {
    fn drop(&mut self) {
        if self.initialized {
            let _ = futures::executor::block_on(self.shutdown());
        }
    }
}