//! Common input macros and sequences

use crate::{
    InputEvent, KeyboardEvent, MouseEvent, KeyState, MouseButton, VirtualKey,
    DriverResult, InputController
};
use std::time::Duration;

/// Pre-defined key combinations
pub mod shortcuts {
    use super::*;
    
    /// Copy (Ctrl+C)
    pub fn copy() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::C as u16, 0x2E),
            InputEvent::key_up(VirtualKey::C as u16, 0x2E),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Paste (Ctrl+V)
    pub fn paste() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::V as u16, 0x2F),
            InputEvent::key_up(VirtualKey::V as u16, 0x2F),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Cut (Ctrl+X)
    pub fn cut() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::X as u16, 0x2D),
            InputEvent::key_up(VirtualKey::X as u16, 0x2D),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Select All (Ctrl+A)
    pub fn select_all() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::A as u16, 0x1E),
            InputEvent::key_up(VirtualKey::A as u16, 0x1E),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Undo (Ctrl+Z)
    pub fn undo() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::Z as u16, 0x2C),
            InputEvent::key_up(VirtualKey::Z as u16, 0x2C),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Redo (Ctrl+Y)
    pub fn redo() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::Y as u16, 0x15),
            InputEvent::key_up(VirtualKey::Y as u16, 0x15),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Save (Ctrl+S)
    pub fn save() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::S as u16, 0x1F),
            InputEvent::key_up(VirtualKey::S as u16, 0x1F),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// New (Ctrl+N)
    pub fn new() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::N as u16, 0x31),
            InputEvent::key_up(VirtualKey::N as u16, 0x31),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Open (Ctrl+O)
    pub fn open() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::O as u16, 0x18),
            InputEvent::key_up(VirtualKey::O as u16, 0x18),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Find (Ctrl+F)
    pub fn find() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::F as u16, 0x21),
            InputEvent::key_up(VirtualKey::F as u16, 0x21),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Close Tab (Ctrl+W)
    pub fn close_tab() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::W as u16, 0x11),
            InputEvent::key_up(VirtualKey::W as u16, 0x11),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Switch Tab (Ctrl+Tab)
    pub fn next_tab() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Previous Tab (Ctrl+Shift+Tab)
    pub fn prev_tab() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::LShift as u16, 0x2A),
            InputEvent::key_down(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::LShift as u16, 0x2A),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Alt+Tab (Window switch)
    pub fn alt_tab() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LMenu as u16, 0x38),
            InputEvent::key_down(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::Tab as u16, 0x0F),
            InputEvent::key_up(VirtualKey::LMenu as u16, 0x38),
        ]
    }
    
    /// Win+D (Show desktop)
    pub fn show_desktop() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LWin as u16, 0x5B),
            InputEvent::key_down(VirtualKey::D as u16, 0x20),
            InputEvent::key_up(VirtualKey::D as u16, 0x20),
            InputEvent::key_up(VirtualKey::LWin as u16, 0x5B),
        ]
    }
    
    /// Win+E (File Explorer)
    pub fn file_explorer() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LWin as u16, 0x5B),
            InputEvent::key_down(VirtualKey::E as u16, 0x12),
            InputEvent::key_up(VirtualKey::E as u16, 0x12),
            InputEvent::key_up(VirtualKey::LWin as u16, 0x5B),
        ]
    }
    
    /// Win+R (Run dialog)
    pub fn run_dialog() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LWin as u16, 0x5B),
            InputEvent::key_down(VirtualKey::R as u16, 0x13),
            InputEvent::key_up(VirtualKey::R as u16, 0x13),
            InputEvent::key_up(VirtualKey::LWin as u16, 0x5B),
        ]
    }
    
    /// Ctrl+Shift+Esc (Task Manager)
    pub fn task_manager() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LControl as u16, 0x1D),
            InputEvent::key_down(VirtualKey::LShift as u16, 0x2A),
            InputEvent::key_down(VirtualKey::Escape as u16, 0x01),
            InputEvent::key_up(VirtualKey::Escape as u16, 0x01),
            InputEvent::key_up(VirtualKey::LShift as u16, 0x2A),
            InputEvent::key_up(VirtualKey::LControl as u16, 0x1D),
        ]
    }
    
    /// Print Screen
    pub fn print_screen() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::Snapshot as u16, 0x37),
            InputEvent::key_up(VirtualKey::Snapshot as u16, 0x37),
        ]
    }
    
    /// Alt+PrintScreen (Active window)
    pub fn alt_print_screen() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::LMenu as u16, 0x38),
            InputEvent::key_down(VirtualKey::Snapshot as u16, 0x37),
            InputEvent::key_up(VirtualKey::Snapshot as u16, 0x37),
            InputEvent::key_up(VirtualKey::LMenu as u16, 0x38),
        ]
    }
}

