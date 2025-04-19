use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Cell, Chart, Dataset, Gauge, Paragraph, Row, Table, Tabs},
};

use crate::util::SystemState;

#[derive(Clone, Copy)]
enum DashboardView {
    Overview,
    Cpu,
    Memory,
    Disk,
    Network,
    Processes,
}

pub struct Dashboard {
    current_view: DashboardView,
    should_quit: bool,
    system_state: Arc<Mutex<SystemState>>,
}

impl Dashboard {
    pub fn new(system_state: Arc<Mutex<SystemState>>) -> Self {
        Self {
            current_view: DashboardView::Overview,
            should_quit: false,
            system_state,
        }
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        enable_raw_mode()?;
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        terminal.clear()?;

        while !self.should_quit {
            terminal.draw(|f| {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints(
                        [
                            Constraint::Length(3),
                            Constraint::Min(0),
                            Constraint::Length(1),
                        ]
                        .as_ref(),
                    )
                    .split(f.size());

                let tab_titles = vec!["Overview", "CPU", "Memory", "Disk", "Network", "Processes"];
                let tabs = Tabs::new(
                    tab_titles
                        .iter()
                        .map(|t| Spans::from(vec![Span::styled(*t, Style::default())]))
                        .collect(),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("System Monitor"),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .select(self.current_view as usize);
                f.render_widget(tabs, chunks[0]);

                match self.current_view {
                    DashboardView::Overview => self.render_overview(f, chunks[1]),
                    DashboardView::Cpu => self.render_cpu(f, chunks[1]),
                    DashboardView::Memory => self.render_memory(f, chunks[1]),
                    DashboardView::Disk => self.render_disk(f, chunks[1]),
                    DashboardView::Network => self.render_network(f, chunks[1]),
                    DashboardView::Processes => self.render_processes(f, chunks[1]),
                };

                let status = Paragraph::new("Press 'q' to quit, arrow keys to navigate")
                    .style(Style::default().fg(Color::White));
                f.render_widget(status, chunks[2]);
            })?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_input(key.code);
                }
            }
        }

        disable_raw_mode()?;
        terminal.clear()?;

