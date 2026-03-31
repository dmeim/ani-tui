use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let Some(ref anime) = app.selected_anime else {
        return;
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),  // title bar
        Constraint::Min(8),    // info + synopsis
        Constraint::Length(12), // episode list
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    // Title bar
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", anime.title),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Info + Synopsis area
    let info_chunks = Layout::horizontal([
        Constraint::Min(30),       // synopsis
        Constraint::Length(30),    // metadata sidebar
    ])
    .split(chunks[1]);

    // Synopsis
    let synopsis_text = anime
        .synopsis
        .as_deref()
        .unwrap_or("No description available.");
    let synopsis = Paragraph::new(synopsis_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Synopsis "));
    frame.render_widget(synopsis, info_chunks[0]);

    // Metadata sidebar
    let mut meta_lines: Vec<Line> = Vec::new();

    if let Some(eps) = anime.episode_count {
        meta_lines.push(Line::from(vec![
            Span::styled("Episodes: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{eps}")),
        ]));
    }

    if let Some(rating) = anime.rating {
        meta_lines.push(Line::from(vec![
            Span::styled("Rating:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{rating:.1}/10"),
                Style::default().fg(if rating >= 7.0 {
                    Color::Green
                } else if rating >= 5.0 {
                    Color::Yellow
                } else {
                    Color::Red
                }),
            ),
        ]));
    }

    if !anime.genres.is_empty() {
        meta_lines.push(Line::from(""));
        meta_lines.push(Line::from(Span::styled(
            "Genres:",
            Style::default().fg(Color::DarkGray),
        )));
        for genre in &anime.genres {
            meta_lines.push(Line::from(format!("  {genre}")));
        }
    }

    let metadata = Paragraph::new(meta_lines)
        .block(Block::default().borders(Borders::ALL).title(" Info "));
    frame.render_widget(metadata, info_chunks[1]);

    // Episode list
    if let Some(ref err) = app.error_message {
        let error = Paragraph::new(format!("Error: {err}"))
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Episodes "),
            );
        frame.render_widget(error, chunks[2]);
    } else if app.episodes.is_empty() {
        let loading = Paragraph::new("Loading episodes...")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Episodes "),
            );
        frame.render_widget(loading, chunks[2]);
    } else {
        let items: Vec<ListItem> = app
            .episodes
            .iter()
            .enumerate()
            .map(|(i, ep)| {
                let num = if ep.number == ep.number.floor() {
                    format!("{}", ep.number as i32)
                } else {
                    format!("{}", ep.number)
                };
                let label = match &ep.title {
                    Some(t) => format!("Episode {num}: {t}"),
                    None => format!("Episode {num}"),
                };
                let style = if i == app.selected_episode {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Span::styled(label, style))
            })
            .collect();

        let episode_list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Episodes ({}) ", app.episodes.len())),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(Some(app.selected_episode));
        frame.render_stateful_widget(episode_list, chunks[2], &mut state);
    }

    // Status bar
    let status = Line::from(vec![
        Span::styled(" Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back  "),
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" play episode  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[3]);
}

pub fn render_playing(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    // Title
    let title_text = match &app.now_playing_title {
        Some(t) => format!(" {t} "),
        None => " Now Playing ".to_string(),
    };
    let title_color = if app.play_error.is_some() {
        Color::Red
    } else if app.streams.is_empty() {
        Color::Yellow
    } else {
        Color::Green
    };
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(title_color)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Content
    let mut lines = vec![Line::from("")];

    if let Some(ref err) = app.play_error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {err}"),
            Style::default().fg(Color::Red),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from("  Press Esc to go back and try again."));
    } else if app.streams.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Resolving stream and launching player...",
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
            Span::raw(" — back to detail"),
        ]));
    }

    let content = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
    frame.render_widget(content, chunks[1]);

    // Status bar
    let status = Line::from(vec![
        Span::styled(" n", Style::default().fg(Color::Yellow)),
        Span::raw(" next  "),
        Span::styled("p", Style::default().fg(Color::Yellow)),
        Span::raw(" prev  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" back  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit"),
    ]);
    frame.render_widget(Paragraph::new(status), chunks[2]);
}
