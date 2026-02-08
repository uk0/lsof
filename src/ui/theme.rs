use ratatui::style::{Color, Modifier, Style};

use crate::model::FileType;

pub fn header_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_style() -> Style {
    Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD)
}

pub fn normal_style() -> Style {
    Style::default().fg(Color::White)
}

pub fn search_style() -> Style {
    Style::default().fg(Color::Green)
}

pub fn status_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Color style for each file type to visually distinguish entries.
pub fn file_type_style(ft: &FileType) -> Style {
    match ft {
        FileType::Reg => Style::default().fg(Color::White),
        FileType::Dir => Style::default()
            .fg(Color::Blue)
            .add_modifier(Modifier::BOLD),
        FileType::Link => Style::default().fg(Color::Magenta),
        FileType::Chr => Style::default().fg(Color::Yellow),
        FileType::Blk => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        FileType::Fifo => Style::default().fg(Color::LightRed),
        FileType::Pipe => Style::default().fg(Color::LightRed),
        FileType::Sock => Style::default().fg(Color::LightGreen),
        FileType::Unix => Style::default().fg(Color::LightGreen),
        FileType::IPv4 => Style::default().fg(Color::Green),
        FileType::IPv6 => Style::default().fg(Color::Cyan),
        FileType::Kqueue => Style::default().fg(Color::DarkGray),
        FileType::Systm => Style::default().fg(Color::DarkGray),
        FileType::Unknown(_) => Style::default().fg(Color::Gray),
    }
}
