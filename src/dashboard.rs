use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::{io, time::Duration};
use tui::{
    Terminal,
    backend::{self, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    terminal,
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph, Tabs},
};

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
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            current_view: DashboardView::Overview,
            should_quit: false,
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
        let cpu_summary = Block::default().title("CPU Summary").borders(Borders::ALL);
        f.render_widget(cpu_summary, chunks[0]);

        let memory_summary = Block::default()
            .title("Memory Summary")
            .borders(Borders::ALL);
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
        let processes_block = Block::default()
            .title("Processes Details")
            .borders(Borders::ALL);
        f.render_widget(processes_block, area);
    }
}
