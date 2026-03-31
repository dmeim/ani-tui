pub mod detail;
pub mod search;
pub mod setup;

use ratatui::Frame;

use crate::app::{App, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Search => search::render(frame, app),
        Screen::Detail => detail::render(frame, app),
        Screen::Playing => detail::render_playing(frame, app),
        Screen::Setup => setup::render(frame, app),
    }
}
