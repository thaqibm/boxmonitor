use crate::monitor::{Statistics, TargetStats};
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, Paragraph, Tabs},
};
use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Copy, PartialEq)]
pub enum PlotView {
    AllTargets,
    PingOnly,
    SshOnly,
}

#[derive(Clone, Copy, PartialEq)]
pub enum TabMode {
    AllTargets,
    Individual(usize),
}

pub struct App {
    pub should_quit: bool,
    pub current_tab: usize,
    pub current_plot_view: PlotView,
    pub tab_mode: TabMode,
    pub targets: Arc<Mutex<Vec<TargetStats>>>,
}

impl App {
    pub fn new(targets: Arc<Mutex<Vec<TargetStats>>>) -> Self {
        Self {
            should_quit: false,
            current_tab: 0,
            current_plot_view: PlotView::AllTargets,
            tab_mode: TabMode::AllTargets,
            targets,
        }
    }

    pub fn next_tab(&mut self, max_tabs: usize) {
        let total_tabs = max_tabs + 1; // +1 for "All Targets" tab
        self.current_tab = (self.current_tab + 1) % total_tabs;
        self.update_tab_mode(max_tabs);
    }

    pub fn previous_tab(&mut self, max_tabs: usize) {
        let total_tabs = max_tabs + 1; // +1 for "All Targets" tab
        if self.current_tab > 0 {
            self.current_tab -= 1;
        } else {
            self.current_tab = total_tabs - 1;
        }
        self.update_tab_mode(max_tabs);
    }

    fn update_tab_mode(&mut self, _max_targets: usize) {
        if self.current_tab == 0 {
            self.tab_mode = TabMode::AllTargets;
        } else {
            self.tab_mode = TabMode::Individual(self.current_tab - 1);
        }
    }

    pub fn next_plot_view(&mut self, has_ssh: bool) {
        self.current_plot_view = match self.current_plot_view {
            PlotView::AllTargets => PlotView::PingOnly,
            PlotView::PingOnly => {
                if has_ssh {
                    PlotView::SshOnly
                } else {
                    PlotView::AllTargets
                }
            }
            PlotView::SshOnly => PlotView::AllTargets,
        };
    }
}

pub async fn run_ui(targets: Arc<Mutex<Vec<TargetStats>>>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(targets);
    let res = run_app(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        let targets = app.targets.lock().await;
        terminal.draw(|f| ui(f, app, &targets))?;
        drop(targets);

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Tab => {
                            let target_count = {
                                let targets = app.targets.lock().await;
                                targets.len()
                            };
                            app.next_tab(target_count);
                        }
                        KeyCode::BackTab => {
                            let target_count = {
                                let targets = app.targets.lock().await;
                                targets.len()
                            };
                            app.previous_tab(target_count);
                        }
                        KeyCode::Char('p') => {
                            let has_ssh = {
                                let targets = app.targets.lock().await;
                                match app.tab_mode {
                                    TabMode::AllTargets => {
                                        targets.iter().any(|t| t.target.ssh_port.is_some())
                                    }
                                    TabMode::Individual(idx) => {
                                        if let Some(target) = targets.get(idx) {
                                            target.target.ssh_port.is_some()
                                        } else {
                                            false
                                        }
                                    }
                                }
                            };
                            app.next_plot_view(has_ssh);
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

fn ui(f: &mut Frame, app: &App, targets: &[TargetStats]) {
    let size = f.area();

    if targets.is_empty() {
        let block = Block::default().title("Box Monitor").borders(Borders::ALL);
        let paragraph = Paragraph::new("No targets configured. Check ~/.config/box/.iplist")
            .block(block)
            .style(Style::default().fg(Color::Red));
        f.render_widget(paragraph, size);
        return;
    }

    let mut tab_titles: Vec<Line> = vec![Line::from(vec![Span::raw("All Targets")])];
    tab_titles.extend(targets.iter().map(|target| {
        let name = target.target.name.as_ref().unwrap_or(&target.target.ip);
        Line::from(vec![Span::raw(name)])
    }));

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().title("Targets").borders(Borders::ALL))
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow))
        .select(app.current_tab);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(size);

    f.render_widget(tabs, chunks[0]);

    match app.tab_mode {
        TabMode::AllTargets => {
            render_all_targets_view(f, chunks[1], targets, app.current_plot_view);
        }
        TabMode::Individual(idx) => {
            if let Some(target) = targets.get(idx) {
                render_target_details(f, chunks[1], target, app.current_plot_view);
            }
        }
    }
}

