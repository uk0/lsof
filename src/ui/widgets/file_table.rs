use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::{Cell, Row, Table};

use crate::app::AppState;
use crate::ui::theme;

pub fn render(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let proc = match &state.selected_process {
        Some(p) => p,
        None => return,
    };

    let header = Row::new(vec![
        Cell::from("FD"),
        Cell::from("TYPE"),
        Cell::from("DEVICE"),
        Cell::from("SIZE/OFF"),
        Cell::from("NODE"),
        Cell::from("NAME"),
    ])
    .style(theme::header_style());

    let rows: Vec<Row> = proc
        .open_files
        .iter()
        .map(|f| {
            let style = theme::file_type_style(&f.file_type);
            let size_str = f
                .size_off
                .map(|s| format_size(s))
                .unwrap_or_default();
            let name_display = match &f.link_target {
                Some(target) => format!("{} -> {}", f.name, target),
                None => f.name.clone(),
            };
            Row::new(vec![
                Cell::from(Span::styled(f.fd.to_string(), style)),
                Cell::from(Span::styled(f.file_type.to_string(), style)),
                Cell::from(f.device.clone()),
                Cell::from(size_str),
                Cell::from(f.node.clone()),
                Cell::from(Span::styled(name_display, style)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Length(12),
        Constraint::Length(10),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            theme::selected_style().add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut state.file_table_state);
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
