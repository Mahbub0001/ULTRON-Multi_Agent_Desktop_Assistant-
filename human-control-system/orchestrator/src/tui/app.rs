use crate::tui::dashboard::Dashboard;
use crate::tui::task_editor::TaskEditor;
use crate::tui::macro_manager::MacroManager;
use crate::tui::device_monitor::DeviceMonitor;
use crate::tui::log_viewer::LogViewer;
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    TaskEditor,
    MacroManager,
    DeviceMonitor,
    LogViewer,
}

impl Tab {
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::TaskEditor => "Task Editor",
            Tab::MacroManager => "Macros",
            Tab::DeviceMonitor => "Devices",
            Tab::LogViewer => "Logs",
        }
    }

    pub fn all() -> Vec<Tab> {
        vec![
            Tab::Dashboard,
            Tab::TaskEditor,
            Tab::MacroManager,
            Tab::DeviceMonitor,
            Tab::LogViewer,
        ]
    }

    pub fn from_index(index: usize) -> Option<Tab> {
        match index {
            0 => Some(Tab::Dashboard),
            1 => Some(Tab::TaskEditor),
            2 => Some(Tab::MacroManager),
            3 => Some(Tab::DeviceMonitor),
            4 => Some(Tab::LogViewer),
            _ => None,
        }
    }
}

pub struct App {
    current_tab: Tab,
    tabs: Vec<Tab>,
    dashboard: Dashboard,
    task_editor: TaskEditor,
    macro_manager: MacroManager,
    device_monitor: DeviceMonitor,
    log_viewer: LogViewer,
    should_quit: bool,
    status_message: Option<String>,
    status_timer: Option<std::time::Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::Dashboard,
            tabs: Tab::all(),
            dashboard: Dashboard::new(),
            task_editor: TaskEditor::new(),
            macro_manager: MacroManager::new(),
            device_monitor: DeviceMonitor::new(),
            log_viewer: LogViewer::new(),
            should_quit: false,
            status_message: None,
            status_timer: None,
        }
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = Some(message);
        self.status_timer = Some(std::time::Instant::now());
    }

    pub fn next_tab(&mut self) {
        let current_idx = self.tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        let next_idx = (current_idx + 1) % self.tabs.len();
        self.current_tab = self.tabs[next_idx];
    }

    pub fn previous_tab(&mut self) {
        let current_idx = self.tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0);
        let prev_idx = if current_idx == 0 { self.tabs.len() - 1 } else { current_idx - 1 };
        self.current_tab = self.tabs[prev_idx];
    }

    pub fn go_to_tab(&mut self, tab: Tab) {
        if self.tabs.contains(&tab) {
            self.current_tab = tab;
        }
    }

    pub async fn handle_event(&mut self, event: Event) -> Result<(), anyhow::Error> {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        self.should_quit = true;
                    }
                    KeyCode::Tab => {
                        self.next_tab();
                    }
                    KeyCode::BackTab => {
                        self.previous_tab();
                    }
                    KeyCode::F(1) => self.go_to_tab(Tab::Dashboard),
                    KeyCode::F(2) => self.go_to_tab(Tab::TaskEditor),
                    KeyCode::F(3) => self.go_to_tab(Tab::MacroManager),
                    KeyCode::F(4) => self.go_to_tab(Tab::DeviceMonitor),
                    KeyCode::F(5) => self.go_to_tab(Tab::LogViewer),
                    _ => {
                        // Delegate to current tab handler
                        match self.current_tab {
                            Tab::Dashboard => self.dashboard.handle_key(key).await?,
                            Tab::TaskEditor => self.task_editor.handle_key(key).await?,
                            Tab::MacroManager => self.macro_manager.handle_key(key).await?,
                            Tab::DeviceMonitor => self.device_monitor.handle_key(key).await?,
                            Tab::LogViewer => self.log_viewer.handle_key(key).await?,
                        }
                    }
                }
            }
            Event::Mouse(mouse) => {
                match self.current_tab {
                    Tab::Dashboard => self.dashboard.handle_mouse(mouse).await?,
                    Tab::TaskEditor => self.task_editor.handle_mouse(mouse).await?,
                    Tab::MacroManager => self.macro_manager.handle_mouse(mouse).await?,
                    Tab::DeviceMonitor => self.device_monitor.handle_mouse(mouse).await?,
                    Tab::LogViewer => self.log_viewer.handle_mouse(mouse).await?,
                }
            }
            Event::Resize(_, _) => {
                // Terminal resize handled automatically
            }
            _ => {}
        }
        Ok(())
    }

    pub async fn update(&mut self) {
        // Update status message timer
        if let Some(timer) = self.status_timer {
            if timer.elapsed().as_secs() > 3 {
                self.status_message = None;
                self.status_timer = None;
            }
        }

        // Update current tab
        match self.current_tab {
            Tab::Dashboard => self.dashboard.update().await,
            Tab::TaskEditor => self.task_editor.update().await,
            Tab::MacroManager => self.macro_manager.update().await,
            Tab::DeviceMonitor => self.device_monitor.update().await,
            Tab::LogViewer => self.log_viewer.update().await,
        }
    }

    pub fn draw(&mut self, frame: &mut Frame) {
        let size = frame.size();
        
        // Main layout with tabs at top
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Tab bar
                Constraint::Min(0),     // Content
                Constraint::Length(1),  // Status bar
            ])
            .split(size);

        // Draw tabs
        let tab_titles: Vec<String> = self.tabs.iter().map(|t| t.title().to_string()).collect();
        let tabs_widget = Tabs::new(tab_titles)
            .block(Block::default().borders(Borders::ALL).title("HCS Orchestrator"))
            .select(self.tabs.iter().position(|t| *t == self.current_tab).unwrap_or(0))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        frame.render_widget(tabs_widget, chunks[0]);

        // Draw current tab content
        match self.current_tab {
            Tab::Dashboard => self.dashboard.draw(frame, chunks[1]),
            Tab::TaskEditor => self.task_editor.draw(frame, chunks[1]),
            Tab::MacroManager => self.macro_manager.draw(frame, chunks[1]),
            Tab::DeviceMonitor => self.device_monitor.draw(frame, chunks[1]),
            Tab::LogViewer => self.log_viewer.draw(frame, chunks[1]),
        }

        // Draw status bar
        let status_text = self.status_message.as_deref().unwrap_or("Ready | F1-F5: Tabs | Tab: Next | q: Quit");
        let status = Paragraph::new(status_text)
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(status, chunks[2]);
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

