use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{
    io,
    os::linux::raw::stat,
    sync::{Arc, Mutex},
    time::Duration,
};
use tui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{
        Axis, Block, Borders, Cell, Chart, Dataset, Gauge, GraphType, Paragraph, Row, Table, Tabs,
    },
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

        let mut total_space = 0;
        let mut total_used = 0;
        for disk in state.disks.list() {
            total_space += disk.total_space();
            total_used += disk.total_space() - disk.available_space();
        }
        let disk_percent = if total_space > 0 {
            total_used as f64 / total_space as f64 * 100.0
        } else {
            0.0
        };
        let disk_unit = 1_000_000_000;
        let disk_summary = Paragraph::new(vec![
            Spans::from(format!("Usage: {:.1}%", disk_percent)),
            Spans::from(format!(
                "Used: {:.} GB",
                total_used as f64 / disk_unit as f64
            )),
        ])
        .block(Block::default().title("Disk Summary").borders(Borders::ALL));
        f.render_widget(disk_summary, chunks[2]);

        let (rx_rate, tx_rate) = if state.network_history.len() >= 2 {
            let current = state.network_history.iter().nth_back(0).unwrap();
            let previous = state.network_history.iter().nth_back(1).unwrap();
            (
                current.0.saturating_sub(previous.0),
                current.1.saturating_sub(previous.1),
            )
        } else {
            (0, 0)
        };

        fn format_rate(bytes_per_sec: u64) -> String {
            const KB: f64 = 1024.0;
            const MB: f64 = 1024.0 * KB;
            if bytes_per_sec == 0 {
                return "0 B/s".to_string();
            }
            let rate = bytes_per_sec as f64;
            if rate < KB {
                format!("{} B/s", rate / KB)
            } else {
                format!("{:.1} MB/s", rate / MB)
            }
        }

        let network_summary = Paragraph::new(vec![
            Spans::from(vec![
                Span::styled("Down: ", Style::default().fg(Color::Green)),
                Span::raw(format_rate(rx_rate)),
            ]),
            Spans::from(vec![
                Span::styled("Up: ", Style::default().fg(Color::Red)),
                Span::raw(format_rate(tx_rate)),
            ]),
        ])
        .block(
            Block::default()
                .title("Network Summary")
                .borders(Borders::ALL),
        );
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
                .graph_type(GraphType::Line)
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
        let state_guard = self.system_state.lock();
        let state = match state_guard {
            Ok(ref state) => state,
            Err(_) => {
                f.render_widget(Paragraph::new("Error locking state"), area);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // RAM Gauge
                    Constraint::Length(3), // Swap Gauge
                    Constraint::Min(5),    // Potentially top memory consuming processes
                ]
                .as_ref(),
            )
            .split(area);

        // --- RAM ---
        let mem_total = state.system.total_memory();
        let mem_used = state.system.used_memory();
        let mem_percent = if mem_total > 0 {
            mem_used as f64 / mem_total as f64 * 100.0
        } else {
            0.0
        };
        let mem_unit = 1_024 * 1_024 * 1_024; // GiB

        let ram_gauge = Gauge::default()
            .block(Block::default().title("RAM Usage").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Magenta))
            .percent(mem_percent.round() as u16)
            .label(format!(
                "{:.1}/{:.1} GiB ({:.1}%)",
                mem_used as f64 / mem_unit as f64,
                mem_total as f64 / mem_unit as f64,
                mem_percent
            ));
        f.render_widget(ram_gauge, chunks[0]);

        // --- Swap ---
        let swap_total = state.system.total_swap();
        let swap_used = state.system.used_swap();
        let swap_percent = if swap_total > 0 {
            swap_used as f64 / swap_total as f64 * 100.0
        } else {
            0.0
        };

        let swap_unit = 1_024 * 1_024; // MiB

        let swap_gauge = Gauge::default()
            .block(Block::default().title("Swap Usage").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Yellow))
            .percent(swap_percent.round() as u16)
            .label(format!(
                "{:.0}/{:.0} MiB ({:.1}%)",
                swap_used as f64 / swap_unit as f64,
                swap_total as f64 / swap_unit as f64,
                swap_percent
            ));
        // Only render swap if it exists
        if swap_total > 0 {
            f.render_widget(swap_gauge, chunks[1]);
        } else {
            let no_swap = Paragraph::new("No swap configured")
                .block(Block::default().title("Swap Usage").borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(no_swap, chunks[1]);
        }
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
        let state_guard = self.system_state.lock();
        let state = match state_guard {
            Ok(ref state) => state,
            Err(_) => {
                let error_msg = Paragraph::new("Error: Could not access system state.")
                    .style(Style::default().fg(Color::Red));
                f.render_widget(error_msg, area);
                return;
            }
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3),      // Current Rates Summary
                    Constraint::Percentage(50), // Network History Chart
                    Constraint::Min(5),         // Interface Details Table
                ]
                .as_ref(),
            )
            .split(area);

        let rate_area = chunks[0];
        let chart_area = chunks[1];
        let table_area = chunks[2];

        let (rx_rate, tx_rate) = if state.network_history.len() >= 2 {
            let current = state.network_history.iter().nth_back(0).unwrap();
            let previous = state.network_history.iter().nth_back(1).unwrap();
            (
                current.0.saturating_sub(previous.0),
                current.1.saturating_sub(previous.1),
            )
        } else {
            (0, 0)
        };

        fn format_rate(bytes_per_sec: u64) -> String {
            const KB: f64 = 1024.0;
            const MB: f64 = 1024.0 * KB;
            if bytes_per_sec == 0 {
                return "0 B/s".to_string();
            }
            let rate = bytes_per_sec as f64;
            if rate < KB {
                format!("{} B/s", rate / KB)
            } else {
                format!("{:.1} MB/s", rate / MB)
            }
        }

        let network_summary = Paragraph::new(vec![Spans::from(vec![
            Span::styled("Down: ", Style::default().fg(Color::Green)),
            Span::raw(format_rate(tx_rate)),
        ])])
        .block(
            Block::default()
                .title("Current Traffic Rate")
                .borders(Borders::ALL),
        )
        .alignment(tui::layout::Alignment::Center);
        f.render_widget(network_summary, rate_area);

        let network_history = &state.network_history;

        let mut rx_data: Vec<(f64, f64)> = Vec::new();
        let mut tx_data: Vec<(f64, f64)> = Vec::new();

        for i in 1..network_history.len() {
            let current = network_history[i];
            let prev = network_history[i - 1];

            let rx_rate_bps = current.0.saturating_sub(prev.0);
            let tx_rate_bps = current.1.saturating_sub(prev.1);

            rx_data.push((i as f64, rx_rate_bps as f64 / 1024.0));
            tx_data.push((i as f64, tx_rate_bps as f64 / 1024.0));
        }

        let datasets = vec![
            Dataset::default()
                .name("Download (KB/s)")
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Green))
                .data(&rx_data),
            Dataset::default()
                .name("Upload (KB/s)")
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Red))
                .data(&tx_data),
        ];

        let max_rate_kbps = rx_data
            .iter()
            .chain(tx_data.iter())
            .map(|&(_, v)| v)
            .fold(0.0, |max, v| if v.is_finite() && v > max { v } else { max });

        // Set a minimum top bound for the Y axis, e.g., 10 KB/s, and add headroom
        let y_bound_top = (max_rate_kbps * 1.1).max(10.0);
        let history_len = state.network_history.len() as f64;

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title("Network History (KB/s)")
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    // .title("Time") // Often redundant
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, history_len]) // X represents time steps
                    .labels(vec![
                        Span::styled(
                            format!("{}s", history_len.round()),
                            Style::default().fg(Color::Gray),
                        ), // Start label (oldest)
                        Span::styled("0s", Style::default().fg(Color::Gray)), // End label (now)
                    ]),
            )
            .y_axis(
                Axis::default()
                    .title("KB/s")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, y_bound_top]) // Dynamic upper bound
                    .labels(
                        // Generate labels dynamically based on the top bound
                        vec![
                            Span::raw("0"),
                            Span::raw(format!("{:.0}", y_bound_top / 2.0)),
                            Span::raw(format!("{:.0}", y_bound_top)),
                        ],
                    ),
            );
        f.render_widget(chart, chart_area);

        let headers = ["Interface Name", "Total Recived", "Total Transmitted"];
        let header_cells = headers
            .iter()
            .map(|h| Cell::from(Span::styled(*h, Style::default().fg(Color::Yellow))));
        let header = Row::new(header_cells)
            .style(Style::default().bg(Color::DarkGray))
            .height(1);

        fn format_total_bytes(bytes: u64) -> String {
            const MB: f64 = 1_000_000.0;
            const GB: f64 = 1_000.0 * MB;
            if bytes == 0 {
                return "0 B".to_string();
            }
            let b = bytes as f64;
            if b < MB {
                format!("{:.1} KB", b / 1000.0)
            } else if b < GB {
                format!("{:.2} MB", b / MB)
            } else {
                format!("{:.2} GB", b / GB)
            }
        }

        let mut rows = Vec::new();
        for (interface_name, data) in state.networks.list() {
            let row = Row::new(vec![
                Cell::from(interface_name.clone()),
                Cell::from(format_total_bytes(data.total_received())),
                Cell::from(format_total_bytes(data.total_transmitted())),
            ]);
            rows.push(row);
        }

        let table = Table::new(rows)
            .header(header)
            .block(
                Block::default()
                    .title("Network Interfaces (Total Data)")
                    .borders(Borders::ALL),
            )
            .widths(&[
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ])
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(">> ");
        f.render_widget(table, table_area);
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
