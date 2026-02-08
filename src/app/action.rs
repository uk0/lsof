use crate::app::state::ViewMode;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub enum Action {
    Quit,
    SearchInput(char),
    SearchBackspace,
    SearchClear,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Select,
    Back,
    NextTab,
    PrevTab,
    Refresh,
    YankSelected,
    ExportProcess,
}

/// Map a key event to an action based on the current view mode and search state.
pub fn map_key_to_action(key: KeyEvent, mode: &ViewMode, search_empty: bool) -> Option<Action> {
    // Only handle key press events to avoid duplicate events
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match mode {
        ViewMode::Search => map_search_key(key, search_empty),
        ViewMode::Detail => map_detail_key(key),
    }
}

fn map_search_key(key: KeyEvent, search_empty: bool) -> Option<Action> {
    // Check for Ctrl modifiers first
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('u') => Some(Action::SearchClear),
            KeyCode::Char('c') => Some(Action::Quit),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Esc => {
            if search_empty {
                Some(Action::Quit)
            } else {
                Some(Action::SearchClear)
            }
        }
        KeyCode::Enter => Some(Action::Select),
        KeyCode::Backspace => Some(Action::SearchBackspace),
        KeyCode::Up => Some(Action::MoveUp),
        KeyCode::Down => Some(Action::MoveDown),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::F(5) => Some(Action::Refresh),
        KeyCode::Char('k') if search_empty => Some(Action::MoveUp),
        KeyCode::Char('j') if search_empty => Some(Action::MoveDown),
        KeyCode::Char('q') if search_empty => Some(Action::Quit),
        KeyCode::Char(c) => Some(Action::SearchInput(c)),
        _ => None,
    }
}

fn map_detail_key(key: KeyEvent) -> Option<Action> {
    // Check for Ctrl modifiers first
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => Some(Action::Quit),
            KeyCode::Char('y') => Some(Action::YankSelected),
            KeyCode::Char('e') => Some(Action::ExportProcess),
            _ => None,
        };
    }

    match key.code {
        KeyCode::Esc => Some(Action::Back),
        KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Tab => Some(Action::NextTab),
        KeyCode::BackTab => Some(Action::PrevTab),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::F(5) => Some(Action::Refresh),
        _ => None,
    }
}
