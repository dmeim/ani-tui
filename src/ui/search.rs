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
    if app.search_results.is_empty() {
        render_empty(frame, app);
    } else {
        render_results(frame, app);
    }
}

fn render_empty(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3),  // title
        Constraint::Min(1),    // empty space
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    let title = Paragraph::new(" ani-tui")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(title, chunks[0]);

    if app.active_modal.is_none() {
        let status = Line::from(vec![
            Span::styled(" s", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" settings  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[2]);
    }
}

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

fn render_results(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),    // main content
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    let anime = &app.search_results[app.selected_result];
    let anime_id = anime.id.clone();
    let has_poster = app.poster_cache.contains_key(&anime_id);
    let has_metadata = anime.episode_count.is_some()
        || anime.rating.is_some()
        || !anime.genres.is_empty();
    let has_synopsis = anime.synopsis.is_some();

    // Layout: use aspect-ratio-based poster width when loaded, else default 30/40/30
    let available_h = chunks[0].height;
    let main_cols = if has_poster {
        let poster_cols = if let Some((img_w, img_h, _)) = app.poster_cache.get(&anime_id) {
            let (cell_w, cell_h) = app
                .picker
                .as_ref()
                .map(|p| p.font_size())
                .unwrap_or((8, 16));
            let inner_rows = available_h.saturating_sub(2);
            let cols = (inner_rows as f32 * *img_w as f32 * cell_h as f32
                / (*img_h as f32 * cell_w as f32))
                .round() as u16;
            (cols + 2).min(chunks[0].width.saturating_sub(20))
        } else {
            chunks[0].width * 40 / 100
        };
        Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(poster_cols),
            Constraint::Fill(1),
        ])
        .split(chunks[0])
    } else {
        // Default 30/40/30 while poster is loading
        Layout::horizontal([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(chunks[0])
    };

    // Results list (left column)
    let list_inner_width = main_cols[0].width.saturating_sub(2) as usize;
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let style = if i == app.selected_result {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let lines: Vec<Line> = wrap_text(&a.title, list_inner_width)
                .into_iter()
                .map(|chunk| Line::from(Span::styled(chunk, style)))
                .collect();
            ListItem::new(lines)
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
    frame.render_stateful_widget(list, main_cols[0], &mut state);

    // Poster (center column)
    let poster_block = Block::default().borders(Borders::ALL).title(" Poster ");
    let poster_inner = poster_block.inner(main_cols[1]);
    frame.render_widget(poster_block, main_cols[1]);

    if has_poster {
        if let Some((_, _, protocol)) = app.poster_cache.get_mut(&anime_id) {
            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            frame.render_stateful_widget(image_widget, poster_inner, protocol);
        }
    } else {
        // Loading placeholder with spinner
        let spinner = SPINNER[app.tick_count % SPINNER.len()];
        let loading = Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{spinner} Loading Poster..."),
                Style::default().fg(Color::Yellow),
            ),
        ]));
        // Center vertically by placing in the middle of poster_inner
        let vert = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .split(poster_inner);
        let horiz = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(20),
            Constraint::Fill(1),
        ])
        .split(vert[1]);
        frame.render_widget(loading, horiz[1]);
    }

    // Re-borrow anime after mutable poster_cache access
    let anime = &app.search_results[app.selected_result];

    // Right column: top 20% metadata, bottom 80% synopsis
    let right_rows = Layout::vertical([
        Constraint::Percentage(20),
        Constraint::Percentage(80),
    ])
    .split(main_cols[2]);

    // Top: metadata section with show title as block title
    let title_text = format!(" {} ", anime.title);
    let meta_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(
            title_text,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    if has_metadata {
        let mut meta_lines: Vec<Line> = Vec::new();
        let mut info_spans: Vec<Span> = Vec::new();

        if let Some(eps) = anime.episode_count {
            info_spans.push(Span::styled(
                "Episodes: ",
                Style::default().fg(Color::DarkGray),
            ));
            info_spans.push(Span::raw(format!("{eps}")));
            info_spans.push(Span::raw("    "));
        }

        if let Some(rating) = anime.rating {
            let color = if rating >= 7.0 {
                Color::Green
            } else if rating >= 5.0 {
                Color::Yellow
            } else {
                Color::Red
            };
            info_spans.push(Span::styled(
                "Rating: ",
                Style::default().fg(Color::DarkGray),
            ));
            info_spans.push(Span::styled(
                format!("{rating:.1}/10"),
                Style::default().fg(color),
            ));
            info_spans.push(Span::raw("    "));
        }

        if !info_spans.is_empty() {
            meta_lines.push(Line::from(info_spans));
        }

        if !anime.genres.is_empty() {
            meta_lines.push(Line::from(vec![
                Span::styled("Genres: ", Style::default().fg(Color::DarkGray)),
                Span::raw(anime.genres.join(", ")),
            ]));
        }

        let metadata = Paragraph::new(meta_lines)
            .wrap(Wrap { trim: true })
            .block(meta_block);
        frame.render_widget(metadata, right_rows[0]);
    } else {
        let spinner = SPINNER[app.tick_count % SPINNER.len()];
        let loading = Paragraph::new(Line::from(Span::styled(
            format!("{spinner} Loading..."),
            Style::default().fg(Color::Yellow),
        )))
        .block(meta_block);
        frame.render_widget(loading, right_rows[0]);
    }

    // Bottom: synopsis
    let synopsis_block = Block::default().borders(Borders::ALL).title(" Synopsis ");
    if has_synopsis {
        let synopsis = Paragraph::new(anime.synopsis.as_deref().unwrap_or(""))
            .wrap(Wrap { trim: true })
            .block(synopsis_block);
        frame.render_widget(synopsis, right_rows[1]);
    } else {
        let spinner = SPINNER[app.tick_count % SPINNER.len()];
        let loading = Paragraph::new(Line::from(Span::styled(
            format!("{spinner} Loading..."),
            Style::default().fg(Color::Yellow),
        )))
        .block(synopsis_block);
        frame.render_widget(loading, right_rows[1]);
    }

    // Status bar (hidden when a modal is active — modal shows its own keybinds)
    if app.active_modal.is_none() {
        let result_indicator = format!(
            " [{}/{}] ",
            app.selected_result + 1,
            app.search_results.len()
        );
        let status = Line::from(vec![
            Span::styled(result_indicator, Style::default().fg(Color::Cyan)),
            Span::styled("s", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" settings  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ]);
        frame.render_widget(Paragraph::new(status), chunks[1]);
    }
}

/// Word-wrap `text` to fit within `width` columns, breaking at word boundaries.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            // Word longer than width gets its own line (will be clipped by terminal, but not lost)
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
