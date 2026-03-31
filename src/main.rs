#![allow(dead_code)]

mod action;
mod app;
mod config;
mod tui;

mod api;
mod model;
mod player;
mod ui;

use color_eyre::Result;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::allanime::AllAnimeClient;
use crate::api::anilist::AniListClient;
use crate::app::App;
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let config = Config::load()?;
    let mut terminal = tui::init()?;
    let mut events = tui::EventHandler::new(4.0, 30.0);
    let mut app = App::new(config);

    let allanime = AllAnimeClient::new()?;
    let anilist = AniListClient::new();

    // Channel for async tasks to send actions back
    let (async_tx, mut async_rx) = mpsc::unbounded_channel::<Action>();

    loop {
        // Check for async results (non-blocking)
        while let Ok(action) = async_rx.try_recv() {
            if let Some(follow_up) = app.handle_action(action) {
                dispatch_action(follow_up, &app, &allanime, &anilist, &async_tx);
            }
        }

        let action = events.next().await?;

        match action {
            Action::Render => {
                terminal.draw(|frame| ui::render(frame, &app))?;
                continue;
            }
            Action::Tick => continue,
            _ => {}
        }

        if let Some(follow_up) = app.handle_action(action) {
            dispatch_action(follow_up, &app, &allanime, &anilist, &async_tx);
        }

        if app.should_quit {
            break;
        }
    }

    events.stop();
    tui::restore()?;
    Ok(())
}

/// Spawn async work for actions that need API calls or player launching.
fn dispatch_action(
    action: Action,
    app: &App,
    allanime: &AllAnimeClient,
    anilist: &AniListClient,
    tx: &mpsc::UnboundedSender<Action>,
) {
    match action {
        Action::Search(query) => {
            let mode = app.mode_str().to_string();
            let allanime = allanime.clone();
            let anilist = anilist.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(Action::SearchLoading);
                match allanime.search(&query, &mode).await {
                    Ok(mut results) => {
                        // Enrich with AniList metadata (best effort)
                        if let Ok(anilist_results) = anilist.search(&query).await {
                            for result in &mut results {
                                if let Some(enriched) = anilist_results
                                    .iter()
                                    .find(|a| fuzzy_match(&a.title, &result.title))
                                {
                                    result.synopsis = enriched.synopsis.clone();
                                    result.poster_url = enriched.poster_url.clone();
                                    result.genres = enriched.genres.clone();
                                    result.rating = enriched.rating;
                                }
                            }
                        }
                        let _ = tx.send(Action::SearchResults(results));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::SearchError(e.to_string()));
                    }
                }
            });
        }

        Action::SelectAnime(idx) => {
            let tx = tx.clone();
            let allanime = allanime.clone();
            let mode = app.mode_str().to_string();

            if let Some(anime) = app.search_results.get(idx).cloned() {
                let show_id = anime.id.clone();
                // Immediately transition to detail screen
                let _ = tx.send(Action::AnimeDetail(Box::new(anime)));

                // Load episodes in background
                tokio::spawn(async move {
                    match allanime.episodes(&show_id, &mode).await {
                        Ok(episodes) => {
                            let _ = tx.send(Action::EpisodesLoaded(episodes));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::Error(format!(
                                "Failed to load episodes: {e}"
                            )));
                        }
                    }
                });
            }
        }

        Action::SelectEpisode(idx) => {
            let tx = tx.clone();
            let allanime = allanime.clone();
            let mode = app.mode_str().to_string();

            if let (Some(anime), Some(episode)) =
                (&app.selected_anime, app.episodes.get(idx))
            {
                let show_id = anime.id.clone();
                let ep_str = format_episode_number(episode.number);
                let title = anime.title.clone();
                let ep_display = ep_str.clone();
                let player_name = app.config.player.name;
                let custom_cmd = app.config.player.custom_command.clone();

                // Show loading state immediately
                let _ = tx.send(Action::PlayLoading(format!(
                    "{title} — Episode {ep_display}"
                )));

                tokio::spawn(async move {
                    match allanime
                        .get_stream_urls(&show_id, &ep_str, &mode)
                        .await
                    {
                        Ok(streams) if !streams.is_empty() => {
                            let stream = &streams[0];
                            let opts = player::PlayOptions {
                                url: stream.url.clone(),
                                title: format!("{title} Episode {ep_display}"),
                                referer: stream.referer.clone(),
                                subtitle_path: None,
                            };
                            match player::launch(
                                player_name,
                                custom_cmd.as_deref(),
                                opts,
                            )
                            .await
                            {
                                Ok(()) => {
                                    let _ = tx.send(Action::StreamsResolved(streams));
                                }
                                Err(e) => {
                                    let _ = tx.send(Action::PlayError(format!(
                                        "Player failed: {e}"
                                    )));
                                }
                            }
                        }
                        Ok(_) => {
                            let _ = tx.send(Action::PlayError(
                                "No streams found for this episode".to_string(),
                            ));
                        }
                        Err(e) => {
                            let _ = tx.send(Action::PlayError(format!(
                                "Stream resolution failed: {e}"
                            )));
                        }
                    }
                });
            }
        }

        _ => {}
    }
}

fn fuzzy_match(a: &str, b: &str) -> bool {
    let a = a.to_lowercase();
    let b = b.to_lowercase();
    a == b || a.contains(&b) || b.contains(&a)
}

fn format_episode_number(n: f32) -> String {
    if n == n.floor() {
        format!("{}", n as i32)
    } else {
        format!("{n}")
    }
}
