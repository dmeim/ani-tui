use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::App;
use crate::config::PlayerName;

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // title
        Constraint::Length(3),  // step indicator
        Constraint::Min(1),    // options
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Title
    let title = Paragraph::new(" ani-tui Setup")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Step indicator
    let steps = [
        "Series Details Provider",
        "Episode Details Provider",
        "Poster Provider",
        "Video Player",
        "Audio Preference",
        "Minimum Quality",
    ];
    let step_display = format!(
        " Step {} of {}: {} ",
        app.setup_step + 1,
        steps.len(),
        steps.get(app.setup_step).unwrap_or(&"Done")
    );
    let step_indicator = Paragraph::new(step_display)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(step_indicator, chunks[1]);

    // Options for current step
    let (items, description) = match app.setup_step {
        0 => series_provider_options(),
        1 => episode_provider_options(),
        2 => poster_provider_options(),
        3 => player_options(),
        4 => audio_mode_options(),
        5 => min_quality_options(),
        _ => (vec![], "Setup complete!".to_string()),
    };

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let style = if i == app.setup_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if i == app.setup_selected { "> " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{marker}{name}"), style),
                Span::styled(format!("  {desc}"), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {description} ")),
    );

    let mut state = ListState::default();
    state.select(Some(app.setup_selected));
    frame.render_stateful_widget(list, chunks[2], &mut state);

    // Status bar
    let status = Line::from(vec![
        Span::styled(" ↑/↓/j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" select  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back"),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[3]);
}

fn series_provider_options() -> (Vec<(&'static str, &'static str)>, String) {
    let items = vec![
        ("Jikan (MAL)", "Ratings, genres, episode count, synopsis"),
        ("AniList", "Free, modern API, ratings + genres"),
        ("AniDB", "Comprehensive database, requires client registration"),
        ("Kitsu", "JSON:API, ratings + genres, no auth"),
        ("Notify.moe", "Open-source tracker, ratings + genres"),
    ];
    (items, "Series details provider (ratings, genres, etc.)".to_string())
}

fn episode_provider_options() -> (Vec<(&'static str, &'static str)>, String) {
    let items = vec![
        ("Jikan (MAL)", "Episode titles, synopses, filler flags"),
        ("AniList", "Limited episode data"),
        ("AniDB", "Comprehensive episode database, requires registration"),
        ("Kitsu", "Episode titles + synopses, no auth"),
        ("Notify.moe", "No episode details available"),
    ];
    (items, "Episode details provider (titles, synopses)".to_string())
}

fn poster_provider_options() -> (Vec<(&'static str, &'static str)>, String) {
    let items = vec![
        ("Jikan (MAL)", "MAL cover images"),
        ("AniList", "High-quality cover art"),
        ("AniDB", "AniDB cover images, requires registration"),
        ("Kitsu", "High-quality poster images"),
        ("Notify.moe", "Notify.moe cover images"),
    ];
    (items, "Poster provider (cover images)".to_string())
}

fn player_options() -> (Vec<(&'static str, &'static str)>, String) {
    let detected = crate::player::detect_installed();
    let mut items: Vec<(&str, &str)> = Vec::new();

    for player in &detected {
        match player {
            PlayerName::Mpv => items.push(("mpv", "Lightweight, powerful media player")),
            PlayerName::Iina => items.push(("IINA", "Modern macOS media player")),
            PlayerName::Vlc => items.push(("VLC", "Universal media player")),
            PlayerName::Quicktime => items.push(("QuickTime", "macOS built-in player")),
            PlayerName::Custom => {}
        }
    }

    items.push(("Custom", "Specify a custom player command"));

    (items, "Choose your video player".to_string())
}

fn audio_mode_options() -> (Vec<(&'static str, &'static str)>, String) {
    let items = vec![
        ("Sub", "Japanese audio with subtitles"),
        ("Dub", "English dubbed audio"),
    ];
    (items, "Choose your default audio preference".to_string())
}

fn min_quality_options() -> (Vec<(&'static str, &'static str)>, String) {
    let items = vec![
        ("Any", "Play whatever is available"),
        ("360p", "Low quality, saves bandwidth"),
        ("480p", "Standard definition"),
        ("720p", "HD quality"),
        ("1080p", "Full HD (recommended)"),
    ];
    (items, "Minimum stream quality to play".to_string())
}
