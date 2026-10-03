use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Cell},
    Frame,
};
use std::time::{Duration, Instant};
use tracing::{debug, info};

pub struct Dashboard {
    system_info: Option<crate::client::grpc_client::hcs_agent_proto::SystemInfo>,
    last_update: Instant,
    update_interval: Duration,
    task_stats: TaskStats,
    driver_stats: Option<crate::client::grpc_client::hcs_agent_proto::ControllerStats>,
}

#[derive(Debug, Default, Clone)]
pub struct TaskStats {
    pub total: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub pending: u32,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            system_info: None,
            last_update: Instant::now(),
            update_interval: Duration::from_secs(2),
            task_stats: TaskStats::default(),
            driver_stats: None,
        }
    }

    pub async fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub async fn handle_mouse(&mut self, _mouse: crossterm::event::MouseEvent) -> Result<(), anyhow::Error> {
        Ok(())
    }

    pub async fn update(&mut self) {
        if self.last_update.elapsed() >= self.update_interval {
            self.last_update = Instant::now();
            // In a real implementation, this would fetch data from gRPC services
            // For now, we'll use mock data
        }
    }

    pub fn set_system_info(&mut self, info: crate::client::grpc_client::hcs_agent_proto::SystemInfo) {
        self.system_info = Some(info);
    }

    pub fn set_task_stats(&mut self, stats: TaskStats) {
        self.task_stats = stats;
    }

    pub fn set_driver_stats(&mut self, stats: crate::client::grpc_client::hcs_agent_proto::ControllerStats) {
        self.driver_stats = Some(stats);
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12), // System info
                Constraint::Length(8),  // Task stats
                Constraint::Min(0),     // Driver stats / recent activity
            ])
            .split(area);

        self.draw_system_info(frame, chunks[0]);
        self.draw_task_stats(frame, chunks[1]);
        self.draw_driver_stats(frame, chunks[2]);
    }

    fn draw_system_info(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(area);

        // Left: System info
        let sys_info = self.system_info.as_ref();
        let sys_text = if let Some(info) = sys_info {
            format!(
                "Hostname: {}\nOS: {} {}\nArch: {}\nUptime: {}s\nCPU: {} ({} cores, {} threads)\nCPU Usage: {:.1}%\nMemory: {} / {} MB ({:.1}%)\nGPUs: {}",
                info.hostname,
                info.os,
                info.os, // version would be separate
                info.arch,
                info.uptime_seconds,
                info.cpu.brand,
                info.cpu.cores,
                info.cpu.threads,
                info.cpu.usage_percent,
                info.memory.used_bytes / 1024 / 1024,
                info.memory.total_bytes / 1024 / 1024,
                (info.memory.used_bytes as f64 / info.memory.total_bytes as f64) * 100.0,
                info.gpus.len()
            )
        } else {
            "System info not available\nConnecting to agent...".to_string()
        };

        let sys_para = Paragraph::new(sys_text)
            .block(Block::default().borders(Borders::ALL).title("System"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(sys_para, chunks[0]);

        // Right: Quick stats
        let cpu_usage = sys_info.map(|i| i.cpu.usage_percent).unwrap_or(0.0);
        let mem_usage = sys_info.map(|i| {
            (i.memory.used_bytes as f64 / i.memory.total_bytes as f64) * 100.0
        }).unwrap_or(0.0);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Min(0),
            ])
            .split(chunks[1]);

        let cpu_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("CPU Usage"))
            .gauge_style(Style::default().fg(if cpu_usage > 80.0 { Color::Red } else { Color::Green }))
            .ratio(cpu_usage / 100.0)
            .label(format!("{:.1}%", cpu_usage));
        frame.render_widget(cpu_gauge, right_chunks[0]);

        let mem_gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Memory Usage"))
            .gauge_style(Style::default().fg(if mem_usage > 80.0 { Color::Red } else { Color::Blue }))
            .ratio(mem_usage / 100.0)
            .label(format!("{:.1}%", mem_usage));
        frame.render_widget(mem_gauge, right_chunks[1]);

        // GPU info
        let gpu_text = if let Some(info) = sys_info {
            if info.gpus.is_empty() {
                "No GPUs detected".to_string()
            } else {
                info.gpus.iter().map(|g| {
                    format!("{}: {} MB, {:.1}% used", g.name, g.memory_bytes / 1024 / 1024, g.usage_percent)
                }).collect::<Vec<_>>().join("\n")
            }
        } else {
            "GPU info not available".to_string()
        };

        let gpu_para = Paragraph::new(gpu_text)
            .block(Block::default().borders(Borders::ALL).title("GPUs"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(gpu_para, right_chunks[2]);
    }

    fn draw_task_stats(&self, frame: &mut Frame, area: Rect) {
        let rows = vec![
            Row::new(vec![
                Cell::from("Total").style(Style::default().fg(Color::White)),
                Cell::from(self.task_stats.total.to_string()).style(Style::default().fg(Color::Cyan)),
            ]),
            Row::new(vec![
                Cell::from("Running").style(Style::default().fg(Color::White)),
                Cell::from(self.task_stats.running.to_string()).style(Style::default().fg(Color::Yellow)),
            ]),
            Row::new(vec![
                Cell::from("Completed").style(Style::default().fg(Color::White)),
                Cell::from(self.task_stats.completed.to_string()).style(Style::default().fg(Color::Green)),
            ]),
            Row::new(vec![
                Cell::from("Failed").style(Style::default().fg(Color::White)),
                Cell::from(self.task_stats.failed.to_string()).style(Style::default().fg(Color::Red)),
            ]),
            Row::new(vec![
                Cell::from("Pending").style(Style::default().fg(Color::White)),
                Cell::from(self.task_stats.pending.to_string()).style(Style::default().fg(Color::Gray)),
            ]),
        ];

        let table = Table::new(rows, [Constraint::Percentage(50), Constraint::Percentage(50)])
            .block(Block::default().borders(Borders::ALL).title("Task Statistics"))
            .header(Row::new(vec!["Status", "Count"]).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            .row_highlight_style(Style::default().add_modifier(Modifier::BOLD));
        
        frame.render_widget(table, area);
    }

    fn draw_driver_stats(&self, frame: &mut Frame, area: Rect) {
        let stats = self.driver_stats.as_ref();
        
        let text = if let Some(s) = stats {
            format!(
                "Backend: {:?}\nEvents Injected: {}\nEvents Failed: {}\nAvg Latency: {}µs\nLast Error: {}",
                s.backend, s.events_injected, s.events_failed, s.avg_latency_us, s.last_error
            )
        } else {
            "Driver stats not available".to_string()
        };

        let para = Paragraph::new(text)
            .block(Block::default().borders(Borders::ALL).title("Input Driver"))
            .style(Style::default().fg(Color::White));
        frame.render_widget(para, area);
    }
}