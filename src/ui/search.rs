use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::{App, InputMode};

pub fn render(frame: &mut Frame, app: &mut App) {
    let outer = Layout::horizontal([
        Constraint::Percentage(70), // left: search + results
        Constraint::Percentage(30), // right: preview panel
    ])
    .split(frame.area());

    render_left_panel(frame, app, outer[0]);
    render_preview_panel(frame, app, outer[1]);
}

fn render_left_panel(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(3), // search input
        Constraint::Min(1),   // results
        Constraint::Length(1), // status bar
    ])
    .split(area);

    // Title
    let title = Paragraph::new(" ani-tui")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    // Search input
    let input_style = match app.input_mode {
        InputMode::Normal => Style::default(),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };
    let input_title = if app.search_loading {
        " Search (loading...) "
    } else {
        " Search "
    };
    let input = Paragraph::new(app.search_input.as_str())
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title(input_title));
    frame.render_widget(input, chunks[1]);

    if app.input_mode == InputMode::Editing {
        frame.set_cursor_position((
            chunks[1].x + app.cursor_position as u16 + 1,
            chunks[1].y + 1,
        ));
    }

    // Results
    if let Some(ref err) = app.search_error {
        let error = Paragraph::new(format!("Error: {err}"))
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title(" Results "));
        frame.render_widget(error, chunks[2]);
    } else if app.search_results.is_empty() {
        let msg = if app.search_loading {
            "Searching..."
        } else if app.search_input.is_empty() {
            "Press s to search for anime"
        } else {
            "No results found"
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Results "));
        frame.render_widget(empty, chunks[2]);
    } else {
        let items: Vec<ListItem> = app
            .search_results
            .iter()
            .enumerate()
            .map(|(i, anime)| {
                let ep_info = anime
                    .episode_count
                    .map(|n| format!(" ({n} episodes)"))
                    .unwrap_or_default();
                let rating_info = anime
                    .rating
                    .map(|r| format!(" [{r:.1}]"))
                    .unwrap_or_default();
                let line = Line::from(vec![
                    Span::styled(
                        &anime.title,
                        if i == app.selected_result {
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::styled(ep_info, Style::default().fg(Color::DarkGray)),
                    Span::styled(rating_info, Style::default().fg(Color::Cyan)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Results ({}) ", app.search_results.len())),
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        let mut state = ListState::default();
        state.select(Some(app.selected_result));
        frame.render_stateful_widget(list, chunks[2], &mut state);
    }

    // Status bar
    let status = match app.input_mode {
        InputMode::Normal => {
            if app.search_results.is_empty() {
                Line::from(vec![
                    Span::styled(" s", Style::default().fg(Color::Yellow)),
                    Span::raw(" search  "),
                    Span::styled("/", Style::default().fg(Color::Yellow)),
                    Span::raw(" settings  "),
                    Span::styled("q", Style::default().fg(Color::Yellow)),
                    Span::raw(" quit"),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" s", Style::default().fg(Color::Yellow)),
                    Span::raw(" search  "),
                    Span::styled("j/k", Style::default().fg(Color::Yellow)),
                    Span::raw(" navigate  "),
                    Span::styled("Enter", Style::default().fg(Color::Yellow)),
                    Span::raw(" select  "),
                    Span::styled("/", Style::default().fg(Color::Yellow)),
                    Span::raw(" settings  "),
                    Span::styled("q", Style::default().fg(Color::Yellow)),
                    Span::raw(" quit"),
                ])
            }
        }
        InputMode::Editing => Line::from(vec![
            Span::styled(" Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" search"),
        ]),
    };
    frame.render_widget(Paragraph::new(status), chunks[3]);
}

fn render_preview_panel(frame: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let selected = app.search_results.get(app.selected_result);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ");

    let Some(anime) = selected else {
        let empty = Paragraph::new("No selection")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let anime_id = anime.id.clone();
    let has_poster = app.poster_cache.contains_key(&anime_id);

    // Split preview into sections
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let poster_height = if has_poster {
        // Use roughly half the panel for the poster
        (inner.height / 2).max(6)
    } else {
        0
    };

    let sections = if has_poster {
        Layout::vertical([
            Constraint::Length(poster_height), // poster
            Constraint::Length(1),             // title
            Constraint::Length(1),             // blank
            Constraint::Min(3),               // synopsis
            Constraint::Length(1),             // blank separator
            Constraint::Length(6),             // metadata
        ])
        .split(inner)
    } else {
        Layout::vertical([
            Constraint::Length(0),  // no poster
            Constraint::Length(1),  // title
            Constraint::Length(1),  // blank
            Constraint::Min(3),    // synopsis
            Constraint::Length(1),  // blank separator
            Constraint::Length(6),  // metadata
        ])
        .split(inner)
    };

    // Poster
    if has_poster
        && let Some((_, _, protocol)) = app.poster_cache.get_mut(&anime_id)
    {
        let image_widget = StatefulImage::new().resize(Resize::Scale(None));
        frame.render_stateful_widget(image_widget, sections[0], protocol);
    }

    // Re-borrow anime after mutable poster_cache access
    let anime = &app.search_results[app.selected_result];

    // Title
    let title = Paragraph::new(Line::from(Span::styled(
        &anime.title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(title, sections[1]);

    // Synopsis
    let synopsis_text = anime
        .synopsis
        .as_deref()
        .unwrap_or("No description available.");
    let synopsis = Paragraph::new(synopsis_text)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White));
    frame.render_widget(synopsis, sections[3]);

    // Metadata
    let mut meta_lines: Vec<Line> = Vec::new();

    if let Some(eps) = anime.episode_count {
        meta_lines.push(Line::from(vec![
            Span::styled("Episodes: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{eps}")),
        ]));
    }

    if let Some(rating) = anime.rating {
        let color = if rating >= 7.0 {
            Color::Green
        } else if rating >= 5.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        meta_lines.push(Line::from(vec![
            Span::styled("Rating:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{rating:.1}/10"), Style::default().fg(color)),
        ]));
    }

    if !anime.genres.is_empty() {
        meta_lines.push(Line::from(vec![
            Span::styled("Genres:   ", Style::default().fg(Color::DarkGray)),
            Span::raw(anime.genres.join(", ")),
        ]));
    }

    let metadata = Paragraph::new(meta_lines).wrap(Wrap { trim: true });
    frame.render_widget(metadata, sections[5]);
}
