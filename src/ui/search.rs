use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, InputMode};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(3), // title
        Constraint::Length(3), // search input
        Constraint::Min(1),   // results
        Constraint::Length(1), // status bar
    ])
    .split(frame.area());

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
            "Type / to search for anime"
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
                    Span::styled(" /", Style::default().fg(Color::Yellow)),
                    Span::raw(" search  "),
                    Span::styled("q", Style::default().fg(Color::Yellow)),
                    Span::raw(" quit"),
                ])
            } else {
                Line::from(vec![
                    Span::styled(" /", Style::default().fg(Color::Yellow)),
                    Span::raw(" search  "),
                    Span::styled("j/k", Style::default().fg(Color::Yellow)),
                    Span::raw(" navigate  "),
                    Span::styled("Enter", Style::default().fg(Color::Yellow)),
                    Span::raw(" select  "),
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
