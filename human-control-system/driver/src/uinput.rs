//! Linux uinput Driver Implementation
//! 
//! Uses Linux uinput kernel module for input injection
//! Requires /dev/uinput access (usually root or uinput group)

use crate::{
    DriverConfig, DriverError, DriverResult, InputDriver, InputEvent,
    KeyboardEvent, MouseEvent, KeyState, MouseButton, DeviceInfo, DeviceType,
    BackendType
};
use async_trait::async_trait;
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
use nix::unistd::{close, write};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::io::RawFd;
use std::sync::Arc;
use parking_lot::Mutex;
use tracing::{info, warn, error, debug, instrument};
use libc::{c_int, c_ulong, c_void, ioctl};

// uinput constants
const UINPUT_MAX_NAME_SIZE: usize = 80;
const UINPUT_DEV_CREATE: c_ulong = 0x5501;
const UINPUT_DEV_DESTROY: c_ulong = 0x5502;

// UI_SET_* ioctls
const UI_SET_EVBIT: c_ulong = 0x40045564;
const UI_SET_KEYBIT: c_ulong = 0x40045565;
const UI_SET_RELBIT: c_ulong = 0x40045566;
const UI_SET_ABSBIT: c_ulong = 0x40045567;
const UI_SET_MSCBIT: c_ulong = 0x40045568;
const UI_SET_LEDBIT: c_ulong = 0x40045569;
const UI_SET_SNDBIT: c_ulong = 0x4004556a;
const UI_SET_FFBIT: c_ulong = 0x4004556b;
const UI_SET_PHYS: c_ulong = 0x8030556c;
const UI_SET_SWBIT: c_ulong = 0x4004556d;
const UI_SET_PROPBIT: c_ulong = 0x4004556e;

// Event types
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const EV_MSC: u16 = 0x04;
const EV_SW: u16 = 0x05;
const EV_LED: u16 = 0x11;
const EV_SND: u16 = 0x12;
const EV_REP: u16 = 0x14;
const EV_FF: u16 = 0x15;
const EV_PWR: u16 = 0x16;
const EV_FF_STATUS: u16 = 0x17;

