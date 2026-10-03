use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};
use tui_textarea::TextArea;
use crate::dsl::{DslParser, TaskDefinition};
use std::path::PathBuf;
use anyhow::Result;

pub struct TaskEditor {
    textarea: TextArea<'static>,
    file_path: Option<PathBuf>,
    parser: DslParser,
    task: Option<TaskDefinition>,
    validation_errors: Vec<String>,
    validation_warnings: Vec<String>,
    list_state: ListState,
    show_help: bool,
    modified: bool,
    status_message: Option<String>,
}

impl TaskEditor {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().borders(Borders::ALL).title("Task Editor (YAML/JSON)"));
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        
        Self {
            textarea,
            file_path: None,
            parser: DslParser::new(),
            task: None,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
            list_state: ListState::default(),
            show_help: false,
            modified: false,
            status_message: None,
        }
    }

    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        match (key.code, key.modifiers) {
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                self.save_file().await?;
            }
            (KeyCode::Char('o'), KeyModifiers::CONTROL) => {
                // Would open file dialog
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                self.new_file();
            }
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                self.validate().await?;
            }
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => {
                // Run task
            }
            (KeyCode::F(1), _) => {
                self.show_help = !self.show_help;
            }
            _ => {
                self.textarea.input(key);
                self.modified = true;
            }
        }
        Ok(())
    }

    pub async fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> Result<()> {
        Ok(())
    }

    pub async fn update(&mut self) {
        // Auto-validate on change (debounced)
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if self.show_help {
            self.draw_help(frame, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70),
                Constraint::Percentage(30),
            ])
            .split(area);

        // Editor pane
        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Task Editor{} - {}", 
                    if self.modified { "*" } else { "" },
                    self.file_path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "Untitled".to_string())
                ))
        );
        frame.render_widget(&self.textarea, chunks[0]);

        // Side panel with validation and structure
        let side_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8),  // Validation status
                Constraint::Min(0),     // Structure/Outline
            ])
            .split(chunks[1]);

        self.draw_validation_panel(frame, side_chunks[0]);
        self.draw_structure_panel(frame, side_chunks[1]);
    }

    fn draw_validation_panel(&self, frame: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
        
        if self.validation_errors.is_empty() && self.validation_warnings.is_empty() {
            if self.task.is_some() {
                lines.push("✓ Valid".to_string());
            } else {
                lines.push("No task loaded".to_string());
            }
        } else {
            for err in &self.validation_errors {
                lines.push(format!("✗ Error: {}", err));
            }
            for warn in &self.validation_warnings {
                lines.push(format!("⚠ Warning: {}", warn));
            }
        }

        let text = lines.join("\n");
        let para = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Validation"))
            .style(Style::default().fg(if self.validation_errors.is_empty() { Color::Green } else { Color::Red }))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    fn draw_structure_panel(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = if let Some(task) = &self.task {
            let mut items = vec![
                ListItem::new(format!("📋 {} v{}", task.name, task.version)).style(Style::default().fg(Color::Cyan)),
                ListItem::new(format!("📝 {}", task.description)).style(Style::default().fg(Color::Gray)),
            ];
            
            if !task.variables.is_empty() {
                items.push(ListItem::new("").style(Style::default()));
                items.push(ListItem::new("Variables:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                for (name, def) in &task.variables {
                    items.push(ListItem::new(format!("  {} ({})", name, def.var_type)).style(Style::default().fg(Color::White)));
                }
            }
            
            if !task.macros.is_empty() {
                items.push(ListItem::new("").style(Style::default()));
                items.push(ListItem::new("Macros:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
                for (name, macro_def) in &task.macros {
                    items.push(ListItem::new(format!("  {} ({} steps)", name, macro_def.steps.len())).style(Style::default().fg(Color::White)));
                }
            }
            
            items.push(ListItem::new("").style(Style::default()));
            items.push(ListItem::new("Steps:").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));
            for step in &task.steps {
                items.push(ListItem::new(format!("  ▶ {}", self.step_summary(step))).style(Style::default().fg(Color::Green)));
            }
            
            items
        } else {
            vec![ListItem::new("No task loaded").style(Style::default().fg(Color::DarkGray))]
        };

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Structure"))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn step_summary(&self, step: &crate::dsl::types::Step) -> String {
        use crate::dsl::types::*;
        match step {
            Step::Action(s) => format!("Action: {} ({:?})", s.name, s.action),
            Step::Macro(s) => format!("Macro: {} ({})", s.name, s.macro_name),
            Step::Sequential(s) => format!("Sequential: {} ({} steps)", s.name, s.steps.len()),
            Step::Parallel(s) => format!("Parallel: {} ({} steps)", s.name, s.steps.len()),
            Step::Conditional(s) => format!("If: {} then...", s.condition),
            Step::Loop(s) => format!("Loop: {:?}", s.loop_type),
            Step::Vision(s) => format!("Vision: {:?}", s.operation),
            Step::Brain(s) => format!("Brain: {:?}", s.operation),
            Step::Adapter(s) => format!("Adapter: {:?} {}", s.adapter, s.operation),
            Step::Variable(s) => format!("Variable: {:?}", s.operation),
            Step::Wait(s) => format!("Wait: {}", s.condition),
            Step::Log(s) => format!("Log: {}", s.message),
        }
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = r#"
Task Editor Help

Keyboard Shortcuts:
  Ctrl+S    Save file
  Ctrl+O    Open file
  Ctrl+N    New file
  Ctrl+V    Validate task
  Ctrl+R    Run task
  F1        Toggle this help
  Tab       Next tab
  Esc       Close dialogs

Task DSL Format (YAML):
  name: "Task Name"
  version: "1.0"
  description: "Description"
  variables:
    var_name:
      type: "string"
      default: "value"
  steps:
    - type: action
      id: "step1"
      name: "Press Enter"
      action:
        kind: "key_press"
        code: 13
    - type: macro
      id: "step2"
      name: "Run macro"
      macro_name: "my_macro"
  macros:
    my_macro:
      name: "My Macro"
      steps:
        - type: action
          ...

Action Types:
  key_down, key_up, key_press
  mouse_move, mouse_click, mouse_down, mouse_up, mouse_scroll
  delay, type_text

Control Flow:
  sequential, parallel, conditional, loop

Variables:
  Use ${variable_name} in strings for templating
"#;
        
        let para = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    pub async fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let content = tokio::fs::read_to_string(&path).await?;
        self.textarea = TextArea::from(content.lines().collect::<Vec<_>>());
        self.file_path = Some(path.clone());
        self.modified = false;
        
        let task = self.parser.parse_file(&path)?;
        self.validate_task(&task).await?;
        self.task = Some(task);
        
        self.status_message = Some(format!("Loaded: {}", path.display()));
        Ok(())
    }

    pub async fn save_file(&mut self) -> Result<()> {
        if let Some(path) = &self.file_path {
            let content = self.textarea.lines().join("\n");
            tokio::fs::write(path, content).await?;
            self.modified = false;
            self.status_message = Some(format!("Saved: {}", path.display()));
        }
        Ok(())
    }

    pub fn new_file(&mut self) {
        let template = r#"name: "New Task"
version: "1.0"
description: "Task description"
variables: {}
steps:
  - type: action
    id: "step1"
    name: "Example step"
    action:
      kind: "key_press"
      code: 13
macros: {}
"#;
        self.textarea = TextArea::from(template.lines().collect::<Vec<_>>());
        self.file_path = None;
        self.task = None;
        self.validation_errors.clear();
        self.validation_warnings.clear();
        self.modified = true;
    }

    pub async fn validate(&mut self) -> Result<()> {
        let content = self.textarea.lines().join("\n");
        let task = self.parser.parse_content(&content)?;
        self.validate_task(&task).await?;
        self.task = Some(task);
        Ok(())
    }

    async fn validate_task(&mut self, task: &TaskDefinition) -> Result<()> {
        self.validation_errors.clear();
        self.validation_warnings.clear();

        let warnings = self.parser.validate(task)?;
        self.validation_warnings = warnings;

        // Additional validation
        if task.steps.is_empty() {
            self.validation_warnings.push("Task has no steps".to_string());
        }

        Ok(())
    }
}