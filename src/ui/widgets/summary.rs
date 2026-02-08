use std::collections::HashMap;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::AppState;
use crate::model::FileType;
use crate::ui::theme;

pub fn render(frame: &mut Frame, state: &AppState, area: Rect) {
    let proc = match &state.selected_process {
        Some(p) => p,
        None => return,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Process info
            Constraint::Length(1), // Separator
            Constraint::Min(3),   // FD type stats
        ])
        .split(area);

    // Process info section
    let info_lines = vec![
        Line::from(vec![
            Span::styled("  PID:     ", theme::header_style()),
            Span::styled(proc.pid.to_string(), theme::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("  Command: ", theme::header_style()),
            Span::styled(&proc.comm, theme::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("  User:    ", theme::header_style()),
            Span::styled(&proc.user, theme::normal_style()),
        ]),
        Line::from(vec![
            Span::styled("  PPID:    ", theme::header_style()),
            Span::styled(
                proc.ppid.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
                theme::normal_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Total FDs: ", theme::header_style()),
            Span::styled(proc.open_files.len().to_string(), theme::normal_style()),
        ]),
    ];
    frame.render_widget(Paragraph::new(info_lines), chunks[0]);

    // Separator
    let sep = Paragraph::new(Line::from(Span::styled(
        "  --- FD Type Statistics ---",
        theme::status_style(),
    )));
    frame.render_widget(sep, chunks[1]);

    // FD type statistics — collect (type_name, count, style) tuples
    let mut type_counts: HashMap<String, (usize, ratatui::style::Style)> = HashMap::new();
    let mut total_size: u64 = 0;

    for f in &proc.open_files {
        let entry = type_counts
            .entry(f.file_type.to_string())
            .or_insert((0, theme::file_type_style(&f.file_type)));
        entry.0 += 1;
        if let Some(size) = f.size_off {
            total_size += size;
        }
    }

    let mut sorted_types: Vec<_> = type_counts.into_iter().collect();
    sorted_types.sort_by(|a, b| (b.1).0.cmp(&(a.1).0));

    let mut stat_lines: Vec<Line> = Vec::new();
    for (type_name, (count, style)) in &sorted_types {
        stat_lines.push(Line::from(vec![
            Span::styled(format!("  {:<10}", type_name), *style),
            Span::styled(format!("{:>6}", count), theme::normal_style()),
        ]));
    }

    // Network file count
    let net_count = proc
        .open_files
        .iter()
        .filter(|f| matches!(
            f.file_type,
            FileType::IPv4 | FileType::IPv6 | FileType::Sock | FileType::Unix
        ))
        .count();

    stat_lines.push(Line::from(Span::raw("")));
    stat_lines.push(Line::from(vec![
        Span::styled("  Network:   ", theme::header_style()),
        Span::styled(format!("{}", net_count), theme::normal_style()),
    ]));
    stat_lines.push(Line::from(vec![
        Span::styled("  Disk used: ", theme::header_style()),
        Span::styled(format_size(total_size), theme::normal_style()),
    ]));

    frame.render_widget(Paragraph::new(stat_lines), chunks[2]);
}

fn format_size(bytes: u64) -> String {
    if bytes == 0 {
        "0".to_string()
    } else if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