// Key codes (Linux input.h subset)
const KEY_RESERVED: u16 = 0;
const KEY_ESC: u16 = 1;
const KEY_1: u16 = 2;
const KEY_2: u16 = 3;
const KEY_3: u16 = 4;
const KEY_4: u16 = 5;
const KEY_5: u16 = 6;
const KEY_6: u16 = 7;
const KEY_7: u16 = 8;
const KEY_8: u16 = 9;
const KEY_9: u16 = 10;
const KEY_0: u16 = 11;
const KEY_MINUS: u16 = 12;
const KEY_EQUAL: u16 = 13;
const KEY_BACKSPACE: u16 = 14;
const KEY_TAB: u16 = 15;
const KEY_Q: u16 = 16;
const KEY_W: u16 = 17;
const KEY_E: u16 = 18;
const KEY_R: u16 = 19;
const KEY_T: u16 = 20;
const KEY_Y: u16 = 21;
const KEY_U: u16 = 22;
const KEY_I: u16 = 23;
const KEY_O: u16 = 24;
const KEY_P: u16 = 25;
const KEY_LEFTBRACE: u16 = 26;
const KEY_RIGHTBRACE: u16 = 27;
const KEY_ENTER: u16 = 28;
const KEY_LEFTCTRL: u16 = 29;
const KEY_A: u16 = 30;
const KEY_S: u16 = 31;
const KEY_D: u16 = 32;
const KEY_F: u16 = 33;
const KEY_G: u16 = 34;
const KEY_H: u16 = 35;
const KEY_J: u16 = 36;
const KEY_K: u16 = 37;
const KEY_L: u16 = 38;
const KEY_SEMICOLON: u16 = 39;
const KEY_APOSTROPHE: u16 = 40;
const KEY_GRAVE: u16 = 41;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_BACKSLASH: u16 = 43;
const KEY_Z: u16 = 44;
const KEY_X: u16 = 45;
const KEY_C: u16 = 46;
const KEY_V: u16 = 47;
const KEY_B: u16 = 48;
const KEY_N: u16 = 49;
const KEY_M: u16 = 50;
const KEY_COMMA: u16 = 51;
const KEY_DOT: u16 = 52;
const KEY_SLASH: u16 = 53;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_KPASTERISK: u16 = 55;
const KEY_LEFTALT: u16 = 56;
const KEY_SPACE: u16 = 57;
const KEY_CAPSLOCK: u16 = 58;
const KEY_F1: u16 = 59;
const KEY_F2: u16 = 60;
const KEY_F3: u16 = 61;
const KEY_F4: u16 = 62;
const KEY_F5: u16 = 63;
const KEY_F6: u16 = 64;
const KEY_F7: u16 = 65;
const KEY_F8: u16 = 66;
const KEY_F9: u16 = 67;
const KEY_F10: u16 = 68;
const KEY_F11: u16 = 69;
const KEY_F12: u16 = 88;
const KEY_NUMLOCK: u16 = 69;
const KEY_SCROLLLOCK: u16 = 70;
const KEY_KP7: u16 = 71;
const KEY_KP8: u16 = 72;
const KEY_KP9: u16 = 73;
const KEY_KPMINUS: u16 = 74;
const KEY_KP4: u16 = 75;
const KEY_KP5: u16 = 76;
const KEY_KP6: u16 = 77;
const KEY_KPPLUS: u16 = 78;
const KEY_KP1: u16 = 79;
const KEY_KP2: u16 = 80;
const KEY_KP3: u16 = 81;
const KEY_KP0: u16 = 82;
const KEY_KPDOT: u16 = 83;
const KEY_ZENKAKUHANKAKU: u16 = 85;
const KEY_102ND: u16 = 86;
const KEY_F13: u16 = 87;
const KEY_F14: u16 = 88;
const KEY_F15: u16 = 89;
const KEY_F16: u16 = 90;
const KEY_F17: u16 = 91;
const KEY_F18: u16 = 92;
const KEY_F19: u16 = 93;
const KEY_F20: u16 = 94;
const KEY_F21: u16 = 95;
const KEY_F22: u16 = 96;
const KEY_F23: u16 = 97;
const KEY_F24: u16 = 98;
const KEY_KPENTER: u16 = 96;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_KPSLASH: u16 = 98;
const KEY_SYSRQ: u16 = 99;
const KEY_RIGHTALT: u16 = 100;
const KEY_LINEFEED: u16 = 101;
const KEY_HOME: u16 = 102;
const KEY_UP: u16 = 103;
const KEY_PAGEUP: u16 = 104;
const KEY_LEFT: u16 = 105;
const KEY_RIGHT: u16 = 106;
const KEY_END: u16 = 107;
const KEY_DOWN: u16 = 108;
const KEY_PAGEDOWN: u16 = 109;
const KEY_INSERT: u16 = 110;
const KEY_DELETE: u16 = 111;
const KEY_MACRO: u16 = 112;
const KEY_MUTE: u16 = 113;
const KEY_VOLUMEDOWN: u16 = 114;
const KEY_VOLUMEUP: u16 = 115;
const KEY_POWER: u16 = 116;
const KEY_KPEQUAL: u16 = 117;
const KEY_KPPLUSMINUS: u16 = 118;
const KEY_PAUSE: u16 = 119;
const KEY_SCALE: u16 = 120;
const KEY_KPCOMMA: u16 = 121;
const KEY_HANGEUL: u16 = 122;
const KEY_HANJA: u16 = 123;
const KEY_YEN: u16 = 124;
const KEY_LEFTMETA: u16 = 125;
const KEY_RIGHTMETA: u16 = 126;
const KEY_COMPOSE: u16 = 127;
const KEY_STOP: u16 = 128;
const KEY_AGAIN: u16 = 129;
const KEY_PROPS: u16 = 130;
const KEY_UNDO: u16 = 131;
const KEY_FRONT: u16 = 132;
const KEY_COPY: u16 = 133;
const KEY_OPEN: u16 = 134;
const KEY_PASTE: u16 = 135;
const KEY_FIND: u16 = 136;
const KEY_CUT: u16 = 137;
const KEY_HELP: u16 = 138;
const KEY_MENU: u16 = 139;
const KEY_CALC: u16 = 140;
const KEY_SETUP: u16 = 141;
const KEY_SLEEP: u16 = 142;
const KEY_WAKEUP: u16 = 143;
const KEY_FILE: u16 = 144;
const KEY_SENDFILE: u16 = 145;
const KEY_DELETEFILE: u16 = 146;
const KEY_XFER: u16 = 147;
const KEY_PROG1: u16 = 148;
const KEY_PROG2: u16 = 149;
const KEY_WWW: u16 = 150;
const KEY_MSDOS: u16 = 151;
const KEY_COFFEE: u16 = 152;
const KEY_SCREENLOCK: u16 = 153;
const KEY_ROTATE_DISPLAY: u16 = 154;
const KEY_CYCLEWINDOWS: u16 = 155;
const KEY_MAIL: u16 = 156;
const KEY_BOOKMARKS: u16 = 157;
const KEY_COMPUTER: u16 = 158;
const KEY_BACK: u16 = 159;
const KEY_FORWARD: u16 = 160;
const KEY_CLOSECD: u16 = 161;
const KEY_EJECTCD: u16 = 162;
const KEY_EJECTCLOSECD: u16 = 163;
const KEY_NEXTSONG: u16 = 164;
const KEY_PLAYPAUSE: u16 = 165;
const KEY_PREVIOUSSONG: u16 = 166;
const KEY_STOPCD: u16 = 167;
const KEY_RECORD: u16 = 168;
const KEY_REWIND: u16 = 169;
const KEY_PHONE: u16 = 170;
const KEY_ISO: u16 = 171;
const KEY_CONFIG: u16 = 172;
const KEY_HOMEPAGE: u16 = 173;
const KEY_REFRESH: u16 = 174;
const KEY_EXIT: u16 = 175;
const KEY_MOVE: u16 = 176;
const KEY_EDIT: u16 = 177;
const KEY_SCROLLUP: u16 = 178;
const KEY_SCROLLDOWN: u16 = 179;
const KEY_KPLEFTPAREN: u16 = 180;
const KEY_KPRIGHTPAREN: u16 = 181;
const KEY_NEW: u16 = 182;
const KEY_REDO: u16 = 183;

