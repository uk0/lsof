pub mod detail_view;
pub mod search_view;
pub mod theme;
pub mod widgets;

use ratatui::Frame;
use crate::app::AppState;
use crate::app::state::ViewMode;

pub fn render(frame: &mut Frame, state: &mut AppState) {
    let area = frame.area();
    match state.mode {
        ViewMode::Search => search_view::render(frame, state),
        ViewMode::Detail => detail_view::render(frame, state, area),
    }
}
