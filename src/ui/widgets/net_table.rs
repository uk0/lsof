use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::{Cell, Paragraph, Row, Table};

use crate::app::AppState;
use crate::model::FileType;
use crate::ui::theme;

pub fn render(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let proc = match &state.selected_process {
        Some(p) => p,
        None => return,
    };

    let net_files: Vec<_> = proc
        .open_files
        .iter()
        .filter(|f| matches!(
            f.file_type,
            FileType::IPv4 | FileType::IPv6 | FileType::Sock | FileType::Unix
        ))
        .collect();

    if net_files.is_empty() {
        let msg = Paragraph::new(Span::styled(
            "  No network connections",
            theme::status_style(),
        ));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("FD"),
        Cell::from("PROTO"),
        Cell::from("NAME"),
    ])
    .style(theme::header_style());

    let rows: Vec<Row> = net_files
        .iter()
        .map(|f| {
            let style = theme::file_type_style(&f.file_type);
            Row::new(vec![
                Cell::from(Span::styled(f.fd.to_string(), style)),
                Cell::from(Span::styled(f.file_type.to_string(), style)),
                Cell::from(Span::styled(f.name.clone(), style)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(7),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .row_highlight_style(
            theme::selected_style().add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut state.file_table_state);
}
