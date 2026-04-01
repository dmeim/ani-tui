# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ani-tui is a Rust TUI for browsing and streaming anime. It calls the same AllAnime GraphQL API that ani-cli uses for content (search, episodes, stream URLs), enriches results with metadata from AniList or AniDB (user's choice), and launches episodes in an external video player (iina, mpv, VLC, QuickTime).

Built with ratatui (terminal UI), tokio (async), reqwest (HTTP), and ratatui-image (inline poster rendering).

## Build Commands

```sh
cargo build                  # debug build
cargo build --release        # release build
cargo run                    # run debug
cargo test                   # run all tests
cargo test <test_name>       # run single test
cargo clippy                 # lint
cargo fmt                    # format
cargo fmt -- --check         # check formatting without modifying
```

## Architecture

**Async action-channel pattern** (from ratatui async template):
- `tui.rs` — terminal lifecycle + event stream (crossterm EventStream via tokio). Sends `Event::Key`, `Event::Tick`, `Event::Render` through an mpsc channel.
- `action.rs` — `Action` enum decouples input events from state mutations. Async work (API calls) spawns tokio tasks that send `Action` variants back through the channel.
- `app.rs` — `App` struct holds all state. Tracks current screen via an enum (`Search`, `Detail`, `Playing`, `Setup`). Dispatches actions to update state.

**API layer** (`src/api/`):
- `allanime.rs` — content source. Replicates ani-cli's GraphQL queries for search, episode lists, and stream URL resolution. Includes a port of ani-cli's character-mapping cipher for URL decryption.
- `anilist.rs` — metadata provider option A. Free GraphQL API, no auth. Provides cover images, synopses, ratings.
- `anidb.rs` — metadata provider option B. HTTP XML API, requires registered client ID, strict 1 req/2 sec rate limit.
- `opensubtitles.rs` — REST v2 API. Requires API key + JWT for downloads.

**UI layer** (`src/ui/`): each screen is a component that implements rendering and handles relevant actions. Poster images rendered via `ratatui-image` with auto-detected protocol (Kitty/Sixel/iTerm2/halfblock fallback).

**Player layer** (`src/player/`): `Player` trait with implementations for each supported player. Spawns external process via `tokio::process::Command`.

## CI/CD & Release

GitHub Actions (`.github/workflows/release.yml`) builds release binaries on every push to `master` and on tag push (`v*`). Targets:

| Target | Runner |
|--------|--------|
| `aarch64-apple-darwin` | `macos-latest` |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` |
| `x86_64-pc-windows-msvc` | `windows-latest` |

CI builds use `--no-default-features` to avoid the `chafa` system dependency. The `chafa` Cargo feature (enabled by default for local builds) provides halfblock image fallback but requires `libchafa-dev` installed on the system. Without it, image rendering still works via Kitty/Sixel/iTerm2 protocols.

On tag push, `softprops/action-gh-release` creates a GitHub Release with all archives attached. Asset naming: `ani-tui-<target>.tar.gz` (Unix) or `.zip` (Windows).

**Install scripts** (`install.sh`, `install.ps1`) download prebuilt binaries from GitHub Releases — no Rust toolchain required on the user's machine.

**Self-update** (`--update` flag in `main.rs`) uses `reqwest::blocking` to query the GitHub Releases API, compare versions, and download+install the correct binary for the current platform. No git or cargo needed.

**To publish a release:**

1. Bump version in `Cargo.toml`
2. Commit the version bump
3. Tag and push:
   ```sh
   git tag v0.x.x
   git push && git push --tags
   ```

The tag push triggers the full workflow: build all targets → create GitHub Release with binaries attached.

## Key Design Decisions

- **Wrapper, not reimplementation**: uses the same AllAnime API as ani-cli but calls it directly from Rust rather than shelling out to the bash script.
- **No in-terminal video playback**: only external players (iina, mpv, VLC, QuickTime).
- **Metadata provider is user-configurable**: AniList or AniDB, chosen during first-run setup wizard.
- **Config at** `~/.config/ani-tui/config.toml`, **history at** `~/.local/share/ani-tui/history.json`.

## Progress Tracking

`to-do.md` is the source of truth for what's done and what's next. Update it as tasks are completed.
