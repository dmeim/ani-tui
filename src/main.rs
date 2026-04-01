#![allow(dead_code)]

mod action;
mod app;
mod config;
mod tui;

mod api;
mod model;
mod player;
mod ui;

use std::process::Command;

use color_eyre::Result;
use ratatui_image::picker::Picker;
use tokio::sync::mpsc;

use crate::action::Action;
use crate::api::allanime::AllAnimeClient;
use crate::api::anilist::AniListClient;
use crate::api::jikan::JikanClient;
use crate::app::App;
use crate::config::{Config, MetadataProvider};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Handle CLI flags before launching the TUI
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("ani-tui {VERSION}");
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--update" => {
                return self_update();
            }
            "--uninstall" => {
                return self_uninstall();
            }
            other => {
                eprintln!("Unknown option: {other}");
                eprintln!("Run 'ani-tui --help' for usage.");
                std::process::exit(1);
            }
        }
    }

    let config = Config::load()?;

    // Detect terminal graphics protocol before entering alternate screen
    let picker = Picker::from_query_stdio().ok();

    let mut terminal = tui::init()?;
    let mut events = tui::EventHandler::new(4.0, 30.0);
    let mut app = App::new(config, picker);

    let allanime = AllAnimeClient::new()?;
    let anilist = AniListClient::new();
    let jikan = JikanClient::new();

    // Channel for async tasks to send actions back
    let (async_tx, mut async_rx) = mpsc::unbounded_channel::<Action>();

    loop {
        // Check for async results (non-blocking)
        while let Ok(action) = async_rx.try_recv() {
            if let Some(follow_up) = app.handle_action(action) {
                dispatch_action(follow_up, &app, &allanime, &anilist, &jikan, &async_tx);
            }
        }

        let action = events.next().await?;

        match action {
            Action::Render => {
                terminal.draw(|frame| ui::render(frame, &mut app))?;
                continue;
            }
            Action::Tick => {
                app.tick_count = app.tick_count.wrapping_add(1);
                continue;
            }
            _ => {}
        }

        if let Some(follow_up) = app.handle_action(action) {
            dispatch_action(follow_up, &app, &allanime, &anilist, &jikan, &async_tx);
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
    jikan: &JikanClient,
    tx: &mpsc::UnboundedSender<Action>,
) {
    match action {
        Action::Search(query) => {
            let mode = app.mode_str().to_string();
            let series_provider = app.config.general.series_provider;
            let allanime = allanime.clone();
            let anilist = anilist.clone();
            let jikan = jikan.clone();
            let tx = tx.clone();
            tokio::spawn(async move {
                let _ = tx.send(Action::SearchLoading);
                match allanime.search(&query, &mode).await {
                    Ok(mut results) => {
                        // Enrich with metadata from the configured series provider
                        match series_provider {
                            MetadataProvider::Jikan => {
                                if let Ok(jikan_results) = jikan.search(&query).await {
                                    for result in &mut results {
                                        if let Some(enriched) = jikan_results
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
                            }
                            MetadataProvider::Anilist => {
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
                            }
                            MetadataProvider::Anidb => {
                                // AniDB not yet implemented
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
            let jikan = jikan.clone();
            let mode = app.mode_str().to_string();
            let episode_provider = app.config.general.episode_provider;

            if let Some(anime) = app.search_results.get(idx).cloned() {
                let show_id = anime.id.clone();
                let anime_title = anime.title.clone();
                // Immediately transition to detail screen
                let _ = tx.send(Action::AnimeDetail(Box::new(anime)));

                // Load episodes in background
                let tx2 = tx.clone();
                tokio::spawn(async move {
                    match allanime.episodes(&show_id, &mode).await {
                        Ok(episodes) => {
                            let _ = tx.send(Action::EpisodesLoaded(episodes));

                            // Fetch episode details from Jikan if configured
                            if episode_provider == MetadataProvider::Jikan
                                && let Ok(jikan_results) = jikan.search(&anime_title).await
                                && let Some(matched) = jikan_results.first()
                                && let Ok(mal_id) = matched.id.parse::<i64>()
                            {
                                // Store the MAL ID so we can fetch per-episode synopses later
                                let _ = tx2.send(Action::SetMalId(mal_id));
                                // Small delay to respect rate limit
                                tokio::time::sleep(
                                    std::time::Duration::from_millis(350),
                                )
                                .await;
                                if let Ok(details) = jikan.episodes(mal_id).await {
                                    let _ = tx2.send(
                                        Action::EpisodeDetailsLoaded(details),
                                    );
                                }
                            }
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

        Action::LoadPoster(anime_id, url) => {
            let tx = tx.clone();
            let anilist = anilist.clone();
            let jikan = jikan.clone();
            let poster_provider = app.config.general.poster_provider;
            tokio::spawn(async move {
                let bytes = match poster_provider {
                    MetadataProvider::Jikan => jikan.download_image(&url).await,
                    _ => anilist.download_image(&url).await,
                };
                let Ok(bytes) = bytes else {
                    return;
                };
                // Decode image off the main thread to avoid blocking the UI
                let Ok(img) = tokio::task::spawn_blocking(move || {
                    image::load_from_memory(&bytes)
                }).await else {
                    return;
                };
                if let Ok(img) = img {
                    let _ = tx.send(Action::PosterLoaded(
                        anime_id,
                        crate::action::DecodedImage(img),
                    ));
                }
            });
        }

        Action::FetchEpisodeSynopsis(mal_id, ep_num) => {
            let tx = tx.clone();
            let jikan = jikan.clone();
            tokio::spawn(async move {
                if let Ok(detail) = jikan.episode_detail(mal_id, ep_num).await
                    && let Some(synopsis) = detail.synopsis
                {
                    let _ = tx.send(Action::EpisodeSynopsisLoaded(
                        ep_num as f32,
                        synopsis,
                    ));
                }
            });
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
                            // Streams are already sorted by quality (best first)
                            let stream = &streams[0];
                            let opts = player::PlayOptions {
                                url: stream.url.clone(),
                                title: format!("{title} Episode {ep_display}"),
                                referer: stream.referer.clone(),
                                subtitle_path: None,
                            };
                            // Update UI before launching player
                            let _ = tx.send(Action::StreamsResolved(streams));
                            if let Err(e) = player::launch(
                                player_name,
                                custom_cmd.as_deref(),
                                opts,
                            )
                            .await
                            {
                                let _ = tx.send(Action::PlayError(format!(
                                    "Player failed: {e}"
                                )));
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

fn print_help() {
    println!("ani-tui {VERSION} — Terminal UI for browsing and streaming anime");
    println!();
    println!("USAGE:");
    println!("  ani-tui              Launch the TUI");
    println!("  ani-tui --update     Pull latest changes, rebuild, and reinstall");
    println!("  ani-tui --uninstall  Remove ani-tui from your system");
    println!("  ani-tui --version    Show version");
    println!("  ani-tui --help       Show this help");
}

fn repo_path() -> Result<std::path::PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine data directory"))?;
    let path_file = data_dir.join("ani-tui").join(".repo-path");
    if !path_file.exists() {
        color_eyre::eyre::bail!(
            "Repo path not found. Please reinstall by running install.sh from the repo directory."
        );
    }
    let path = std::fs::read_to_string(&path_file)?.trim().to_string();
    Ok(std::path::PathBuf::from(path))
}

fn self_update() -> Result<()> {
    let repo = repo_path()?;
    println!("Updating ani-tui from {}", repo.display());

    println!("Pulling latest changes...");
    let status = Command::new("git")
        .args(["pull"])
        .current_dir(&repo)
        .status()?;
    if !status.success() {
        color_eyre::eyre::bail!("git pull failed");
    }

    println!("Building release...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&repo)
        .status()?;
    if !status.success() {
        color_eyre::eyre::bail!("cargo build failed");
    }

    let binary = repo.join("target/release/ani-tui");
    let install_dir = std::path::PathBuf::from("/usr/local/bin/ani-tui");

    println!("Installing to {}...", install_dir.display());
    let status = Command::new("sudo")
        .args(["cp", &binary.to_string_lossy(), &install_dir.to_string_lossy()])
        .status()?;
    if !status.success() {
        color_eyre::eyre::bail!("Failed to copy binary (sudo cp failed)");
    }

    println!("ani-tui updated successfully!");
    Ok(())
}

fn self_uninstall() -> Result<()> {
    let binary = "/usr/local/bin/ani-tui";

    // Prompt for confirmation
    println!("This will remove:");
    println!("  - {binary}");

    let config_dir = dirs::config_dir().map(|d| d.join("ani-tui"));
    let data_dir = dirs::data_dir().map(|d| d.join("ani-tui"));

    if let Some(ref dir) = config_dir {
        if dir.exists() {
            println!("  - {} (config)", dir.display());
        }
    }
    if let Some(ref dir) = data_dir {
        if dir.exists() {
            println!("  - {} (data/history)", dir.display());
        }
    }

    print!("\nProceed? [y/N] ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled.");
        return Ok(());
    }

    // Remove binary
    if std::path::Path::new(binary).exists() {
        let status = Command::new("sudo")
            .args(["rm", binary])
            .status()?;
        if !status.success() {
            color_eyre::eyre::bail!("Failed to remove binary");
        }
        println!("Removed {binary}");
    }

    // Remove config and data directories
    if let Some(dir) = config_dir {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            println!("Removed {}", dir.display());
        }
    }
    if let Some(dir) = data_dir {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
            println!("Removed {}", dir.display());
        }
    }

    println!("\nani-tui uninstalled successfully!");
    Ok(())
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
