use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::AppState;
use super::theme;

pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();

    // Layout: main list area at top, search input + status at bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),     // Process list (takes remaining space)
            Constraint::Length(1),  // Search input line
            Constraint::Length(1),  // Status line
        ])
        .split(area);

    render_process_list(frame, state, chunks[0]);
    render_search_input(frame, state, chunks[1]);
    render_status_line(frame, state, chunks[2]);
}

fn render_process_list(frame: &mut Frame, state: &mut AppState, area: Rect) {
    // Build header line
    let header_text = format!(
        "  {:<8} {:<20} {:<12} {:>6}",
        "PID", "COMMAND", "USER", "FDs"
    );
    let header_line = Line::from(Span::styled(header_text, theme::header_style()));

    // Build list items from filtered indices
    let items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .map(|&idx| {
            let proc = &state.all_processes[idx];
            let fd_count = proc.fd_count();
            let line_text = format!(
                "  {:<8} {:<20} {:<12} {:>6}",
                proc.pid,
                truncate_str(&proc.comm, 20),
                truncate_str(&proc.user, 12),
                fd_count,
            );
            ListItem::new(Line::from(Span::styled(line_text, theme::normal_style())))
        })
        .collect();

    // Split the list area: header at top, scrollable list below
    let list_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(1),   // List
        ])
        .split(area);

    // Render header
    let header_paragraph = Paragraph::new(header_line);
    frame.render_widget(header_paragraph, list_chunks[0]);

    // Render the scrollable list
    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, list_chunks[1], &mut state.list_state);
}

fn render_search_input(frame: &mut Frame, state: &AppState, area: Rect) {
    let input_text = Line::from(vec![
        Span::styled("> ", theme::search_style().add_modifier(Modifier::BOLD)),
        Span::styled(&state.search_input, theme::search_style()),
        Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
    ]);

    let input = Paragraph::new(input_text);
    frame.render_widget(input, area);
}

fn render_status_line(frame: &mut Frame, state: &AppState, area: Rect) {
    let status_text = format!("  {}/{}", state.match_count, state.total_count);
    let status = Paragraph::new(Line::from(Span::styled(
        status_text,
        theme::status_style(),
    )));
    frame.render_widget(status, area);
}

/// Truncate a string to fit within a given width, appending ".." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len > 2 {
        format!("{}..", &s[..max_len - 2])
    } else {
        s[..max_len].to_string()
    }
}