// Relative axes
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const REL_Z: u16 = 0x02;
const REL_HWHEEL: u16 = 0x06;
const REL_WHEEL: u16 = 0x08;
const REL_WHEEL_HI_RES: u16 = 0x0b;

// Absolute axes
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_THROTTLE: u16 = 0x06;
const ABS_RUDDER: u16 = 0x07;
const ABS_WHEEL: u16 = 0x08;
const ABS_GAS: u16 = 0x09;
const ABS_BRAKE: u16 = 0x0a;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;
const ABS_HAT3X: u16 = 0x16;
const ABS_HAT3Y: u16 = 0x17;
const ABS_PRESSURE: u16 = 0x18;
const ABS_DISTANCE: u16 = 0x19;
const ABS_TILT_X: u16 = 0x1a;
const ABS_TILT_Y: u16 = 0x1b;
const ABS_TOOL_WIDTH: u16 = 0x1c;
const ABS_VOLUME: u16 = 0x20;
const ABS_MISC: u16 = 0x28;
const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
const ABS_MT_TOUCH_MINOR: u16 = 0x31;
const ABS_MT_WIDTH_MAJOR: u16 = 0x32;
const ABS_MT_WIDTH_MINOR: u16 = 0x33;
const ABS_MT_ORIENTATION: u16 = 0x34;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TOOL_TYPE: u16 = 0x37;
const ABS_MT_BLOB_ID: u16 = 0x38;
const ABS_MT_TRACKING_ID: u16 = 0x39;
const ABS_MT_PRESSURE: u16 = 0x3a;
const ABS_MT_DISTANCE: u16 = 0x3b;
const ABS_MT_TOOL_X: u16 = 0x3c;
const ABS_MT_TOOL_Y: u16 = 0x3d;

// Synchronization
const SYN_REPORT: u16 = 0;
const SYN_CONFIG: u16 = 1;
const SYN_MT_REPORT: u16 = 2;
const SYN_DROPPED: u16 = 3;

// Input event structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

// uinput setup structure
#[repr(C)]
struct UInputSetup {
    name: [u8; UINPUT_MAX_NAME_SIZE],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; 64],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

/// uinput driver
pub struct UInputDriver {
    config: DriverConfig,
    fd: Option<RawFd>,
    initialized: bool,
    abs_x: i32,
    abs_y: i32,
    screen_width: i32,
    screen_height: i32,
}

impl UInputDriver {
    pub async fn new(config: DriverConfig) -> DriverResult<Self> {
        info!("Initializing uinput driver...");
        
        // Get screen dimensions for absolute positioning
        let (screen_width, screen_height) = Self::get_screen_size().unwrap_or((1920, 1080));
        
        Ok(Self {
            config,
            fd: None,
            initialized: false,
            abs_x: 0,
            abs_y: 0,
            screen_width,
            screen_height,
        })
    }
    
