use std::collections::BTreeMap;

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::AppState;
use crate::model::OpenFileInfo;
use crate::ui::theme;

/// Info carried per file entry in the tree.
struct TreeEntry {
    filename: String,
    file_type: String,
    size: Option<u64>,
    style: ratatui::style::Style,
}

pub fn render(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let proc = match &state.selected_process {
        Some(p) => p,
        None => return,
    };

    if proc.open_files.is_empty() {
        let msg = Paragraph::new(Span::styled("  No open files", theme::status_style()));
        frame.render_widget(msg, area);
        return;
    }

    // Build a tree structure from file paths, carrying type + size info
    let mut tree: BTreeMap<String, Vec<TreeEntry>> = BTreeMap::new();

    for f in &proc.open_files {
        let entry = make_tree_entry(f);
        let path = &f.name;
        if let Some(pos) = path.rfind('/') {
            let dir = &path[..pos];
            let dir_key = if dir.is_empty() {
                "/".to_string()
            } else {
                dir.to_string()
            };
            tree.entry(dir_key).or_default().push(entry);
        } else {
            tree.entry("(other)".to_string()).or_default().push(entry);
        }
    }

    let mut items: Vec<ListItem> = Vec::new();

    for (dir, files) in &tree {
        // Directory line
        items.push(ListItem::new(Line::from(Span::styled(
            format!("  {}/", dir),
            theme::header_style(),
        ))));

        // File lines with type tag and size
        for entry in files {
            let size_str = entry.size.map(format_size).unwrap_or_default();
            let label = if size_str.is_empty() {
                format!("    {} [{}]", entry.filename, entry.file_type)
            } else {
                format!("    {} [{}] {}", entry.filename, entry.file_type, size_str)
            };
            items.push(ListItem::new(Line::from(Span::styled(label, entry.style))));
        }
    }

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, &mut state.tree_list_state);
}

fn make_tree_entry(f: &OpenFileInfo) -> TreeEntry {
    let filename = if let Some(pos) = f.name.rfind('/') {
        f.name[pos + 1..].to_string()
    } else {
        f.name.clone()
    };
    TreeEntry {
        filename,
        file_type: f.file_type.to_string(),
        size: f.size_off,
        style: theme::file_type_style(&f.file_type),
    }
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