fn render_all_targets_view(
    f: &mut Frame,
    area: Rect,
    targets: &[TargetStats],
    plot_view: PlotView,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(area);

    render_all_targets_info(f, chunks[0], targets);
    render_all_targets_charts(f, chunks[1], targets, plot_view);
}

fn render_target_details(f: &mut Frame, area: Rect, target: &TargetStats, plot_view: PlotView) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(8),
            Constraint::Min(10),
        ])
        .split(area);

    render_target_info(f, chunks[0], target);
    render_statistics(f, chunks[1], target);
    render_single_target_charts(f, chunks[2], target, plot_view);
}

fn render_target_info(f: &mut Frame, area: Rect, target: &TargetStats) {
    let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);

    let info_text = vec![Line::from(vec![
        Span::raw("Target: "),
        Span::styled(target_name, Style::default().fg(Color::Cyan)),
        Span::raw(" ("),
        Span::raw(&target.target.ip),
        Span::raw(")"),
    ])];

    let paragraph = Paragraph::new(info_text)
        .block(Block::default().title("Target Info").borders(Borders::ALL));
    f.render_widget(paragraph, area);
}

fn render_statistics(f: &mut Frame, area: Rect, target: &TargetStats) {
    let has_ssh = target.target.ssh_port.is_some();

    let chunks = if has_ssh {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(area)
    };

    if let Some(ping_stats) = &target.ping_stats {
        render_ping_stats(f, chunks[0], ping_stats);
    } else {
        let block = Block::default().title("Ping Stats").borders(Borders::ALL);
        let paragraph = Paragraph::new("No ping data available").block(block);
        f.render_widget(paragraph, chunks[0]);
    }

    if has_ssh {
        if let Some(ssh_stats) = &target.ssh_stats {
            render_ssh_stats(f, chunks[1], ssh_stats);
        } else {
            let block = Block::default().title("SSH Stats").borders(Borders::ALL);
            let paragraph = Paragraph::new("No SSH data available").block(block);
            f.render_widget(paragraph, chunks[1]);
        }
    }
}