    fn get_screen_size() -> Option<(i32, i32)> {
        // Try X11
        if let Ok(output) = std::process::Command::new("xrandr")
            .args(["--current"])
            .output() 
        {
            let out = String::from_utf8_lossy(&output.stdout);
            for line in out.lines() {
                if line.contains('*') && line.contains('x') {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(res) = parts.first() {
                        if let Some((w, h)) = res.split_once('x') {
                            if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                                return Some((w, h));
                            }
                        }
                    }
                }
            }
        }
        
        // Try Wayland via wlr-randr or similar
        None
    }
    
    fn open_uinput(&mut self) -> DriverResult<RawFd> {
        let paths = ["/dev/uinput", "/dev/input/uinput", "/dev/misc/uinput"];
        
        for path in paths {
            match open(path, OFlag::O_WRONLY | OFlag::O_NONBLOCK, Mode::empty()) {
                Ok(fd) => return Ok(fd),
                Err(e) => {
                    debug!("Failed to open {}: {}", path, e);
                }
            }
        }
        
        Err(DriverError::PermissionDenied(
            "Cannot open uinput device. Ensure /dev/uinput exists and you have permissions (add user to 'input' or 'uinput' group)".into()
        ))
    }
    
    fn setup_device(&self, fd: RawFd) -> DriverResult<()> {
        let name = b"HCS Virtual Input Device\0";
        
        // Enable event types
        self.ioctl(fd, UI_SET_EVBIT, EV_SYN as c_ulong)?;
        self.ioctl(fd, UI_SET_EVBIT, EV_KEY as c_ulong)?;
        self.ioctl(fd, UI_SET_EVBIT, EV_REL as c_ulong)?;
        self.ioctl(fd, UI_SET_EVBIT, EV_ABS as c_ulong)?;
        self.ioctl(fd, UI_SET_EVBIT, EV_MSC as c_ulong)?;
        
        // Enable all keys
        for code in 0..=KEY_MAX {
            self.ioctl(fd, UI_SET_KEYBIT, code as c_ulong)?;
        }
        
        // Enable relative axes
        self.ioctl(fd, UI_SET_RELBIT, REL_X as c_ulong)?;
        self.ioctl(fd, UI_SET_RELBIT, REL_Y as c_ulong)?;
        self.ioctl(fd, UI_SET_RELBIT, REL_WHEEL as c_ulong)?;
        self.ioctl(fd, UI_SET_RELBIT, REL_HWHEEL as c_ulong)?;
        
        // Enable absolute axes
        self.ioctl(fd, UI_SET_ABSBIT, ABS_X as c_ulong)?;
        self.ioctl(fd, UI_SET_ABSBIT, ABS_Y as c_ulong)?;
        
        // Create device
        let mut setup = UInputSetup {
            name: [0; UINPUT_MAX_NAME_SIZE],
            id: InputId {
                bustype: 0x03, // BUS_USB
                vendor: 0x1234,
                product: 0x5678,
                version: 0x0100,
            },
            ff_effects_max: 0,
            absmax: [0; 64],
        };
        
        // Copy name
        for (i, &b) in name.iter().enumerate() {
            if i < UINPUT_MAX_NAME_SIZE {
                setup.name[i] = b;
            }
        }
        
        // Set absolute max values
        setup.absmax[ABS_X as usize] = self.screen_width;
        setup.absmax[ABS_Y as usize] = self.screen_height;
        
        // Write setup
        unsafe {
            let ret = libc::write(
                fd,
                &setup as *const _ as *const c_void,
                std::mem::size_of::<UInputSetup>()
            );
            if ret < 0 {
                return Err(DriverError::PlatformError(
                    format!("Failed to write uinput setup: {}", std::io::Error::last_os_error())
                ));
            }
        }
        
        // Create device
        self.ioctl(fd, UINPUT_DEV_CREATE, 0)?;
        
        Ok(())
    }
    
    fn ioctl(&self, fd: RawFd, request: c_ulong, arg: c_ulong) -> DriverResult<()> {
        let ret = unsafe { libc::ioctl(fd, request, arg) };
        if ret < 0 {
            return Err(DriverError::PlatformError(
                format!("ioctl 0x{:x} failed: {}", request, std::io::Error::last_os_error())
            ));
        }
        Ok(())
    }
    
    fn write_event(&self, fd: RawFd, type_: u16, code: u16, value: i32) -> DriverResult<()> {
        let event = InputEvent {
            time: libc::timeval { tv_sec: 0, tv_usec: 0 },
            type_,
            code,
            value,
        };
        
        let bytes = unsafe {
            std::slice::from_raw_parts(
                &event as *const _ as *const u8,
                std::mem::size_of::<InputEvent>()
            )
        };
        
        match write(fd, bytes) {
            Ok(_) => Ok(()),
            Err(e) => Err(DriverError::PlatformError(format!("write failed: {}", e))),
        }
    }
    
    fn syn(&self, fd: RawFd) -> DriverResult<()> {
        self.write_event(fd, EV_SYN, SYN_REPORT, 0)
    }
    
    fn virtual_key_to_linux(&self, vk: u16) -> u16 {
        // Map Windows virtual keys to Linux keycodes
        match vk {
            0x08 => KEY_BACKSPACE,
            0x09 => KEY_TAB,
            0x0D => KEY_ENTER,
            0x10 => KEY_LEFTSHIFT,
            0x11 => KEY_LEFTCTRL,
            0x12 => KEY_LEFTALT,
            0x13 => KEY_PAUSE,
            0x14 => KEY_CAPSLOCK,
            0x1B => KEY_ESC,
            0x20 => KEY_SPACE,
            0x21 => KEY_PAGEUP,
            0x22 => KEY_PAGEDOWN,
            0x23 => KEY_END,
            0x24 => KEY_HOME,
            0x25 => KEY_LEFT,
            0x26 => KEY_UP,
            0x27 => KEY_RIGHT,
            0x28 => KEY_DOWN,
            0x2C => KEY_SNAPSHOT, // Print Screen
            0x2D => KEY_INSERT,
            0x2E => KEY_DELETE,
            0x30..=0x39 => KEY_0 + (vk - 0x30), // 0-9
            0x41..=0x5A => KEY_A + (vk - 0x41), // A-Z
            0x5B => KEY_LEFTMETA,
            0x5C => KEY_RIGHTMETA,
            0x5D => KEY_COMPOSE,
            0x60..=0x69 => KEY_KP0 + (vk - 0x60), // Numpad 0-9
            0x6A => KEY_KPASTERISK,
            0x6B => KEY_KPPLUS,
            0x6D => KEY_KPMINUS,
            0x6E => KEY_KPDOT,
            0x6F => KEY_KPSLASH,
            0x70..=0x87 => KEY_F1 + (vk - 0x70), // F1-F24
            0x90 => KEY_NUMLOCK,
            0x91 => KEY_SCROLLLOCK,
            0xA0 => KEY_LEFTSHIFT,
            0xA1 => KEY_RIGHTSHIFT,
            0xA2 => KEY_LEFTCTRL,
            0xA3 => KEY_RIGHTCTRL,
            0xA4 => KEY_LEFTALT,
            0xA5 => KEY_RIGHTALT,
            0xBA => KEY_SEMICOLON,
            0xBB => KEY_EQUAL,
            0xBC => KEY_COMMA,
            0xBD => KEY_MINUS,
            0xBE => KEY_DOT,
            0xBF => KEY_SLASH,
            0xC0 => KEY_GRAVE,
            0xDB => KEY_LEFTBRACE,
            0xDC => KEY_BACKSLASH,
            0xDD => KEY_RIGHTBRACE,
            0xDE => KEY_APOSTROPHE,
            _ => KEY_RESERVED,
        }
    }
}