pub async fn run_tui(initial_tab: Option<String>) -> Result<(), anyhow::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();
    
    if let Some(tab_str) = initial_tab {
        match tab_str.to_lowercase().as_str() {
            "dashboard" => app.go_to_tab(Tab::Dashboard),
            "editor" | "task_editor" => app.go_to_tab(Tab::TaskEditor),
            "macros" | "macro_manager" => app.go_to_tab(Tab::MacroManager),
            "devices" | "device_monitor" => app.go_to_tab(Tab::DeviceMonitor),
            "logs" | "log_viewer" => app.go_to_tab(Tab::LogViewer),
            _ => {}
        }
    }

    // Main loop
    let tick_rate = std::time::Duration::from_millis(16); // ~60fps
    let mut last_tick = std::time::Instant::now();

    while !app.should_quit() {
        // Handle events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('c') && key.modifiers.contains(event::KeyModifiers::CONTROL) {
                    break;
                }
            }
            app.handle_event(event::read()?).await?;
        }

        // Update at tick rate
        if last_tick.elapsed() >= tick_rate {
            app.update().await;
            last_tick = std::time::Instant::now();
        }

        // Draw
        terminal.draw(|frame| app.draw(frame))?;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new();
        assert_eq!(app.current_tab, Tab::Dashboard);
        
        app.next_tab();
        assert_eq!(app.current_tab, Tab::TaskEditor);
        
        app.next_tab();
        assert_eq!(app.current_tab, Tab::MacroManager);
        
        app.previous_tab();
        assert_eq!(app.current_tab, Tab::TaskEditor);
    }
}