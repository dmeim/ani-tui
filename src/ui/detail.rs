use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    let Some(ref anime) = app.selected_anime else {
        return;
    };

    let chunks = Layout::vertical([
        Constraint::Length(3),  // title bar
        Constraint::Min(8),    // info + synopsis
        Constraint::Min(12),    // episode list + detail
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
    let anime_id = anime.id.clone();
    let has_poster = app.poster_cache.contains_key(&anime_id);

    // Build metadata lines first so we can measure the sidebar width
    // (re-borrow anime immutably — poster_cache access comes later)
    let mut meta_lines: Vec<Line> = Vec::new();
    let mut max_meta_width: usize = 0;

    if let Some(eps) = anime.episode_count {
        let line_len = format!("Episodes: {eps}").len();
        max_meta_width = max_meta_width.max(line_len);
        meta_lines.push(Line::from(vec![
            Span::styled("Episodes: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{eps}")),
        ]));
    }

    if let Some(rating) = anime.rating {
        let line_len = format!("Rating:   {rating:.1}/10").len();
        max_meta_width = max_meta_width.max(line_len);
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
            max_meta_width = max_meta_width.max(genre.len() + 2); // "  genre"
            meta_lines.push(Line::from(format!("  {genre}")));
        }
    }

    // Info sidebar: content width + borders (2) + inner padding (2)
    let info_width = (max_meta_width as u16 + 4).max(12);

    // Poster column: compute width from actual image dimensions + terminal font size.
    // cols = rows * (img_w / img_h) * (cell_h / cell_w)
    let poster_width = if has_poster {
        let available_h = chunks[1].height;
        let (img_w, img_h, _) = app.poster_cache.get(&anime_id).unwrap();
        let (cell_w, cell_h) = app
            .picker
            .as_ref()
            .map(|p| p.font_size())
            .unwrap_or((8, 16));
        let inner_rows = available_h;
        let cols = (inner_rows as f32 * *img_w as f32 * cell_h as f32
            / (*img_h as f32 * cell_w as f32))
            .round() as u16;
        cols.max(10)
    } else {
        0
    };

    let info_chunks = Layout::horizontal([
        Constraint::Length(poster_width), // poster
        Constraint::Min(20),             // synopsis (gets all remaining)
        Constraint::Length(info_width),   // metadata sidebar
    ])
    .split(chunks[1]);

    // Poster — rendered without a border box so image fills the space exactly
    if has_poster
        && let Some((_, _, protocol)) = app.poster_cache.get_mut(&anime_id)
    {
        let image_widget = StatefulImage::new().resize(Resize::Scale(None));
        frame.render_stateful_widget(image_widget, info_chunks[0], protocol);
    }

    // Re-borrow anime after mutable poster_cache access
    let anime = app.selected_anime.as_ref().unwrap();

    // Synopsis
    let synopsis_text = anime
        .synopsis
        .as_deref()
        .unwrap_or("No description available.");
    let synopsis = Paragraph::new(synopsis_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Synopsis "));
    frame.render_widget(synopsis, info_chunks[1]);

    // Metadata sidebar
    let metadata = Paragraph::new(meta_lines)
        .block(Block::default().borders(Borders::ALL).title(" Info "));
    frame.render_widget(metadata, info_chunks[2]);

    // Episode section: 40% list | 60% detail
    let episode_chunks = Layout::horizontal([
        Constraint::Percentage(40), // episode list
        Constraint::Percentage(60), // episode detail
    ])
    .split(chunks[2]);

    // Episode list (left column)
    if let Some(ref err) = app.error_message {
        let error = Paragraph::new(format!("Error: {err}"))
            .style(Style::default().fg(Color::Red))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Episodes "),
            );
        frame.render_widget(error, episode_chunks[0]);
    } else if app.episodes.is_empty() {
        let loading = Paragraph::new("Loading episodes...")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Episodes "),
            );
        frame.render_widget(loading, episode_chunks[0]);
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
        frame.render_stateful_widget(episode_list, episode_chunks[0], &mut state);
    }

    // Episode detail (right column)
    // Inner width = area width minus 2 for block borders
    let detail_inner_width = episode_chunks[1].width.saturating_sub(2) as usize;

    let detail_content = if let Some(ep) = app.episodes.get(app.selected_episode) {
        let num = if ep.number == ep.number.floor() {
            format!("{}", ep.number as i32)
        } else {
            format!("{}", ep.number)
        };

        let mut lines: Vec<Line> = Vec::new();

        // Episode heading with aired date right-aligned on the same line
        let heading = match &ep.title {
            Some(t) => format!("Episode {num}: {t}"),
            None => format!("Episode {num}"),
        };

        if let Some(ref aired) = ep.aired {
            let aired_text = format!("Aired: {aired}");
            let gap = detail_inner_width
                .saturating_sub(heading.len())
                .saturating_sub(aired_text.len());
            if gap >= 2 {
                // Fits on one line: heading + padding + aired
                lines.push(Line::from(vec![
                    Span::styled(
                        heading,
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" ".repeat(gap)),
                    Span::styled(aired_text, Style::default().fg(Color::DarkGray)),
                ]));
            } else {
                // Too narrow — fall back to separate lines
                lines.push(Line::from(Span::styled(
                    heading,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(vec![
                    Span::styled("Aired: ", Style::default().fg(Color::DarkGray)),
                    Span::raw(aired.as_str()),
                ]));
            }
        } else {
            lines.push(Line::from(Span::styled(
                heading,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        // Filler badge
        if ep.is_filler {
            lines.push(Line::from(Span::styled(
                "⚠ Filler episode",
                Style::default().fg(Color::Yellow),
            )));
        }

        lines.push(Line::from(""));

        // Episode synopsis (from Jikan), fall back to anime synopsis
        if let Some(ref synopsis) = ep.synopsis {
            lines.push(Line::from(synopsis.as_str()));
        } else if let Some(ref synopsis) = anime.synopsis {
            lines.push(Line::from(Span::styled(
                "Series Synopsis",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(synopsis.as_str()));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "Select an episode",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let detail_panel = Paragraph::new(detail_content)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Episode Detail "),
        );
    frame.render_widget(detail_panel, episode_chunks[1]);

    // Status bar (hidden when a modal is active — modal shows its own keybinds)
    if app.active_modal.is_none() {
        let status = Line::from(vec![
            Span::styled(" Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" back  "),
            Span::styled("↑/↓/j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" play episode  "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" settings  "),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[3]);
    }
}
