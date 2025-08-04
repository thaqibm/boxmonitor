use crate::monitor::TargetStats;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{BarChart, Block, Borders, List, ListItem, Paragraph},
};
use std::collections::HashMap;

pub fn render_all_targets_failure_chart(f: &mut Frame, area: Rect, targets: &[TargetStats]) {
    if targets.is_empty() {
        let block = Block::default()
            .title("Failure Analysis")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No targets available").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // Split the area into two parts: bar chart on left, failure log on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Aggregate failure reasons across all targets
    let mut failure_counts: HashMap<String, u64> = HashMap::new();
    let mut all_failures = Vec::new();

    for target in targets {
        for failure in &target.failure_log {
            *failure_counts.entry(failure.reason.clone()).or_insert(0) += 1;
            let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
            all_failures.push((
                failure.timestamp,
                target_name.clone(),
                failure.failure_type.clone(),
                failure.reason.clone(),
            ));
        }
    }

    if failure_counts.is_empty() {
        let block = Block::default()
            .title("Failure Analysis - Press 'p' to cycle views")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No failures recorded").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // Render bar chart
    render_failure_bar_chart(f, chunks[0], &failure_counts);

    // Render failure log
    render_failure_log(f, chunks[1], &all_failures);
}

fn render_failure_bar_chart(f: &mut Frame, area: Rect, failure_counts: &HashMap<String, u64>) {
    // Convert to sorted vector for bar chart
    let mut failure_data: Vec<(String, u64)> = failure_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    failure_data.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
    failure_data.truncate(6); // Show top 6 failures to fit better with longer labels

    // Truncate long failure reasons for display but keep them readable
    let bar_data: Vec<(String, u64)> = failure_data
        .iter()
        .map(|(reason, count)| {
            let truncated = if reason.len() > 25 {
                format!("{}...", &reason[..22])
            } else {
                reason.clone()
            };
            (truncated, *count)
        })
        .collect();

    let bar_data_refs: Vec<(&str, u64)> = bar_data
        .iter()
        .map(|(reason, count)| (reason.as_str(), *count))
        .collect();

    let max_count = failure_data
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1);

    let barchart = BarChart::default()
        .block(
            Block::default()
                .title("Top Failure Reasons")
                .borders(Borders::ALL),
        )
        .data(&bar_data_refs)
        .bar_width(3)
        .bar_gap(2) // Add spacing between bars
        .bar_style(Style::default().fg(Color::Red))
        .value_style(Style::default().fg(Color::Black).bg(Color::Red))
        .max(max_count);

    f.render_widget(barchart, area);
}

fn render_failure_log(
    f: &mut Frame,
    area: Rect,
    failures: &[(chrono::DateTime<chrono::Utc>, String, String, String)],
) {
    // Sort failures by timestamp (most recent first)
    let mut sorted_failures = failures.to_vec();
    sorted_failures.sort_by(|a, b| b.0.cmp(&a.0));
    sorted_failures.truncate(20); // Show last 20 failures

    let items: Vec<ListItem> = sorted_failures
        .iter()
        .map(|(timestamp, target, failure_type, reason)| {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let content = format!("{} [{}] {}: {}", time_str, target, failure_type, reason);
            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title("Recent Failures")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(list, area);
}

pub fn render_single_target_failure_chart(f: &mut Frame, area: Rect, target: &TargetStats) {
    if target.failure_log.is_empty() {
        let block = Block::default()
            .title("Failure Analysis - Press 'p' to cycle views")
            .borders(Borders::ALL);
        let paragraph = Paragraph::new("No failures recorded for this target").block(block);
        f.render_widget(paragraph, area);
        return;
    }

    // Split the area into two parts: bar chart on left, failure log on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Count failure reasons for this target
    let mut failure_counts: HashMap<String, u64> = HashMap::new();
    let mut target_failures = Vec::new();

    for failure in &target.failure_log {
        *failure_counts.entry(failure.reason.clone()).or_insert(0) += 1;
        let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
        target_failures.push((
            failure.timestamp,
            target_name.clone(),
            failure.failure_type.clone(),
            failure.reason.clone(),
        ));
    }

    // Render bar chart
    render_single_target_bar_chart(f, chunks[0], &failure_counts, target);

    // Render failure log
    render_failure_log(f, chunks[1], &target_failures);
}

fn render_single_target_bar_chart(
    f: &mut Frame,
    area: Rect,
    failure_counts: &HashMap<String, u64>,
    target: &TargetStats,
) {
    // Convert to sorted vector for bar chart
    let mut failure_data: Vec<(String, u64)> = failure_counts
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    failure_data.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
    failure_data.truncate(6); // Show top 6 failures to fit better with longer labels

    // Truncate long failure reasons for display but keep them readable
    let bar_data: Vec<(String, u64)> = failure_data
        .iter()
        .map(|(reason, count)| {
            let truncated = if reason.len() > 25 {
                format!("{}...", &reason[..22])
            } else {
                reason.clone()
            };
            (truncated, *count)
        })
        .collect();

    let bar_data_refs: Vec<(&str, u64)> = bar_data
        .iter()
        .map(|(reason, count)| (reason.as_str(), *count))
        .collect();

    let max_count = failure_data
        .iter()
        .map(|(_, count)| *count)
        .max()
        .unwrap_or(1);
    let target_name = target.target.name.as_ref().unwrap_or(&target.target.ip);
    let title = format!("Failures for {}", target_name);

    let barchart = BarChart::default()
        .block(Block::default().title(title).borders(Borders::ALL))
        .data(&bar_data_refs)
        .bar_width(3)
        .bar_gap(2) // Add spacing between bars
        .bar_style(Style::default().fg(Color::Red))
        .value_style(Style::default().fg(Color::Black).bg(Color::Red))
        .max(max_count);

    f.render_widget(barchart, area);
}
