//! Common types for the driver

use serde::{Deserialize, Serialize};
use std::fmt;

/// Virtual key codes (Windows-compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum VirtualKey {
    // Mouse buttons (as keys)
    LButton = 0x01,
    RButton = 0x02,
    MButton = 0x04,
    XButton1 = 0x05,
    XButton2 = 0x06,
    
    // Control keys
    Back = 0x08,
    Tab = 0x09,
    Clear = 0x0C,
    Return = 0x0D,
    Shift = 0x10,
    Control = 0x11,
    Menu = 0x12, // Alt
    Pause = 0x13,
    Capital = 0x14,
    Kana = 0x15,
    Junja = 0x17,
    Final = 0x18,
    Hanja = 0x19,
    Escape = 0x1B,
    Convert = 0x1C,
    NonConvert = 0x1D,
    Accept = 0x1E,
    ModeChange = 0x1F,
    Space = 0x20,
    Prior = 0x21, // Page Up
    Next = 0x22,  // Page Down
    End = 0x23,
    Home = 0x24,
    Left = 0x25,
    Up = 0x26,
    Right = 0x27,
    Down = 0x28,
    Select = 0x29,
    Print = 0x2A,
    Execute = 0x2B,
    Snapshot = 0x2C,
    Insert = 0x2D,
    Delete = 0x2E,
    Help = 0x2F,
    
    // Number keys
    Key0 = 0x30,
    Key1 = 0x31,
    Key2 = 0x32,
    Key3 = 0x33,
    Key4 = 0x34,
    Key5 = 0x35,
    Key6 = 0x36,
    Key7 = 0x37,
    Key8 = 0x38,
    Key9 = 0x39,
    
    // Letter keys
    A = 0x41,
    B = 0x42,
    C = 0x43,
    D = 0x44,
    E = 0x45,
    F = 0x46,
    G = 0x47,
    H = 0x48,
    I = 0x49,
    J = 0x4A,
    K = 0x4B,
    L = 0x4C,
    M = 0x4D,
    N = 0x4E,
    O = 0x4F,
    P = 0x50,
    Q = 0x51,
    R = 0x52,
    S = 0x53,
    T = 0x54,
    U = 0x55,
    V = 0x56,
    W = 0x57,
    X = 0x58,
    Y = 0x59,
    Z = 0x5A,
    
    // Windows keys
    LWin = 0x5B,
    RWin = 0x5C,
    Apps = 0x5D,
    Sleep = 0x5F,
    
    // Numpad
    Numpad0 = 0x60,
    Numpad1 = 0x61,
    Numpad2 = 0x62,
    Numpad3 = 0x63,
    Numpad4 = 0x64,
    Numpad5 = 0x65,
    Numpad6 = 0x66,
    Numpad7 = 0x67,
    Numpad8 = 0x68,
    Numpad9 = 0x69,
    Multiply = 0x6A,
    Add = 0x6B,
    Separator = 0x6C,
    Subtract = 0x6D,
    Decimal = 0x6E,
    Divide = 0x6F,
    
    // Function keys
    F1 = 0x70,
    F2 = 0x71,
    F3 = 0x72,
    F4 = 0x73,
    F5 = 0x74,
    F6 = 0x75,
    F7 = 0x76,
    F8 = 0x77,
    F9 = 0x78,
    F10 = 0x79,
    F11 = 0x7A,
    F12 = 0x7B,
    F13 = 0x7C,
    F14 = 0x7D,
    F15 = 0x7E,
    F16 = 0x7F,
    F17 = 0x80,
    F18 = 0x81,
    F19 = 0x82,
    F20 = 0x83,
    F21 = 0x84,
    F22 = 0x85,
    F23 = 0x86,
    F24 = 0x87,
    
    // Lock keys
    NumLock = 0x90,
    Scroll = 0x91,
    
    // Modifier keys (left/right)
    LShift = 0xA0,
    RShift = 0xA1,
    LControl = 0xA2,
    RControl = 0xA3,
    LMenu = 0xA4,
    RMenu = 0xA5,
    
    // Browser keys
    BrowserBack = 0xA6,
    BrowserForward = 0xA7,
    BrowserRefresh = 0xA8,
    BrowserStop = 0xA9,
    BrowserSearch = 0xAA,
    BrowserFavorites = 0xAB,
    BrowserHome = 0xAC,
    
    // Media keys
    VolumeMute = 0xAD,
    VolumeDown = 0xAE,
    VolumeUp = 0xAF,
    MediaNextTrack = 0xB0,
    MediaPrevTrack = 0xB1,
    MediaStop = 0xB2,
    MediaPlayPause = 0xB3,
    LaunchMail = 0xB4,
    LaunchMediaSelect = 0xB5,
    LaunchApp1 = 0xB6,
    LaunchApp2 = 0xB7,
    
    // OEM keys
    Oem1 = 0xBA,      // ;:
    OemPlus = 0xBB,   // +
    OemComma = 0xBC,  // ,
    OemMinus = 0xBD,  // -
    OemPeriod = 0xBE, // .
    Oem2 = 0xBF,      // /?
    Oem3 = 0xC0,      // `~
    Oem4 = 0xDB,      // [{
    Oem5 = 0xDC,      // \|
    Oem6 = 0xDD,      // ]}
    Oem7 = 0xDE,      // '"
    Oem8 = 0xDF,
    Oem102 = 0xE2,    // <> or \|
    
    // IME
    ProcessKey = 0xE5,
    Packet = 0xE7,
    Attn = 0xF6,
    CrSel = 0xF7,
    ExSel = 0xF8,
    ErEof = 0xF9,
    Play = 0xFA,
    Zoom = 0xFB,
    NoName = 0xFC,
    Pa1 = 0xFD,
    OemClear = 0xFE,
}

