use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Table, Row, Cell, Wrap},
    Frame,
};
use crate::client::grpc_client::hcs_agent_proto::DeviceInfo;
use anyhow::Result;

pub struct DeviceMonitor {
    devices: Vec<DeviceInfo>,
    list_state: ListState,
    selected_device: Option<usize>,
    show_details: bool,
    filter: DeviceFilter,
    status_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceFilter {
    All,
    Keyboards,
    Mice,
    Touch,
    Gamepads,
}

impl DeviceMonitor {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            list_state: ListState::default(),
            selected_device: None,
            show_details: false,
            filter: DeviceFilter::All,
            status_message: None,
        }
    }

    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match (key.code, key.modifiers) {
            (KeyCode::Down, _) => self.next(),
            (KeyCode::Up, _) => self.previous(),
            (KeyCode::Enter, _) => {
                self.selected_device = self.list_state.selected();
                self.show_details = true;
            }
            (KeyCode::Char('1'), _) => self.filter = DeviceFilter::All,
            (KeyCode::Char('2'), _) => self.filter = DeviceFilter::Keyboards,
            (KeyCode::Char('3'), _) => self.filter = DeviceFilter::Mice,
            (KeyCode::Char('4'), _) => self.filter = DeviceFilter::Touch,
            (KeyCode::Char('5'), _) => self.filter = DeviceFilter::Gamepads,
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                // Refresh
            }
            (KeyCode::Esc, _) => {
                self.show_details = false;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> Result<()> {
        Ok(())
    }

    pub async fn update(&mut self) {
        // Would refresh from gRPC
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Device list
        let filtered_devices: Vec<&DeviceInfo> = self.devices.iter()
            .filter(|d| self.matches_filter(d))
            .collect();

        let items: Vec<ListItem> = filtered_devices.iter().enumerate().map(|(i, device)| {
            let style = if Some(i) == self.list_state.selected() {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            
            let type_icon = match device.device_type {
                crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeKeyboard => "⌨",
                crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeMouse => "🖱",
                crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeTouchscreen => "👆",
                crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeGamepad => "🎮",
                crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeTablet => "📱",
                _ => "❓",
            };
            
            ListItem::new(format!("{} {} (VID:{:04X} PID:{:04X})", 
                type_icon, device.name, device.vendor_id, device.product_id))
                .style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Input Devices"))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        
        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        // Details or filter help
        if self.show_details && self.selected_device.is_some() {
            self.draw_device_details(frame, chunks[1]);
        } else {
            self.draw_filter_help(frame, chunks[1]);
        }
    }

    fn draw_device_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(idx) = self.selected_device {
            let filtered_devices: Vec<&DeviceInfo> = self.devices.iter()
                .filter(|d| self.matches_filter(d))
                .collect();
            
            if let Some(device) = filtered_devices.get(idx) {
                let rows = vec![
                    Row::new(vec![Cell::from("ID"), Cell::from(&device.id)]),
                    Row::new(vec![Cell::from("Name"), Cell::from(&device.name)]),
                    Row::new(vec![Cell::from("Type"), Cell::from(format!("{:?}", device.device_type))]),
                    Row::new(vec![Cell::from("Vendor ID"), Cell::from(format!("{:04X}", device.vendor_id))]),
                    Row::new(vec![Cell::from("Product ID"), Cell::from(format!("{:04X}", device.product_id))]),
                    Row::new(vec![Cell::from("Keyboard"), Cell::from(if device.is_keyboard { "Yes" } else { "No" })]),
                    Row::new(vec![Cell::from("Mouse"), Cell::from(if device.is_mouse { "Yes" } else { "No" })]),
                    Row::new(vec![Cell::from("Touch"), Cell::from(if device.is_touch { "Yes" } else { "No" })]),
                ];

                let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(70)])
                    .block(Block::default().borders(Borders::ALL).title("Device Details"))
                    .header(Row::new(vec!["Property", "Value"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
                    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));
                
                frame.render_widget(table, area);
            }
        }
    }

    fn draw_filter_help(&self, frame: &mut Frame, area: Rect) {
        let filter_name = match self.filter {
            DeviceFilter::All => "All",
            DeviceFilter::Keyboards => "Keyboards",
            DeviceFilter::Mice => "Mice",
            DeviceFilter::Touch => "Touch",
            DeviceFilter::Gamepads => "Gamepads",
        };

        let help_text = format!(r#"
Device Monitor

Filters:
  1 - All ({})
  2 - Keyboards
  3 - Mice
  4 - Touch
  5 - Gamepads

Keyboard Shortcuts:
  ↑/↓       Navigate devices
  Enter     View details
  Ctrl+R    Refresh
  Esc       Close details

Connected devices are detected by the agent
driver. Use filters to narrow down the list.
"#, filter_name);
        
        let para = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Filters & Help"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    fn matches_filter(&self, device: &DeviceInfo) -> bool {
        match self.filter {
            DeviceFilter::All => true,
            DeviceFilter::Keyboards => device.is_keyboard,
            DeviceFilter::Mice => device.is_mouse,
            DeviceFilter::Touch => device.is_touch,
            DeviceFilter::Gamepads => device.device_type == crate::client::grpc_client::hcs_agent_proto::DeviceType::DeviceTypeGamepad,
        }
    }

    fn next(&mut self) {
        let filtered_count = self.devices.iter().filter(|d| self.matches_filter(d)).count();
        if filtered_count == 0 {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= filtered_count - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let filtered_count = self.devices.iter().filter(|d| self.matches_filter(d)).count();
        if filtered_count == 0 {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 { filtered_count - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    pub fn set_devices(&mut self, devices: Vec<DeviceInfo>) {
        self.devices = devices;
        if !self.devices.is_empty() {
            self.list_state.select(Some(0));
            self.selected_device = Some(0);
        }
    }
}