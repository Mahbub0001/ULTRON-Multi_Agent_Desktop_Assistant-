use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use crate::client::grpc_client::hcs_agent_proto::MacroInfo;
use std::collections::HashMap;
use anyhow::Result;

pub struct MacroManager {
    macros: Vec<MacroInfo>,
    list_state: ListState,
    selected_macro: Option<usize>,
    show_details: bool,
    show_new_macro: bool,
    new_macro_name: String,
    new_macro_description: String,
    status_message: Option<String>,
}

impl MacroManager {
    pub fn new() -> Self {
        Self {
            macros: Vec::new(),
            list_state: ListState::default(),
            selected_macro: None,
            show_details: false,
            show_new_macro: false,
            new_macro_name: String::new(),
            new_macro_description: String::new(),
            status_message: None,
        }
    }

    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        if self.show_new_macro {
            return self.handle_new_macro_input(key).await;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Down, _) => self.next(),
            (KeyCode::Up, _) => self.previous(),
            (KeyCode::Enter, _) => {
                self.selected_macro = self.list_state.selected();
                self.show_details = true;
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.show_new_macro = true;
                self.new_macro_name.clear();
                self.new_macro_description.clear();
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.delete_selected().await?;
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                self.run_selected().await?;
            }
            (KeyCode::Esc, _) => {
                self.show_details = false;
                self.show_new_macro = false;
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
        if self.show_new_macro {
            self.draw_new_macro_dialog(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Macro list
        let items: Vec<ListItem> = self.macros.iter().enumerate().map(|(i, macro_info)| {
            let style = if Some(i) == self.list_state.selected() {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{} ({} params)", macro_info.name, macro_info.schema.parameters.len()))
                .style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Macros"))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        
        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);

        // Details or help
        if self.show_details && self.selected_macro.is_some() {
            self.draw_macro_details(frame, chunks[1]);
        } else {
            self.draw_help(frame, chunks[1]);
        }
    }

    fn draw_macro_details(&self, frame: &mut Frame, area: Rect) {
        if let Some(idx) = self.selected_macro {
            if let Some(macro_info) = self.macros.get(idx) {
                let mut lines = vec![
                    format!("Name: {}", macro_info.name),
                    format!("Description: {}", macro_info.description),
                    format!("Parameters:"),
                ];
                
                for (name, param) in &macro_info.schema.parameters {
                    let required = if param.required { " (required)" } else { "" };
                    lines.push(format!("  {}: {}{}", name, param.type, required));
                    if !param.description.is_empty() {
                        lines.push(format!("    {}", param.description));
                    }
                }

                let text = lines.join("\n");
                let para = Paragraph::new(text)
                    .block(Block::default().borders(Borders::ALL).title("Macro Details"))
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: true });
                frame.render_widget(para, area);
            }
        }
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = r#"
Macro Manager

Keyboard Shortcuts:
  ↑/↓       Navigate macros
  Enter     View details
  Ctrl+N    New macro
  Ctrl+D    Delete macro
  Ctrl+R    Run macro
  Esc       Close dialog

Macros are predefined input sequences that can be
executed with parameters. They are stored on the
agent server and can be called from tasks or REPL.

To create a macro:
1. Press Ctrl+N
2. Enter name and description
3. Define steps in the task editor
4. Save as macro

Parameters can be defined with types:
  string, int, float, bool
"#;
        let para = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    fn draw_new_macro_dialog(&self, frame: &mut Frame, area: Rect) {
        let dialog_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("New Macro")
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(block, dialog_area);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .margin(1)
            .split(dialog_area);

        let name_para = Paragraph::new(format!("Name: {}", self.new_macro_name))
            .block(Block::default().borders(Borders::ALL).title("Macro Name"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(name_para, inner[0]);

        let desc_para = Paragraph::new(format!("Description: {}", self.new_macro_description))
            .block(Block::default().borders(Borders::ALL).title("Description"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(desc_para, inner[1]);

        let hint = Paragraph::new("Enter: Next field | Esc: Cancel | Ctrl+S: Save")
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(hint, inner[2]);
    }

    async fn handle_new_macro_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => {
                if self.new_macro_name.is_empty() {
                    // Move to description
                } else if self.new_macro_description.is_empty() {
                    // Move to save
                } else {
                    // Save macro
                    self.show_new_macro = false;
                }
            }
            (KeyCode::Backspace, _) => {
                if !self.new_macro_description.is_empty() {
                    self.new_macro_description.pop();
                } else if !self.new_macro_name.is_empty() {
                    self.new_macro_name.pop();
                }
            }
            (KeyCode::Char(c), _) => {
                if self.new_macro_name.is_empty() || self.new_macro_description.is_empty() {
                    if self.new_macro_description.is_empty() {
                        self.new_macro_name.push(c);
                    } else {
                        self.new_macro_description.push(c);
                    }
                }
            }
            (KeyCode::Esc, _) => {
                self.show_new_macro = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.macros.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.macros.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    async fn delete_selected(&mut self) -> Result<()> {
        if let Some(idx) = self.selected_macro {
            if let Some(macro_info) = self.macros.get(idx) {
                // Would call gRPC to delete
                self.macros.remove(idx);
                if self.macros.is_empty() {
                    self.list_state.select(None);
                    self.selected_macro = None;
                } else {
                    let new_idx = idx.min(self.macros.len() - 1);
                    self.list_state.select(Some(new_idx));
                    self.selected_macro = Some(new_idx);
                }
                self.show_details = false;
                self.status_message = Some(format!("Deleted macro: {}", macro_info.name));
            }
        }
        Ok(())
    }

    async fn run_selected(&mut self) -> Result<()> {
        if let Some(idx) = self.selected_macro {
            if let Some(macro_info) = self.macros.get(idx) {
                // Would call gRPC to execute
                self.status_message = Some(format!("Running macro: {}", macro_info.name));
            }
        }
        Ok(())
    }

    pub fn set_macros(&mut self, macros: Vec<MacroInfo>) {
        self.macros = macros;
        if !self.macros.is_empty() {
            self.list_state.select(Some(0));
            self.selected_macro = Some(0);
        }
    }
}