#[async_trait]
impl InputDriver for UInputDriver {
    async fn initialize(&mut self) -> DriverResult<()> {
        if self.initialized {
            return Err(DriverError::AlreadyInitialized);
        }
        
        let fd = self.open_uinput()?;
        self.setup_device(fd)?;
        
        self.fd = Some(fd);
        self.initialized = true;
        
        info!("uinput driver initialized successfully");
        Ok(())
    }
    
    fn is_ready(&self) -> bool {
        self.initialized && self.fd.is_some()
    }
    
    #[instrument(skip(self))]
    async fn inject_keyboard(&self, event: KeyboardEvent) -> DriverResult<()> {
        let fd = self.fd.ok_or(DriverError::NotInitialized)?;
        let code = self.virtual_key_to_linux(event.code);
        
        if code == KEY_RESERVED {
            return Err(DriverError::InvalidParameter(format!("Unknown key code: {}", event.code)));
        }
        
        let value = match event.state {
            KeyState::Down => 1,
            KeyState::Up => 0,
        };
        
        self.write_event(fd, EV_KEY, code, value)?;
        self.syn(fd)?;
        
        if self.config.injection_delay_us > 0 {
            tokio::time::sleep(tokio::time::Duration::from_micros(self.config.injection_delay_us)).await;
        }
        
        Ok(())
    }
    