fn render_ping_stats(f: &mut Frame, area: Rect, stats: &Statistics) {
    let items = vec![
        ListItem::new(format!("Mean: {:.2}ms", stats.mean)),
        ListItem::new(format!("Median: {:.2}ms", stats.median)),
        ListItem::new(format!("Min/Max: {:.2}/{:.2}ms", stats.min, stats.max)),
        ListItem::new(format!("P95: {:.2}ms", stats.p95)),
        ListItem::new(format!("Success: {:.1}%", stats.success_rate)),
    ];

    let list = List::new(items)
        .block(Block::default().title("Ping Stats").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_ssh_stats(f: &mut Frame, area: Rect, stats: &Statistics) {
    let items = vec![
        ListItem::new(format!("Mean: {:.2}ms", stats.mean)),
        ListItem::new(format!("Median: {:.2}ms", stats.median)),
        ListItem::new(format!("Min/Max: {:.2}/{:.2}ms", stats.min, stats.max)),
        ListItem::new(format!("P95: {:.2}ms", stats.p95)),
        ListItem::new(format!("Success: {:.2}%", stats.success_rate)),
    ];

    let list = List::new(items)
        .block(Block::default().title("SSH Stats").borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

fn render_all_targets_info(f: &mut Frame, area: Rect, targets: &[TargetStats]) {
    let info_text = vec![Line::from(vec![
        Span::raw("Monitoring "),
        Span::styled(
            format!("{} targets", targets.len()),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" - Use Tab/Shift+Tab to switch views, 'p' to cycle plot types"),
    ])];

    let paragraph = Paragraph::new(info_text).block(
        Block::default()
            .title("All Targets Overview")
            .borders(Borders::ALL),
    );
    f.render_widget(paragraph, area);
}

fn render_all_targets_charts(
    f: &mut Frame,
    area: Rect,
    targets: &[TargetStats],
    plot_view: PlotView,
) {
    match plot_view {
        PlotView::AllTargets => {
            render_all_targets_overlay_chart(f, area, targets);
        }
        PlotView::PingOnly => {
            render_all_targets_ping_chart(f, area, targets);
        }
        PlotView::SshOnly => {
            render_all_targets_ssh_chart(f, area, targets);
        }
    }
}

fn render_single_target_charts(
    f: &mut Frame,
    area: Rect,
    target: &TargetStats,
    plot_view: PlotView,
) {
    let has_ssh = target.target.ssh_port.is_some();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    match plot_view {
        PlotView::AllTargets => {
            render_overlay_chart(f, chunks[0], target);
        }
        PlotView::PingOnly => {
            render_ping_chart(f, chunks[0], target);
        }
        PlotView::SshOnly => {
            if has_ssh {
                render_ssh_chart(f, chunks[0], target);
            } else {
                let block = Block::default().title("SSH Chart").borders(Borders::ALL);
                let paragraph = Paragraph::new("SSH monitoring not configured").block(block);
                f.render_widget(paragraph, chunks[0]);
            }
        }
    }

    render_box_plot(f, chunks[1], target);
}

fn render_overlay_chart(f: &mut Frame, area: Rect, target: &TargetStats) {
    let has_ssh = target.target.ssh_port.is_some();

    if target.ping_history.is_empty() && (!has_ssh || target.ssh_history.is_empty()) {
        let block = Block::default()
            .title("Latency Overlay")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No data available").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let mut datasets = Vec::new();
    let mut max_latency: f64 = 0.0;
    let mut min_latency = f64::INFINITY;
    let mut max_length = 0;

    let ssh_data: Vec<(f64, f64)>;
    let ping_data: Vec<(f64, f64)>;
    // Ping data
    if !target.ping_history.is_empty() {
        ping_data = target
            .ping_history
            .iter()
            .enumerate()
            .filter_map(|(i, result)| result.latency_ms.map(|latency| (i as f64, latency)))
            .collect();

        if !ping_data.is_empty() {
            max_latency = max_latency.max(ping_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
            min_latency = min_latency.min(
                ping_data
                    .iter()
                    .map(|(_, y)| *y)
                    .fold(f64::INFINITY, f64::min),
            );
            max_length = max_length.max(target.ping_history.len());

            datasets.push(
                Dataset::default()
                    .name("Ping")
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Green))
                    .graph_type(GraphType::Line)
                    .data(&ping_data),
            );
        }
    }
    // SSH data
    if has_ssh && !target.ssh_history.is_empty() {
        ssh_data = target
            .ssh_history
            .iter()
            .enumerate()
            .filter_map(|(i, result)| result.connection_time_ms.map(|time| (i as f64, time)))
            .collect();

        if !ssh_data.is_empty() {
            max_latency = max_latency.max(ssh_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
            min_latency = min_latency.min(
                ssh_data
                    .iter()
                    .map(|(_, y)| *y)
                    .fold(f64::INFINITY, f64::min),
            );
            max_length = max_length.max(target.ssh_history.len());

            datasets.push(
                Dataset::default()
                    .name("SSH")
                    .marker(symbols::Marker::Braille)
                    .style(Style::default().fg(Color::Blue))
                    .graph_type(GraphType::Line)
                    .data(&ssh_data),
            );
        }
    }

    if datasets.is_empty() {
        let block = Block::default()
            .title("Latency Overlay")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("All connections failed").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let y_max = max_latency * 1.1;
    let y_min = min_latency.min(0.0);
    let x_max = max_length as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("Latency Overlay (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Latency (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}

fn render_ping_chart(f: &mut Frame, area: Rect, target: &TargetStats) {
    if target.ping_history.is_empty() {
        let block = Block::default().title("Ping Latency").borders(Borders::ALL);
        let paragraph = Paragraph::new("No ping data yet...").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let ping_data: Vec<(f64, f64)> = target
        .ping_history
        .iter()
        .enumerate()
        .filter_map(|(i, result)| result.latency_ms.map(|latency| (i as f64, latency)))
        .collect();

    if ping_data.is_empty() {
        let block = Block::default().title("Ping Latency").borders(Borders::ALL);
        let paragraph = Paragraph::new("All pings failed").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let max_latency = ping_data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let min_latency = ping_data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);

    let datasets = vec![
        Dataset::default()
            .name("Ping")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Green))
            .graph_type(GraphType::Line)
            .data(&ping_data),
    ];

    let y_max = max_latency * 1.1;
    let y_min = min_latency.min(0.0);
    let x_max = target.ping_history.len() as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("Ping Latency (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Latency (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}

fn render_box_plot(f: &mut Frame, area: Rect, target: &TargetStats) {
    if let Some(stats) = &target.ping_stats {
        let box_data = vec![
            (0.0, stats.min),
            (1.0, stats.p25),
            (2.0, stats.median),
            (3.0, stats.p75),
            (4.0, stats.p90),
            (5.0, stats.max),
        ];

        let outlier_data = vec![(6.0, stats.p95), (7.0, stats.p99)];

        let datasets = vec![
            Dataset::default()
                .name("Box Plot")
                .marker(symbols::Marker::Block)
                .style(Style::default().fg(Color::Cyan))
                .graph_type(GraphType::Line)
                .data(&box_data),
            Dataset::default()
                .name("Outliers")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Red))
                .graph_type(GraphType::Scatter)
                .data(&outlier_data),
        ];

        let x_labels = vec!["Min", "P25", "P50", "P75", "P90", "Max", "P95", "P99"];
        let y_max = stats.max.max(stats.p99) * 1.1;
        let y_min = stats.min * 0.9;

        let y_labels: Vec<String> = (0..=5)
            .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
            .collect();

        let chart = Chart::new(datasets)
            .block(
                Block::default()
                    .title("Ping Latency Box Plot (ms)")
                    .borders(Borders::ALL),
            )
            .x_axis(
                Axis::default()
                    .title("Quartiles & Percentiles")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 7.0])
                    .labels(x_labels.iter().map(|s| *s).collect::<Vec<_>>()),
            )
            .y_axis(
                Axis::default()
                    .title("Latency (ms)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([y_min, y_max])
                    .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
            );

        f.render_widget(chart, area);
    } else {
        let block = Block::default()
            .title("Ping Latency Box Plot")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No ping data available for box plot").block(block);
        f.render_widget(paragraph, area);
    }
}

fn render_all_targets_overlay_chart(f: &mut Frame, area: Rect, targets: &[TargetStats]) {
    if targets.is_empty() {
        let block = Block::default()
            .title("All Targets Overlay")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No targets available").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let mut all_data = Vec::new();
    let mut all_names = Vec::new();
    let mut all_colors = Vec::new();
    let mut all_markers = Vec::new();
    let mut max_latency: f64 = 0.0;
    let mut min_latency = f64::INFINITY;
    let mut max_length = 0;

    // Define colors for different targets
    let colors = [
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::Red,
        Color::LightGreen,
        Color::LightBlue,
        Color::LightYellow,
        Color::LightMagenta,
        Color::LightCyan,
        Color::LightRed,
    ];

    for (target_idx, target) in targets.iter().enumerate() {
        let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
        let color = colors[target_idx % colors.len()];

        // Ping data for this target
        if !target.ping_history.is_empty() {
            let ping_data: Vec<(f64, f64)> = target
                .ping_history
                .iter()
                .enumerate()
                .filter_map(|(i, result)| result.latency_ms.map(|latency| (i as f64, latency)))
                .collect();

            if !ping_data.is_empty() {
                max_latency =
                    max_latency.max(ping_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
                min_latency = min_latency.min(
                    ping_data
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::INFINITY, f64::min),
                );
                max_length = max_length.max(target.ping_history.len());

                all_data.push(ping_data);
                all_names.push(format!("{} (Ping)", target_name));
                all_colors.push(color);
                all_markers.push(symbols::Marker::Braille);
            }
        }

        // SSH data for this target
        if target.target.ssh_port.is_some() && !target.ssh_history.is_empty() {
            let ssh_data: Vec<(f64, f64)> = target
                .ssh_history
                .iter()
                .enumerate()
                .filter_map(|(i, result)| result.connection_time_ms.map(|time| (i as f64, time)))
                .collect();

            if !ssh_data.is_empty() {
                max_latency = max_latency.max(ssh_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
                min_latency = min_latency.min(
                    ssh_data
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::INFINITY, f64::min),
                );
                max_length = max_length.max(target.ssh_history.len());

                // Use dashed line style for SSH by alternating color intensity
                let ssh_color = match color {
                    Color::Green => Color::LightGreen,
                    Color::Blue => Color::LightBlue,
                    Color::Yellow => Color::LightYellow,
                    Color::Magenta => Color::LightMagenta,
                    Color::Cyan => Color::LightCyan,
                    Color::Red => Color::LightRed,
                    _ => Color::White,
                };

                all_data.push(ssh_data);
                all_names.push(format!("{} (SSH)", target_name));
                all_colors.push(ssh_color);
                all_markers.push(symbols::Marker::Dot);
            }
        }
    }

    if all_data.is_empty() {
        let block = Block::default()
            .title("All Targets Overlay")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No data available for any target").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let datasets: Vec<Dataset> = all_data
        .iter()
        .zip(all_names.iter())
        .zip(all_colors.iter())
        .zip(all_markers.iter())
        .map(|(((data, name), color), marker)| {
            Dataset::default()
                .name(name.as_str())
                .marker(*marker)
                .style(Style::default().fg(*color))
                .graph_type(GraphType::Line)
                .data(data)
        })
        .collect();

    let y_max = max_latency * 1.1;
    let y_min = min_latency.min(0.0);
    let x_max = max_length as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("All Targets Latency Overlay (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Latency (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}

fn render_all_targets_ping_chart(f: &mut Frame, area: Rect, targets: &[TargetStats]) {
    if targets.is_empty() {
        let block = Block::default()
            .title("All Targets Ping")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No targets available").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let mut all_data = Vec::new();
    let mut all_names = Vec::new();
    let mut all_colors = Vec::new();
    let mut max_latency: f64 = 0.0;
    let mut min_latency = f64::INFINITY;
    let mut max_length = 0;

    let colors = [
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::Red,
        Color::LightGreen,
        Color::LightBlue,
        Color::LightYellow,
        Color::LightMagenta,
        Color::LightCyan,
        Color::LightRed,
    ];

    for (target_idx, target) in targets.iter().enumerate() {
        let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
        let color = colors[target_idx % colors.len()];

        if !target.ping_history.is_empty() {
            let ping_data: Vec<(f64, f64)> = target
                .ping_history
                .iter()
                .enumerate()
                .filter_map(|(i, result)| result.latency_ms.map(|latency| (i as f64, latency)))
                .collect();

            if !ping_data.is_empty() {
                max_latency =
                    max_latency.max(ping_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
                min_latency = min_latency.min(
                    ping_data
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::INFINITY, f64::min),
                );
                max_length = max_length.max(target.ping_history.len());

                all_data.push(ping_data);
                all_names.push(target_name.to_string());
                all_colors.push(color);
            }
        }
    }

    if all_data.is_empty() {
        let block = Block::default()
            .title("All Targets Ping")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No ping data available for any target").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let datasets: Vec<Dataset> = all_data
        .iter()
        .zip(all_names.iter())
        .zip(all_colors.iter())
        .map(|((data, name), color)| {
            Dataset::default()
                .name(name.as_str())
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(*color))
                .graph_type(GraphType::Line)
                .data(data)
        })
        .collect();

    let y_max = max_latency * 1.1;
    let y_min = min_latency.min(0.0);
    let x_max = max_length as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("All Targets Ping Latency (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Latency (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}

fn render_all_targets_ssh_chart(f: &mut Frame, area: Rect, targets: &[TargetStats]) {
    if targets.is_empty() {
        let block = Block::default()
            .title("All Targets SSH")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No targets available").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let mut all_data = Vec::new();
    let mut all_names = Vec::new();
    let mut all_colors = Vec::new();
    let mut max_latency: f64 = 0.0;
    let mut min_latency = f64::INFINITY;
    let mut max_length = 0;

    let colors = [
        Color::Green,
        Color::Blue,
        Color::Yellow,
        Color::Magenta,
        Color::Cyan,
        Color::Red,
        Color::LightGreen,
        Color::LightBlue,
        Color::LightYellow,
        Color::LightMagenta,
        Color::LightCyan,
        Color::LightRed,
    ];

    for (target_idx, target) in targets.iter().enumerate() {
        let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
        let color = colors[target_idx % colors.len()];

        if target.target.ssh_port.is_some() && !target.ssh_history.is_empty() {
            let ssh_data: Vec<(f64, f64)> = target
                .ssh_history
                .iter()
                .enumerate()
                .filter_map(|(i, result)| result.connection_time_ms.map(|time| (i as f64, time)))
                .collect();

            if !ssh_data.is_empty() {
                max_latency = max_latency.max(ssh_data.iter().map(|(_, y)| *y).fold(0.0, f64::max));
                min_latency = min_latency.min(
                    ssh_data
                        .iter()
                        .map(|(_, y)| *y)
                        .fold(f64::INFINITY, f64::min),
                );
                max_length = max_length.max(target.ssh_history.len());

                all_data.push(ssh_data);
                all_names.push(target_name.to_string());
                all_colors.push(color);
            }
        }
    }

    if all_data.is_empty() {
        let block = Block::default()
            .title("All Targets SSH")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No SSH data available for any target").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let datasets: Vec<Dataset> = all_data
        .iter()
        .zip(all_names.iter())
        .zip(all_colors.iter())
        .map(|((data, name), color)| {
            Dataset::default()
                .name(name.as_str())
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(*color))
                .graph_type(GraphType::Line)
                .data(data)
        })
        .collect();

    let y_max = max_latency * 1.1;
    let y_min = min_latency.min(0.0);
    let x_max = max_length as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("All Targets SSH Connection Time (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Connection Time (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}

fn render_ssh_chart(f: &mut Frame, area: Rect, target: &TargetStats) {
    if target.ssh_history.is_empty() {
        let block = Block::default()
            .title("SSH Connection Time")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No SSH data yet...").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let ssh_data: Vec<(f64, f64)> = target
        .ssh_history
        .iter()
        .enumerate()
        .filter_map(|(i, result)| result.connection_time_ms.map(|time| (i as f64, time)))
        .collect();

    if ssh_data.is_empty() {
        let block = Block::default()
            .title("SSH Connection Time")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("All SSH connections failed").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    let max_time = ssh_data.iter().map(|(_, y)| *y).fold(0.0, f64::max);
    let min_time = ssh_data
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::INFINITY, f64::min);

    let datasets = vec![
        Dataset::default()
            .name("SSH")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Blue))
            .graph_type(GraphType::Line)
            .data(&ssh_data),
    ];

    let y_max = max_time * 1.1;
    let y_min = min_time.min(0.0);
    let x_max = target.ssh_history.len() as f64;

    let y_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.1}", y_min + (y_max - y_min) * i as f64 / 5.0))
        .collect();

    let x_labels: Vec<String> = (0..=5)
        .map(|i| format!("{:.0}", x_max * i as f64 / 5.0))
        .collect();

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title("SSH Connection Time (ms) - Press 'p' to cycle views")
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time (samples)")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, x_max])
                .labels(x_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        )
        .y_axis(
            Axis::default()
                .title("Connection Time (ms)")
                .style(Style::default().fg(Color::Gray))
                .bounds([y_min, y_max])
                .labels(y_labels.iter().map(|s| s.as_str()).collect::<Vec<_>>()),
        );

    f.render_widget(chart, area);
}
