use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::AppState;
use crate::app::state::DetailTab;
use super::theme;
use super::widgets;

pub fn render(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Process header
            Constraint::Length(1), // Tab bar
            Constraint::Min(3),   // Content area
            Constraint::Length(1), // Status line
        ])
        .split(area);

    render_header(frame, state, chunks[0]);
    render_tab_bar(frame, state, chunks[1]);
    render_content(frame, state, chunks[2]);
    render_status(frame, chunks[3]);
}

fn render_header(frame: &mut Frame, state: &AppState, area: Rect) {
    let proc = match &state.selected_process {
        Some(p) => p,
        None => return,
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(" PID ", theme::header_style()),
            Span::styled(format!("{}", proc.pid), theme::normal_style()),
            Span::styled("  CMD ", theme::header_style()),
            Span::styled(&proc.comm, theme::normal_style()),
            Span::styled("  USER ", theme::header_style()),
            Span::styled(&proc.user, theme::normal_style()),
            Span::styled(
                format!("  FDs {}", proc.open_files.len()),
                theme::status_style(),
            ),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines), area);
}

fn render_tab_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let tabs = [
        ("Open Files", DetailTab::OpenFiles),
        ("Network", DetailTab::Network),
        ("File Tree", DetailTab::FileTree),
        ("Summary", DetailTab::Summary),
    ];

    let spans: Vec<Span> = tabs
        .iter()
        .enumerate()
        .flat_map(|(i, (label, tab))| {
            let is_active = std::mem::discriminant(&state.detail_tab)
                == std::mem::discriminant(tab);
            let style = if is_active {
                theme::header_style().add_modifier(Modifier::UNDERLINED)
            } else {
                theme::status_style()
            };
            let mut v = vec![Span::styled(format!(" {} ", label), style)];
            if i < tabs.len() - 1 {
                v.push(Span::styled(" | ", theme::status_style()));
            }
            v
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_content(frame: &mut Frame, state: &mut AppState, area: Rect) {
    match state.detail_tab {
        DetailTab::OpenFiles => widgets::file_table::render(frame, state, area),
        DetailTab::Network => widgets::net_table::render(frame, state, area),
        DetailTab::FileTree => widgets::file_tree::render(frame, state, area),
        DetailTab::Summary => widgets::summary::render(frame, state, area),
    }
}

fn render_status(frame: &mut Frame, area: Rect) {
    let status = Paragraph::new(Line::from(Span::styled(
        "  Tab/Shift+Tab: switch tabs | j/k: scroll | Esc: back | q: quit",
        theme::status_style(),
    )));
    frame.render_widget(status, area);
}