        Ok(())
    }

    fn render_overview(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let state = match self.system_state.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                    Constraint::Percentage(25),
                ]
                .as_ref(),
            )
            .split(area);

        let cpu_usage = state.system.global_cpu_usage();
        let cpu_summary = Paragraph::new(vec![
            Spans::from(vec![Span::raw(format!("CPU Usage: {:.1}%", cpu_usage))]),
            Spans::from(vec![Span::raw(format!(
                "Cores: {}",
                state.system.cpus().iter().count()
            ))]),
        ])
        .block(Block::default().title("CPU Summary").borders(Borders::ALL));
        f.render_widget(cpu_summary, chunks[0]);

        let mem_used = state.system.used_memory();
        let mem_total = state.system.total_memory();
        let mem_percent = (mem_used as f64 / mem_total as f64 * 100.0) as u64;

        let memory_summary = Paragraph::new(vec![
            Spans::from(vec![Span::raw(format!("Memory Usage: {}%", mem_percent))]),
            Spans::from(vec![Span::raw(format!(
                "Used: {:.2} GB",
                mem_used as f64 / 1_000_000_000.0
            ))]),
            Spans::from(vec![Span::raw(format!(
                "Total: {:.2} GB",
                mem_total as f64 / 1_000_000_000.0
            ))]),
        ])
        .block(
            Block::default()
                .title("Memory Summary")
                .borders(Borders::ALL),
        );
        f.render_widget(memory_summary, chunks[1]);

        let disk_summary = Block::default().title("Disk Summary").borders(Borders::ALL);
        f.render_widget(disk_summary, chunks[2]);

        let network_summary = Block::default()
            .title("Network Summary")
            .borders(Borders::ALL);
        f.render_widget(network_summary, chunks[3]);
    }

    fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Left => {
                self.current_view = match self.current_view {
                    DashboardView::Overview => DashboardView::Processes,
                    DashboardView::Cpu => DashboardView::Overview,
                    DashboardView::Memory => DashboardView::Cpu,
                    DashboardView::Disk => DashboardView::Memory,
                    DashboardView::Network => DashboardView::Disk,
                    DashboardView::Processes => DashboardView::Network,
                }
            }
            KeyCode::Right => {
                self.current_view = match self.current_view {
                    DashboardView::Overview => DashboardView::Cpu,
                    DashboardView::Cpu => DashboardView::Memory,
                    DashboardView::Memory => DashboardView::Disk,
                    DashboardView::Disk => DashboardView::Network,
                    DashboardView::Network => DashboardView::Processes,
                    DashboardView::Processes => DashboardView::Overview,
                }
            }

            _ => {}
        }
    }

    fn render_cpu(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let state = match self.system_state.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(area);
        let cpu_usage = state.system.global_cpu_usage();
        let cpu_usage_text = format!("CPU Usage: {:.1}%", cpu_usage);

        let cpu_gauge = Gauge::default()
            .block(
                Block::default()
                    .title("Current CPU Usage")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(cpu_usage as u16);

        f.render_widget(cpu_gauge, chunks[0]);

        let cpu_history = &state.cpu_history;

        let mut chart_data: Vec<(f64, f64)> = Vec::new();
        for (i, &usage) in cpu_history.iter().enumerate() {
            chart_data.push((i as f64, usage as f64));
        }

        let datasets = vec![
            Dataset::default()
                .name("CPU Usage")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Cyan))
                .data(&chart_data),
        ];

        let chart = Chart::new(datasets)
            .block(Block::default().title("CPU History").borders(Borders::ALL))
            .x_axis(
                Axis::default()
                    .title(Span::styled("Time", Style::default().fg(Color::Red)))
                    .style(Style::default().fg(Color::White))
                    .bounds([0.0, 60.0])
                    .labels(
                        ["60s ago", "30s ago", "now"]
                            .iter()
                            .map(|s| Span::styled(*s, Style::default().fg(Color::White)))
                            .collect(),
                    ),
            )
            .y_axis(
                Axis::default()
                    .title(Span::styled("Usage (%)", Style::default().fg(Color::Red)))
                    .style(Style::default().fg(Color::White))
                    .bounds([0.0, 100.0])
                    .labels(
                        ["0%", "50%", "100%"]
                            .iter()
                            .map(|s| Span::styled(*s, Style::default().fg(Color::White)))
                            .collect(),
                    ),
            );
        f.render_widget(chart, chunks[1]);

        let cpu_block = Block::default().title("CPU Details").borders(Borders::ALL);
        f.render_widget(cpu_block, area);
    }

    fn render_memory(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let memory_block = Block::default()
            .title("Memory Details")
            .borders(Borders::ALL);
        f.render_widget(memory_block, area);
    }

    fn render_disk(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let state = match self.system_state.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
            .split(area);

        let mut total_space = 0;
        let mut used_space = 0;

        for disk in state.disks.list() {
            total_space += disk.total_space();
            used_space += disk.total_space() - disk.available_space();
        }

        let disk_usage_percent = if total_space > 0 {
            (used_space as f64 / total_space as f64 * 100.0) as u16
        } else {
            0
        };

        let disk_guage = Gauge::default()
            .block(
                Block::default()
                    .title("Total Disk Usage")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::Blue))
            .percent(disk_usage_percent);
        f.render_widget(disk_guage, chunks[0]);

        let headers = ["Mount", "Total", "Used", "Available", "Usage %"];
        let header_cells = headers.iter().map(|h| Cell::from(*h));
        let header = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

        let mut rows = Vec::new();
        for disk in state.disks.list() {
            let mount_point = disk.mount_point().to_string_lossy();
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;
            let usage_percent = if total > 0 {
                (used as f64 / total as f64 * 100.0) as u64
            } else {
                0
            };

            let row = Row::new(vec![
                Cell::from(mount_point.to_string()),
                Cell::from(format!("{:.1} GB", total as f64 / 1_000_000_000.0)),
                Cell::from(format!("{:.1} GB", used as f64 / 1_000_000_000.0)),
                Cell::from(format!("{:.1} GB", available as f64 / 1_000_000_000.0)),
                Cell::from(format!("{}%", usage_percent)),
            ]);
            rows.push(row);
        }
        let table = Table::new(rows)
            .header(header)
            .block(Block::default().title("Disk Details").borders(Borders::ALL))
            .widths(&[
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ])
            .highlight_style(Style::default().bg(Color::DarkGray));
        f.render_widget(table, chunks[1]);
        let disk_block = Block::default().title("Disk Details").borders(Borders::ALL);
        f.render_widget(disk_block, area);
    }

    fn render_network(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let network_block = Block::default()
            .title("Network Details")
            .borders(Borders::ALL);
        f.render_widget(network_block, area);
    }

    fn render_processes(
        &self,
        f: &mut tui::Frame<'_, CrosstermBackend<io::Stdout>>,
        area: tui::layout::Rect,
    ) {
        let state = match self.system_state.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let headers = ["PID", "Name", "CPU%", "Memory", "Status"];
        let header_cells = headers.iter().map(|h| Cell::from(*h));
        let header = Row::new(header_cells).style(Style::default().fg(Color::Yellow));

        let mut rows = Vec::new();
        for (pid, process) in state.system.processes() {
            let row = Row::new(vec![
                Cell::from(pid.to_string()),
                Cell::from(process.name().to_string_lossy()),
                Cell::from(format!("{:.1}%", process.cpu_usage())),
                Cell::from(format!("{} MB", process.memory() / 1024 / 1024)),
                Cell::from(format!("{:?}", process.status())),
            ]);
            rows.push(row);
        }

        let constraints = [
            Constraint::Length(7),
            Constraint::Percentage(40),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        let processes_block = Table::new(rows)
            .header(header)
            .block(
                Block::default()
                    .title("Processes Details")
                    .borders(Borders::ALL),
            )
            .widths(&constraints)
            .highlight_style(Style::default().bg(Color::DarkGray));
        f.render_widget(processes_block, area);
    }
}
