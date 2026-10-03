use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, Wrap},
    Frame,
};
use std::collections::VecDeque;
use anyhow::Result;
use tracing::Level;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub level: Level,
    pub target: String,
    pub message: String,
}

pub struct LogViewer {
    logs: VecDeque<LogEntry>,
    max_logs: usize,
    list_state: ListState,
    filter_level: Option<Level>,
    filter_text: String,
    auto_scroll: bool,
    show_help: bool,
    status_message: Option<String>,
}

impl LogViewer {
    pub fn new() -> Self {
        Self {
            logs: VecDeque::new(),
            max_logs: 10000,
            list_state: ListState::default(),
            filter_level: None,
            filter_text: String::new(),
            auto_scroll: true,
            show_help: false,
            status_message: None,
        }
    }

    pub fn add_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= self.max_logs {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
        
        if self.auto_scroll && self.list_state.selected().is_some() {
            self.list_state.select(Some(self.filtered_logs().len().saturating_sub(1)));
        }
    }

    pub async fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};
        
        if self.show_help {
            if matches!(key.code, KeyCode::Esc | KeyCode::Char('h') | KeyCode::F(1)) {
                self.show_help = false;
            }
            return Ok(());
        }

        match (key.code, key.modifiers) {
            (KeyCode::Down, _) => self.next(),
            (KeyCode::Up, _) => self.previous(),
            (KeyCode::PageDown, _) => self.page_down(),
            (KeyCode::PageUp, _) => self.page_up(),
            (KeyCode::Home, _) => self.list_state.select(Some(0)),
            (KeyCode::End, _) => {
                let len = self.filtered_logs().len();
                if len > 0 {
                    self.list_state.select(Some(len - 1));
                }
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.auto_scroll = !self.auto_scroll;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.clear();
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                // Focus filter input - simplified
            }
            (KeyCode::Char('1'), _) => self.filter_level = Some(Level::ERROR),
            (KeyCode::Char('2'), _) => self.filter_level = Some(Level::WARN),
            (KeyCode::Char('3'), _) => self.filter_level = Some(Level::INFO),
            (KeyCode::Char('4'), _) => self.filter_level = Some(Level::DEBUG),
            (KeyCode::Char('5'), _) => self.filter_level = Some(Level::TRACE),
            (KeyCode::Char('0'), _) => self.filter_level = None,
            (KeyCode::Char('h'), _) | (KeyCode::F(1), _) => {
                self.show_help = true;
            }
            (KeyCode::Esc, _) => {
                self.filter_text.clear();
                self.filter_level = None;
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> Result<()> {
        Ok(())
    }

    pub async fn update(&mut self) {
        // Auto-scroll handled in add_log
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if self.show_help {
            self.draw_help(frame, area);
            return;
        }

        let filtered = self.filtered_logs();
        let items: Vec<ListItem> = filtered.iter().map(|entry| {
            let level_color = match entry.level {
                Level::ERROR => Color::Red,
                Level::WARN => Color::Yellow,
                Level::INFO => Color::Green,
                Level::DEBUG => Color::Blue,
                Level::TRACE => Color::DarkGray,
            };
            
            let level_str = match entry.level {
                Level::ERROR => "ERROR",
                Level::WARN => "WARN ",
                Level::INFO => "INFO ",
                Level::DEBUG => "DEBUG",
                Level::TRACE => "TRACE",
            };
            
            let time_str = entry.timestamp.format("%H:%M:%S%.3f").to_string();
            
            ListItem::new(vec![
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(format!("{} ", time_str), Style::default().fg(Color::DarkGray)),
                    ratatui::text::Span::styled(format!("{} ", level_str), Style::default().fg(level_color).add_modifier(Modifier::BOLD)),
                    ratatui::text::Span::styled(format!("{} ", entry.target), Style::default().fg(Color::Cyan)),
                    ratatui::text::Span::raw(&entry.message),
                ])
            ])
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(self.title()))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        
        let area_height = area.height as usize;
        frame.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn title(&self) -> String {
        let mut title = "Logs".to_string();
        if self.auto_scroll {
            title.push_str(" [Auto]");
        }
        if self.filter_level.is_some() || !self.filter_text.is_empty() {
            title.push_str(" [Filtered]");
        }
        title.push_str(&format!(" ({})", self.logs.len()));
        title
    }

    fn filtered_logs(&self) -> Vec<&LogEntry> {
        self.logs.iter()
            .filter(|entry| {
                if let Some(filter_level) = self.filter_level {
                    if entry.level > filter_level {
                        return false;
                    }
                }
                if !self.filter_text.is_empty() {
                    if !entry.message.to_lowercase().contains(&self.filter_text.to_lowercase()) &&
                       !entry.target.to_lowercase().contains(&self.filter_text.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let help_text = r#"
Log Viewer Help

Keyboard Shortcuts:
  ↑/↓           Navigate logs
  PgUp/PgDn     Page up/down
  Home/End      Jump to start/end
  Ctrl+A        Toggle auto-scroll
  Ctrl+C        Clear logs
  Ctrl+F        Focus filter
  0             Clear level filter
  1-5           Filter by level (Error/Warn/Info/Debug/Trace)
  Esc           Clear all filters
  h / F1        Toggle this help

Levels:
  1 - ERROR   (Red)
  2 - WARN    (Yellow)
  3 - INFO    (Green)
  4 - DEBUG   (Blue)
  5 - TRACE   (Gray)

Logs are captured from the orchestrator and
connected gRPC services. Auto-scroll keeps
the view at the bottom as new logs arrive.
"#;
        
        let para = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .style(Style::default().fg(Color::White))
            .wrap(Wrap { trim: true });
        frame.render_widget(para, area);
    }

    fn next(&mut self) {
        let filtered_len = self.filtered_logs().len();
        if filtered_len == 0 {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= filtered_len - 1 { 0 } else { i + 1 }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        let filtered_len = self.filtered_logs().len();
        if filtered_len == 0 {
            self.list_state.select(None);
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 { filtered_len - 1 } else { i - 1 }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn page_down(&mut self) {
        let filtered_len = self.filtered_logs().len();
        if filtered_len == 0 {
            return;
        }
        let page_size = 20;
        let i = match self.list_state.selected() {
            Some(i) => (i + page_size).min(filtered_len - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn page_up(&mut self) {
        let filtered_len = self.filtered_logs().len();
        if filtered_len == 0 {
            return;
        }
        let page_size = 20;
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(page_size),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn clear(&mut self) {
        self.logs.clear();
        self.list_state.select(None);
    }

    pub fn set_filter_text(&mut self, text: String) {
        self.filter_text = text;
    }

    pub fn set_filter_level(&mut self, level: Option<Level>) {
        self.filter_level = level;
    }
}

// Tracing layer to capture logs
pub struct LogViewerLayer {
    sender: tokio::sync::mpsc::UnboundedSender<LogEntry>,
}

impl LogViewerLayer {
    pub fn new() -> (Self, tokio::sync::mpsc::UnboundedReceiver<LogEntry>) {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        (Self { sender: tx }, rx)
    }
}

impl<S> tracing::Subscriber for LogViewerLayer
where
    S: tracing::Subscriber + Send + Sync,
{
    // This would need proper implementation with tracing-subscriber
    // For now, it's a placeholder
}