/// Gaming macros
pub mod gaming {
    use super::*;
    
    /// WASD movement
    pub fn move_forward() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::W as u16, 0x11)]
    }
    
    pub fn move_backward() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::S as u16, 0x1F)]
    }
    
    pub fn move_left() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::A as u16, 0x1E)]
    }
    
    pub fn move_right() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::D as u16, 0x20)]
    }
    
    pub fn stop_movement() -> Vec<InputEvent> {
        vec![
            InputEvent::key_up(VirtualKey::W as u16, 0x11),
            InputEvent::key_up(VirtualKey::A as u16, 0x1E),
            InputEvent::key_up(VirtualKey::S as u16, 0x1F),
            InputEvent::key_up(VirtualKey::D as u16, 0x20),
        ]
    }
    
    /// Jump (Space)
    pub fn jump() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::Space as u16, 0x39),
            InputEvent::Delay(50_000),
            InputEvent::key_up(VirtualKey::Space as u16, 0x39),
        ]
    }
    
    /// Crouch (Ctrl)
    pub fn crouch() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::LControl as u16, 0x1D)]
    }
    
    pub fn uncrouch() -> Vec<InputEvent> {
        vec![InputEvent::key_up(VirtualKey::LControl as u16, 0x1D)]
    }
    
    /// Sprint (Shift)
    pub fn sprint() -> Vec<InputEvent> {
        vec![InputEvent::key_down(VirtualKey::LShift as u16, 0x2A)]
    }
    
    pub fn unsprint() -> Vec<InputEvent> {
        vec![InputEvent::key_up(VirtualKey::LShift as u16, 0x2A)]
    }
    
    /// Reload (R)
    pub fn reload() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::R as u16, 0x13),
            InputEvent::Delay(50_000),
            InputEvent::key_up(VirtualKey::R as u16, 0x13),
        ]
    }
    
    /// Use/Interact (E)
    pub fn interact() -> Vec<InputEvent> {
        vec![
            InputEvent::key_down(VirtualKey::E as u16, 0x12),
            InputEvent::Delay(50_000),
            InputEvent::key_up(VirtualKey::E as u16, 0x12),
        ]
    }
    
    /// Quick weapon switch (1-5)
    pub fn weapon_slot(slot: u8) -> Vec<InputEvent> {
        let key = match slot {
            1 => VirtualKey::Key1,
            2 => VirtualKey::Key2,
            3 => VirtualKey::Key3,
            4 => VirtualKey::Key4,
            5 => VirtualKey::Key5,
            _ => VirtualKey::Key1,
        };
        vec![
            InputEvent::key_down(key as u16, 0),
            InputEvent::Delay(50_000),
            InputEvent::key_up(key as u16, 0),
        ]
    }
}

/// Text input helpers
pub mod text {
    use super::*;
    
    /// Type string with enter
    pub fn type_line(text: &str) -> Vec<InputEvent> {
        let mut events = Vec::new();
        for ch in text.chars() {
            events.extend(char_to_events(ch));
        }
        events.push(InputEvent::key_down(VirtualKey::Return as u16, 0x1C));
        events.push(InputEvent::key_up(VirtualKey::Return as u16, 0x1C));
        events
    }
    
    /// Type string without enter
    pub fn type_text(text: &str) -> Vec<InputEvent> {
        let mut events = Vec::new();
        for ch in text.chars() {
            events.extend(char_to_events(ch));
        }
        events
    }
    
    fn char_to_events(ch: char) -> Vec<InputEvent> {
        // Simplified - real implementation uses keyboard layout
        let (vk, scan) = char_to_vk_scan(ch);
        vec![
            InputEvent::key_down(vk, scan),
            InputEvent::Delay(10_000),
            InputEvent::key_up(vk, scan),
            InputEvent::Delay(10_000),
        ]
    }
    
    fn char_to_vk_scan(ch: char) -> (u16, u16) {
        match ch {
            'a'..='z' => (ch as u16 - 'a' as u16 + 0x41, 0),
            'A'..='Z' => (ch as u16 - 'A' as u16 + 0x41, 0),
            '0'..='9' => (ch as u16 - '0' as u16 + 0x30, 0),
            ' ' => (0x20, 0x39),
            '\n' => (0x0D, 0x1C),
            '\t' => (0x09, 0x0F),
            _ => (0x00, 0),
        }
    }
}