    #[instrument(skip(self))]
    async fn inject_mouse(&self, event: MouseEvent) -> DriverResult<()> {
        let fd = self.fd.ok_or(DriverError::NotInitialized)?;
        
        if event.absolute {
            // Absolute positioning
            let x = event.x.clamp(0, self.screen_width);
            let y = event.y.clamp(0, self.screen_height);
            
            self.abs_x = x;
            self.abs_y = y;
            
            self.write_event(fd, EV_ABS, ABS_X, x)?;
            self.write_event(fd, EV_ABS, ABS_Y, y)?;
        } else {
            // Relative movement
            self.write_event(fd, EV_REL, REL_X, event.dx)?;
            self.write_event(fd, EV_REL, REL_Y, event.dy)?;
            
            self.abs_x = (self.abs_x + event.dx).clamp(0, self.screen_width);
            self.abs_y = (self.abs_y + event.dy).clamp(0, self.screen_height);
        }
        
        // Handle button
        if let (Some(button), Some(state)) = (event.button, event.button_state) {
            let (code, value) = match (button, state) {
                (MouseButton::Left, KeyState::Down) => (0x110, 1),   // BTN_LEFT
                (MouseButton::Left, KeyState::Up) => (0x110, 0),
                (MouseButton::Right, KeyState::Down) => (0x111, 1),  // BTN_RIGHT
                (MouseButton::Right, KeyState::Up) => (0x111, 0),
                (MouseButton::Middle, KeyState::Down) => (0x112, 1), // BTN_MIDDLE
                (MouseButton::Middle, KeyState::Up) => (0x112, 0),
                (MouseButton::X1, KeyState::Down) => (0x113, 1),     // BTN_SIDE
                (MouseButton::X1, KeyState::Up) => (0x113, 0),
                (MouseButton::X2, KeyState::Down) => (0x114, 1),     // BTN_EXTRA
                (MouseButton::X2, KeyState::Up) => (0x114, 0),
                (MouseButton::WheelUp, _) => {
                    self.write_event(fd, EV_REL, REL_WHEEL, 1)?;
                    self.syn(fd)?;
                    return Ok(());
                }
                (MouseButton::WheelDown, _) => {
                    self.write_event(fd, EV_REL, REL_WHEEL, -1)?;
                    self.syn(fd)?;
                    return Ok(());
                }
                (MouseButton::WheelLeft, _) => {
                    self.write_event(fd, EV_REL, REL_HWHEEL, -1)?;
                    self.syn(fd)?;
                    return Ok(());
                }
                (MouseButton::WheelRight, _) => {
                    self.write_event(fd, EV_REL, REL_HWHEEL, 1)?;
                    self.syn(fd)?;
                    return Ok(());
                }
            };
            
            self.write_event(fd, EV_KEY, code, value)?;
        }
        
        self.syn(fd)?;
        
        if self.config.injection_delay_us > 0 {
            tokio::time::sleep(tokio::time::Duration::from_micros(self.config.injection_delay_us)).await;
        }
        
        Ok(())
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
                id: "uinput-kbd".into(),
                name: "HCS Virtual Keyboard".into(),
                device_type: DeviceType::Keyboard,
                vendor_id: 0x1234,
                product_id: 0x5678,
                is_keyboard: true,
                is_mouse: false,
                is_touch: false,
            },
            DeviceInfo {
                id: "uinput-mouse".into(),
                name: "HCS Virtual Mouse".into(),
                device_type: DeviceType::Mouse,
                vendor_id: 0x1234,
                product_id: 0x5679,
                is_keyboard: false,
                is_mouse: true,
                is_touch: false,
            },
        ])
    }
    
    async fn set_exclusive(&self, _exclusive: bool) -> DriverResult<()> {
        warn!("Exclusive mode not supported with uinput");
        Ok(())
    }
    
    async fn shutdown(&mut self) -> DriverResult<()> {
        if let Some(fd) = self.fd {
            // Destroy device
            let _ = self.ioctl(fd, UINPUT_DEV_DESTROY, 0);
            let _ = close(fd);
        }
        self.fd = None;
        self.initialized = false;
        info!("uinput driver shutdown");
        Ok(())
    }
}

impl Drop for UInputDriver {
    fn drop(&mut self) {
        if self.initialized {
            let _ = futures::executor::block_on(self.shutdown());
        }
    }
}