impl VirtualKey {
    pub fn from_u16(v: u16) -> Option<Self> {
        // This is a simplified version - real impl would use a match
        Some(unsafe { std::mem::transmute(v) })
    }
    
    pub fn as_u16(&self) -> u16 {
        *self as u16
    }
    
    pub fn is_modifier(&self) -> bool {
        matches!(self, 
            VirtualKey::Shift | VirtualKey::Control | VirtualKey::Menu |
            VirtualKey::LShift | VirtualKey::RShift |
            VirtualKey::LControl | VirtualKey::RControl |
            VirtualKey::LMenu | VirtualKey::RMenu |
            VirtualKey::LWin | VirtualKey::RWin
        )
    }
}

impl fmt::Display for VirtualKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Scan codes for keyboard
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanCode(pub u16);

impl ScanCode {
    pub fn new(code: u16) -> Self {
        Self(code)
    }
    
    pub fn is_extended(&self) -> bool {
        self.0 & 0xE000 != 0
    }
    
    pub fn base_code(&self) -> u16 {
        self.0 & 0xFF
    }
}

/// Mouse button representation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl MouseButton {
    pub fn to_interception(&self) -> u16 {
        match self {
            MouseButton::Left => 0x0001,
            MouseButton::Right => 0x0002,
            MouseButton::Middle => 0x0004,
            MouseButton::X1 => 0x0008,
            MouseButton::X2 => 0x0010,
            MouseButton::WheelUp => 0x0400,
            MouseButton::WheelDown => 0x0800,
            MouseButton::WheelLeft => 0x1000,
            MouseButton::WheelRight => 0x2000,
        }
    }
}

/// Input event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InputEvent {
    KeyDown { code: u16, scan: u16, extended: bool },
    KeyUp { code: u16, scan: u16, extended: bool },
    MouseMove { x: i32, y: i32, absolute: bool },
    MouseButtonDown { button: MouseButton },
    MouseButtonUp { button: MouseButton },
    MouseWheel { delta_x: i32, delta_y: i32 },
    Delay { microseconds: u64 },
}

impl InputEvent {
    pub fn key_down(code: u16, scan: u16) -> Self {
        Self::KeyDown { code, scan, extended: false }
    }
    
    pub fn key_up(code: u16, scan: u16) -> Self {
        Self::KeyUp { code, scan, extended: false }
    }
    
    pub fn mouse_move_abs(x: i32, y: i32) -> Self {
        Self::MouseMove { x, y, absolute: true }
    }
    
    pub fn mouse_move_rel(dx: i32, dy: i32) -> Self {
        Self::MouseMove { x: dx, y: dy, absolute: false }
    }
    
    pub fn mouse_down(button: MouseButton) -> Self {
        Self::MouseButtonDown { button }
    }
    
    pub fn mouse_up(button: MouseButton) -> Self {
        Self::MouseButtonUp { button }
    }
    
    pub fn wheel(delta_x: i32, delta_y: i32) -> Self {
        Self::MouseWheel { delta_x, delta_y }
    }
    
    pub fn delay_us(us: u64) -> Self {
        Self::Delay { microseconds: us }
    }
}

/// Device capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCaps {
    pub max_keys: usize,
    pub max_buttons: usize,
    pub has_wheel: bool,
    pub has_h_wheel: bool,
    pub absolute_max_x: i32,
    pub absolute_max_y: i32,
    pub supports_absolute: bool,
    pub supports_relative: bool,
}

impl Default for DeviceCaps {
    fn default() -> Self {
        Self {
            max_keys: 256,
            max_buttons: 8,
            has_wheel: true,
            has_h_wheel: true,
            absolute_max_x: 65535,
            absolute_max_y: 65535,
            supports_absolute: true,
            supports_relative: true,
        }
    }
}

/// Driver capability flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct DriverCaps: u32 {
        const KEYBOARD = 1 << 0;
        const MOUSE = 1 << 1;
        const ABSOLUTE = 1 << 2;
        const RELATIVE = 1 << 3;
        const WHEEL = 1 << 4;
        const H_WHEEL = 1 << 5;
        const EXCLUSIVE = 1 << 6;
        const BATCH = 1 << 7;
        const HID = 1 << 8;
        const TOUCH = 1 << 9;
        const FORCE_FEEDBACK = 1 << 10;
    }
}
#[allow(non_upper_case_globals)]
impl VirtualKey {
    pub const Hangul: Self = Self::Kana;
    pub const Kanji: Self = Self::Hanja;
}
