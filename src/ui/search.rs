use ratatui::{
    layout::{Constraint, Flex, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use ratatui_image::{Resize, StatefulImage};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &mut App) {
    if app.search_results.is_empty() {
        render_empty(frame);
    } else {
        render_results(frame, app);
    }
}

fn render_empty(frame: &mut Frame) {
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

fn render_results(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::vertical([
        Constraint::Min(1),    // main content
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

    let anime = &app.search_results[app.selected_result];
    let anime_id = anime.id.clone();
    let has_poster = app.poster_cache.contains_key(&anime_id);

    // Three columns: results list (25%) | poster (50%) | details (25%)
    let main_cols = Layout::horizontal([
        Constraint::Percentage(25), // results list
        Constraint::Percentage(50), // poster
        Constraint::Percentage(25), // details
    ])
    .split(chunks[0]);

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
            // Wrap long titles into multiple lines
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

    // Poster (center column) — bordered box with centered image
    let poster_block = Block::default().borders(Borders::ALL).title(" Poster ");
    let poster_inner = poster_block.inner(main_cols[1]);
    frame.render_widget(poster_block, main_cols[1]);

    if has_poster {
        if let Some((img_w, img_h, protocol)) = app.poster_cache.get_mut(&anime_id) {
            // Compute image size in columns/rows to center it
            let (cell_w, cell_h) = app
                .picker
                .as_ref()
                .map(|p| p.font_size())
                .unwrap_or((8, 16));
            let img_cols = (poster_inner.height as f32 * *img_w as f32 * cell_h as f32
                / (*img_h as f32 * cell_w as f32))
                .round() as u16;
            let img_cols = img_cols.min(poster_inner.width);

            // Center horizontally
            let centered = Layout::horizontal([Constraint::Length(img_cols)])
                .flex(Flex::Center)
                .split(poster_inner);

            let image_widget = StatefulImage::new().resize(Resize::Scale(None));
            frame.render_stateful_widget(image_widget, centered[0], protocol);
        }
    } else {
        let placeholder = Paragraph::new("No poster")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(placeholder, poster_inner);
    }

    // Re-borrow anime after mutable poster_cache access
    let anime = &app.search_results[app.selected_result];

    // Right column: top 20% metadata, bottom 80% synopsis
    let right_rows = Layout::vertical([
        Constraint::Percentage(20), // metadata with title
        Constraint::Percentage(80), // synopsis
    ])
    .split(main_cols[2]);

    // Top: metadata section with show title as block title
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

    let title_text = format!(" {} ", anime.title);
    let metadata = Paragraph::new(meta_lines)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    title_text,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
        );
    frame.render_widget(metadata, right_rows[0]);

    // Bottom: synopsis
    let synopsis_text = anime
        .synopsis
        .as_deref()
        .unwrap_or("No description available.");
    let synopsis = Paragraph::new(synopsis_text)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(" Synopsis "));
    frame.render_widget(synopsis, right_rows[1]);

    // Status bar
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
