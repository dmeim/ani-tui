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
use crate::config::{Config, MetadataProvider, MinQuality};
use crate::model::stream::StreamUrl;

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
                dispatch_action(follow_up, &mut app, &allanime, &anilist, &jikan, &async_tx);
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
            dispatch_action(follow_up, &mut app, &allanime, &anilist, &jikan, &async_tx);
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
    app: &mut App,
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
            let tx_synopsis = tx.clone();
            let jikan = jikan.clone();
            tokio::spawn(async move {
                if let Ok(detail) = jikan.episode_detail(mal_id, ep_num).await
                    && let Some(synopsis) = detail.synopsis
                {
                    let _ = tx_synopsis.send(Action::EpisodeSynopsisLoaded(
                        ep_num as f32,
                        synopsis,
                    ));
                }
            });

            // Also prefetch stream URLs for the highlighted episode
            if let (Some(anime), Some(episode)) =
                (&app.selected_anime, app.episodes.get(app.selected_episode))
            {
                let show_id = anime.id.clone();
                let ep_str = format_episode_number(episode.number);
                let mode = app.mode_str().to_string();
                if !app.stream_cache.contains_key(&ep_str) {
                    let tx_prefetch = tx.clone();
                    let allanime = allanime.clone();
                    tokio::spawn(async move {
                        if let Ok(streams) = allanime
                            .get_stream_urls(&show_id, &ep_str, &mode)
                            .await
                        {
                            if !streams.is_empty() {
                                let _ = tx_prefetch.send(Action::StreamsPrefetched(ep_str, streams));
                            }
                        }
                    });
                }
            }
        }

        Action::PrefetchStreams(show_id, ep_str, mode) => {
            if !app.stream_cache.contains_key(&ep_str) {
                let tx = tx.clone();
                let allanime = allanime.clone();
                tokio::spawn(async move {
                    if let Ok(streams) = allanime
                        .get_stream_urls(&show_id, &ep_str, &mode)
                        .await
                    {
                        if !streams.is_empty() {
                            let _ = tx.send(Action::StreamsPrefetched(ep_str, streams));
                        }
                    }
                });
            }
        }

        Action::PrefetchAllStreams(show_id, ep_strs, mode) => {
            // Filter out already-cached episodes
            let to_fetch: Vec<String> = ep_strs
                .into_iter()
                .filter(|ep| !app.stream_cache.contains_key(ep))
                .collect();

            if !to_fetch.is_empty() {
                let tx = tx.clone();
                let allanime = allanime.clone();
                // Single coordinator task with a semaphore to limit concurrency
                tokio::spawn(async move {
                    let sem = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
                    let mut handles = Vec::with_capacity(to_fetch.len());
                    for ep_str in to_fetch {
                        let tx = tx.clone();
                        let allanime = allanime.clone();
                        let show_id = show_id.clone();
                        let mode = mode.clone();
                        let permit = sem.clone().acquire_owned().await;
                        handles.push(tokio::spawn(async move {
                            let _permit = permit;
                            if let Ok(streams) = allanime
                                .get_stream_urls(&show_id, &ep_str, &mode)
                                .await
                            {
                                if !streams.is_empty() {
                                    let _ = tx.send(Action::StreamsPrefetched(ep_str, streams));
                                }
                            }
                        }));
                    }
                    for h in handles {
                        let _ = h.await;
                    }
                });
            }
        }

        Action::SelectEpisode(idx) => {
            let tx = tx.clone();
            let allanime = allanime.clone();
            let mode = app.mode_str().to_string();
            let min_quality = app.config.general.min_quality;

            if let (Some(anime), Some(episode)) =
                (&app.selected_anime, app.episodes.get(idx))
            {
                let show_id = anime.id.clone();
                let ep_str = format_episode_number(episode.number);
                let title = anime.title.clone();
                let ep_display = ep_str.clone();
                let player_name = app.config.player.name;
                let custom_cmd = app.config.player.custom_command.clone();

                // Check prefetch cache first
                let cached = app.stream_cache.remove(&ep_str);

                if let Some(streams) = cached {
                    // Cache hit — launch immediately
                    if let Some(stream) = pick_stream(&streams, min_quality) {
                        let opts = player::PlayOptions {
                            url: stream.url.clone(),
                            title: format!("{title} Episode {ep_display}"),
                            referer: stream.referer.clone(),
                            subtitle_path: None,
                        };
                        let _ = tx.send(Action::StreamsResolved(streams));
                        tokio::spawn(async move {
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
                        });
                    } else {
                        let _ = tx.send(Action::PlayError(
                            "No streams match your minimum quality setting".to_string(),
                        ));
                    }
                } else {
                    // Cache miss — show loading and fetch
                    let _ = tx.send(Action::PlayLoading(format!(
                        "{title} — Episode {ep_display}"
                    )));

                    tokio::spawn(async move {
                        match allanime
                            .get_stream_urls(&show_id, &ep_str, &mode)
                            .await
                        {
                            Ok(streams) if !streams.is_empty() => {
                                if let Some(stream) = pick_stream(&streams, min_quality) {
                                    let opts = player::PlayOptions {
                                        url: stream.url.clone(),
                                        title: format!("{title} Episode {ep_display}"),
                                        referer: stream.referer.clone(),
                                        subtitle_path: None,
                                    };
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
                                } else {
                                    let _ = tx.send(Action::PlayError(
                                        "No streams match your minimum quality setting".to_string(),
                                    ));
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
        }

        _ => {}
    }
}

fn print_help() {
    println!("ani-tui {VERSION} — Terminal UI for browsing and streaming anime");
    println!();
    println!("USAGE:");
    println!("  ani-tui              Launch the TUI");
    println!("  ani-tui --update     Download and install the latest release");
    println!("  ani-tui --uninstall  Remove ani-tui from your system");
    println!("  ani-tui --version    Show version");
    println!("  ani-tui --help       Show this help");
}

const REPO: &str = "dmeim/ani-tui";

fn install_dir() -> std::path::PathBuf {
    if cfg!(windows) {
        let local = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| {
            dirs::data_local_dir()
                .map(|d| d.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        std::path::PathBuf::from(local).join("ani-tui").join("bin")
    } else {
        std::path::PathBuf::from("/usr/local/bin")
    }
}

fn current_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => {
            eprintln!("Unsupported platform: {os}/{arch}");
            std::process::exit(1);
        }
    }
}

fn self_update() -> Result<()> {
    let target = current_target();
    println!("Checking for updates (platform: {target})...");

    let client = reqwest::blocking::Client::builder()
        .user_agent("ani-tui-updater")
        .build()?;

    let release: serde_json::Value = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .send()?
        .error_for_status()
        .map_err(|_| color_eyre::eyre::eyre!(
            "No releases found. Check https://github.com/{REPO}/releases"
        ))?
        .json()?;

    let tag = release["tag_name"]
        .as_str()
        .unwrap_or("unknown");
    let remote_version = tag.strip_prefix('v').unwrap_or(tag);

    if remote_version == VERSION {
        println!("Already up to date (v{VERSION}).");
        return Ok(());
    }

    println!("New version available: v{remote_version} (current: v{VERSION})");

    let assets = release["assets"]
        .as_array()
        .ok_or_else(|| color_eyre::eyre::eyre!("No assets in release"))?;

    let asset = assets
        .iter()
        .find(|a| {
            a["name"]
                .as_str()
                .is_some_and(|name| name.contains(target))
        })
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("No release asset found for {target}")
        })?;

    let download_url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| color_eyre::eyre::eyre!("Missing download URL"))?;

    println!("Downloading {download_url}...");
    let archive_bytes = client.get(download_url).send()?.bytes()?;

    let tmp_dir = std::env::temp_dir().join("ani-tui-update");
    std::fs::create_dir_all(&tmp_dir)?;

    let binary_name = if cfg!(windows) { "ani-tui.exe" } else { "ani-tui" };
    let extracted = tmp_dir.join(binary_name);

    if cfg!(windows) {
        let archive_path = tmp_dir.join("update.zip");
        std::fs::write(&archive_path, &archive_bytes)?;
        let status = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.display(),
                    tmp_dir.display()
                ),
            ])
            .status()?;
        if !status.success() {
            color_eyre::eyre::bail!("Failed to extract archive");
        }
    } else {
        let archive_path = tmp_dir.join("update.tar.gz");
        std::fs::write(&archive_path, &archive_bytes)?;
        let status = Command::new("tar")
            .args(["xzf", &archive_path.to_string_lossy(), "-C", &tmp_dir.to_string_lossy()])
            .status()?;
        if !status.success() {
            color_eyre::eyre::bail!("Failed to extract archive");
        }
    }

    let dest = install_dir().join(binary_name);
    println!("Installing to {}...", dest.display());

    if cfg!(windows) {
        std::fs::create_dir_all(dest.parent().unwrap())?;
        // On Windows, rename current exe out of the way first
        let old = dest.with_extension("old.exe");
        if dest.exists() {
            let _ = std::fs::rename(&dest, &old);
        }
        std::fs::copy(&extracted, &dest)?;
        let _ = std::fs::remove_file(&old);
    } else {
        let status = Command::new("sudo")
            .args(["install", "-m", "755", &extracted.to_string_lossy(), &dest.to_string_lossy()])
            .status()?;
        if !status.success() {
            color_eyre::eyre::bail!("Failed to install binary (sudo failed)");
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);

    println!("ani-tui updated to v{remote_version}!");
    Ok(())
}

fn self_uninstall() -> Result<()> {
    let binary_name = if cfg!(windows) { "ani-tui.exe" } else { "ani-tui" };
    let binary = install_dir().join(binary_name);
    let binary_display = binary.display().to_string();

    // Prompt for confirmation
    println!("This will remove:");
    println!("  - {binary_display}");

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
    if binary.exists() {
        if cfg!(windows) {
            std::fs::remove_file(&binary)?;
        } else {
            let status = Command::new("sudo")
                .args(["rm", &binary_display])
                .status()?;
            if !status.success() {
                color_eyre::eyre::bail!("Failed to remove binary");
            }
        }
        println!("Removed {binary_display}");
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

/// Pick the best stream that meets the minimum quality.
/// Streams are already sorted best-first, so find the first that passes.
/// Falls back to the best available stream if none meet the minimum.
fn pick_stream(streams: &[StreamUrl], min_quality: MinQuality) -> Option<&StreamUrl> {
    streams
        .iter()
        .find(|s| min_quality.accepts(s.quality))
        .or_else(|| streams.first())
}
