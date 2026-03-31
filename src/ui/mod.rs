pub mod detail;
pub mod search;
pub mod setup;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, ModalKind, Screen};
use crate::config::PlayerName;

pub fn render(frame: &mut Frame, app: &mut App) {
    // Render the base screen
    match app.screen {
        Screen::Search => search::render(frame, app),
        Screen::Detail => detail::render(frame, app),
        Screen::Setup => setup::render(frame, app),
    }

    // Render modal overlay if active
    match app.active_modal {
        Some(ModalKind::Settings) => render_settings_modal(frame, app),
        Some(ModalKind::Player) => render_player_modal(frame, app),
        Some(ModalKind::Search) => render_search_modal(frame, app),
        None => {}
    }
}

fn centered_rect(width_pct: u16, height_pct: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(height_pct)])
        .flex(Flex::Center)
        .split(area);
    Layout::horizontal([Constraint::Percentage(width_pct)])
        .flex(Flex::Center)
        .split(vertical[0])[0]
}

fn render_player_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 40, frame.area());
    frame.render_widget(Clear, area);

    let title_text = match &app.now_playing_title {
        Some(t) => format!(" {} ", t),
        None => " Now Playing ".to_string(),
    };

    let title_color = if app.play_error.is_some() {
        Color::Red
    } else if app.streams.is_empty() {
        Color::Yellow
    } else {
        Color::Green
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    if let Some(ref err) = app.play_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("  Press Esc to dismiss."));
    } else if app.streams.is_empty() {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame_char = SPINNER[app.tick_count % SPINNER.len()];
        lines.push(Line::from(Span::styled(
            format!("  {frame_char} Resolving stream and launching player..."),
            Style::default().fg(Color::Yellow),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Player launched!",
            Style::default().fg(Color::Green),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("  Controls:"));
        lines.push(Line::from(vec![
            Span::styled("    n", Style::default().fg(Color::Yellow)),
            Span::raw(" — next episode"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    p", Style::default().fg(Color::Yellow)),
            Span::raw(" — previous episode"),
        ]));
        lines.push(Line::from(vec![
            Span::styled("    Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" — dismiss"),
        ]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title_text,
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        ))
        .border_style(Style::default().fg(title_color));
    let content = Paragraph::new(lines).wrap(Wrap { trim: false }).block(block);
    frame.render_widget(content, area);
}

fn render_search_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Search ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(3), // search input
        Constraint::Min(1),   // loading / error message
        Constraint::Length(1), // status bar
    ])
    .split(inner);

    // Search input
    let input_title = if app.search_loading {
        " Searching... "
    } else {
        " Query "
    };
    let input = Paragraph::new(app.search_input.as_str())
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, chunks[0]);

    // Cursor
    if app.input_mode == crate::app::InputMode::Editing {
        frame.set_cursor_position((
            chunks[0].x + app.cursor_position as u16 + 1,
            chunks[0].y + 1,
        ));
    }

    // Loading spinner or error
    if app.search_loading {
        const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        let frame_char = SPINNER[app.tick_count % SPINNER.len()];
        let loading = Paragraph::new(format!("  {frame_char} Searching..."))
            .style(Style::default().fg(Color::Yellow));
        frame.render_widget(loading, chunks[1]);
    } else if let Some(ref err) = app.search_error {
        let error = Paragraph::new(format!("  Error: {err}"))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error, chunks[1]);
    }

    // Status bar
    let status = Line::from(vec![
        Span::styled(" Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" search  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(if app.search_results.is_empty() {
            " quit"
        } else {
            " cancel"
        }),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[2]);
}

fn render_settings_modal(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 50, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Settings ")
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 3 setting rows + status bar
    let chunks = Layout::vertical([
        Constraint::Length(2), // Metadata Provider
        Constraint::Length(2), // Video Player
        Constraint::Length(2), // Audio Preference
        Constraint::Min(0),   // spacer
        Constraint::Length(1), // status bar
    ])
    .split(inner);

    let settings: [(&str, String); 3] = [
        (
            "Metadata Provider",
            current_value_label(app, 0),
        ),
        (
            "Video Player",
            current_value_label(app, 1),
        ),
        (
            "Audio Preference",
            current_value_label(app, 2),
        ),
    ];

    for (i, (label, value)) in settings.iter().enumerate() {
        let is_focused = app.settings_cursor == i;
        let is_open = is_focused && app.settings_editing;

        let marker = if is_focused { "> " } else { "  " };
        let label_style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let value_style = if is_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let row = Layout::horizontal([
            Constraint::Length(2),  // marker
            Constraint::Length(22), // label
            Constraint::Min(10),   // value
        ])
        .split(chunks[i]);

        frame.render_widget(
            Paragraph::new(marker).style(label_style),
            row[0],
        );
        frame.render_widget(
            Paragraph::new(label.to_string()).style(label_style),
            row[1],
        );

        if is_open {
            // Show dropdown indicator
            frame.render_widget(
                Paragraph::new(format!("[{}] ▼", value))
                    .style(Style::default().fg(Color::Yellow)),
                row[2],
            );
        } else {
            frame.render_widget(
                Paragraph::new(format!("[{}]", value)).style(value_style),
                row[2],
            );
        }
    }

    // If a dropdown is open, render it as a floating overlay below the row
    if app.settings_editing {
        let options = setting_options(app.settings_cursor);
        let dropdown_row = app.settings_cursor;
        // Position dropdown just below the setting row
        let anchor = chunks[dropdown_row];
        let dropdown_height = (options.len() as u16 + 2).min(inner.height.saturating_sub(anchor.y - inner.y + anchor.height));
        let dropdown_area = Rect {
            x: anchor.x + 24, // align with value column
            y: anchor.y + anchor.height,
            width: anchor.width.saturating_sub(24).max(20),
            height: dropdown_height,
        };
        // Clamp to terminal bounds
        let dropdown_area = clamp_rect(dropdown_area, frame.area());

        frame.render_widget(Clear, dropdown_area);

        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let is_selected = i == app.settings_option_cursor;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let marker = if is_selected { "> " } else { "  " };
                ListItem::new(Span::styled(format!("{marker}{label}"), style))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL));
        let mut state = ListState::default();
        state.select(Some(app.settings_option_cursor));
        frame.render_stateful_widget(list, dropdown_area, &mut state);
    }

    // Status bar
    let status = if app.settings_editing {
        Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
            Span::styled("Enter/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" confirm"),
        ])
    } else {
        Line::from(vec![
            Span::styled(" j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" change  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" close"),
        ])
    };
    frame.render_widget(Paragraph::new(status), chunks[4]);
}