/// Mouse macros
pub mod mouse {
    use super::*;
    
    /// Click at current position
    pub fn click(button: MouseButton) -> Vec<InputEvent> {
        vec![
            button_event(button, KeyState::Down),
            InputEvent::Delay(50_000),
            button_event(button, KeyState::Up),
        ]
    }
    
    /// Double click
    pub fn double_click(button: MouseButton) -> Vec<InputEvent> {
        vec![
            button_event(button, KeyState::Down),
            InputEvent::Delay(50_000),
            button_event(button, KeyState::Up),
            InputEvent::Delay(100_000),
            button_event(button, KeyState::Down),
            InputEvent::Delay(50_000),
            button_event(button, KeyState::Up),
        ]
    }
    
    /// Drag from current position
    pub fn drag_start(button: MouseButton) -> Vec<InputEvent> {
        vec![button_event(button, KeyState::Down)]
    }
    
    pub fn drag_end(button: MouseButton) -> Vec<InputEvent> {
        vec![button_event(button, KeyState::Up)]
    }
    
    /// Scroll
    pub fn scroll(delta: i32) -> Vec<InputEvent> {
        (0..delta.unsigned_abs()).map(|_| button_event(if delta > 0 { MouseButton::WheelUp } else { MouseButton::WheelDown }, KeyState::Down)).collect()
    }
    
    /// Smooth move to position
    pub fn smooth_move(x: i32, y: i32, steps: u32) -> Vec<InputEvent> {
        // This would need current position - simplified
        vec![InputEvent::Mouse(MouseEvent { x, y, dx: 0, dy: 0, absolute: true, button: None, button_state: None, timestamp: 0 })]
    }
}

/// High-level macro executor
pub struct MacroExecutor {
    controller: InputController,
}

impl MacroExecutor {
    pub fn new(controller: InputController) -> Self {
        Self { controller }
    }
    
    /// Execute a macro with human-like timing
    pub async fn execute(&self, events: Vec<InputEvent>, variance_ms: u64) -> DriverResult<()> {
        for event in events {
            match event {
                InputEvent::Keyboard(event) => self.controller.inject_keyboard(event).await?,
                InputEvent::Mouse(event) => self.controller.inject_mouse(event).await?,
                InputEvent::Delay(microseconds) => {
                    let variance = if variance_ms == 0 { 0 } else { (rand::random::<u64>() % variance_ms).saturating_mul(1000) };
                    tokio::time::sleep(Duration::from_micros(microseconds.saturating_add(variance))).await;
                }
            }
        }
        Ok(())
    }
    
    /// Execute predefined shortcut
    pub async fn shortcut(&self, name: &str) -> DriverResult<()> {
        let events = match name {
            "copy" => shortcuts::copy(),
            "paste" => shortcuts::paste(),
            "cut" => shortcuts::cut(),
            "select_all" => shortcuts::select_all(),
            "undo" => shortcuts::undo(),
            "redo" => shortcuts::redo(),
            "save" => shortcuts::save(),
            "new" => shortcuts::new(),
            "open" => shortcuts::open(),
            "find" => shortcuts::find(),
            "close_tab" => shortcuts::close_tab(),
            "next_tab" => shortcuts::next_tab(),
            "prev_tab" => shortcuts::prev_tab(),
            "alt_tab" => shortcuts::alt_tab(),
            "show_desktop" => shortcuts::show_desktop(),
            "file_explorer" => shortcuts::file_explorer(),
            "run_dialog" => shortcuts::run_dialog(),
            "task_manager" => shortcuts::task_manager(),
            "print_screen" => shortcuts::print_screen(),
            _ => return Err(crate::DriverError::InvalidParameter(
                format!("Unknown shortcut: {}", name)
            )),
        };
        
        self.execute(events, 50).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_shortcut_generation() {
        let copy = shortcuts::copy();
        assert_eq!(copy.len(), 4);
        
        let paste = shortcuts::paste();
        assert_eq!(paste.len(), 4);
    }
    
    #[test]
    fn test_gaming_macros() {
        let jump = gaming::jump();
        assert!(jump.len() >= 3);
        
        let reload = gaming::reload();
        assert!(reload.len() >= 3);
    }
}
fn button_event(button: MouseButton, state: KeyState) -> InputEvent {
    InputEvent::Mouse(MouseEvent { x: 0, y: 0, dx: 0, dy: 0, button: Some(button), button_state: Some(state), absolute: false, timestamp: 0 })
}