/// Get the display label for the current value of a setting row.
fn current_value_label(app: &App, row: usize) -> String {
    match row {
        0 => match app.config.general.metadata_provider {
            crate::config::MetadataProvider::Jikan => "Jikan (MAL)".to_string(),
            crate::config::MetadataProvider::Anilist => "AniList".to_string(),
            crate::config::MetadataProvider::Anidb => "AniDB".to_string(),
        },
        1 => match app.config.player.name {
            PlayerName::Mpv => "mpv".to_string(),
            PlayerName::Iina => "IINA".to_string(),
            PlayerName::Vlc => "VLC".to_string(),
            PlayerName::Quicktime => "QuickTime".to_string(),
            PlayerName::Custom => "Custom".to_string(),
        },
        2 => match app.config.general.default_mode {
            crate::config::AudioMode::Sub => "Sub".to_string(),
            crate::config::AudioMode::Dub => "Dub".to_string(),
        },
        _ => String::new(),
    }
}

/// Get the list of option labels for a given setting row.
fn setting_options(row: usize) -> Vec<String> {
    match row {
        0 => vec!["Jikan (MAL)".to_string(), "AniList".to_string(), "AniDB".to_string()],
        1 => {
            let detected = crate::player::detect_installed();
            let mut opts: Vec<String> = detected
                .iter()
                .map(|p| match p {
                    PlayerName::Mpv => "mpv".to_string(),
                    PlayerName::Iina => "IINA".to_string(),
                    PlayerName::Vlc => "VLC".to_string(),
                    PlayerName::Quicktime => "QuickTime".to_string(),
                    PlayerName::Custom => "Custom".to_string(),
                })
                .collect();
            opts.push("Custom".to_string());
            opts
        }
        2 => vec!["Sub".to_string(), "Dub".to_string()],
        _ => vec![],
    }
}

/// Clamp a rect so it doesn't overflow the terminal area.
fn clamp_rect(r: Rect, bounds: Rect) -> Rect {
    let x = r.x.min(bounds.x + bounds.width.saturating_sub(r.width));
    let y = r.y.min(bounds.y + bounds.height.saturating_sub(r.height));
    let w = r.width.min(bounds.width);
    let h = r.height.min(bounds.height);
    Rect::new(x, y, w, h)
}
