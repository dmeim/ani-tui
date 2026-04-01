# ani-tui

A terminal user interface for browsing and watching anime, powered by the same content sources as [ani-cli](https://github.com/pystardust/ani-cli). Built with Rust and [ratatui](https://ratatui.rs).

Search anime, view posters and descriptions, pick episodes, and launch them in your preferred video player — all without leaving the terminal.

<!-- TODO: add a screenshot/gif here -->

## Features

- **Search & browse** anime with an interactive TUI
- **Metadata enrichment** — posters, synopses, ratings, and episode info from Jikan/MAL, AniList, or AniDB (your choice, mix-and-match per data type)
- **Inline poster display** — renders anime cover art directly in the terminal (Kitty, Sixel, iTerm2, or halfblock fallback)
- **Multiple video players** — mpv, iina, VLC, QuickTime, or any custom player
- **First-run setup wizard** — choose your metadata providers, preferred player, and audio preference
- **Configurable** — everything lives in a single `config.toml` you can hand-edit

## How It Works

ani-tui uses the same AllAnime GraphQL API that ani-cli uses to search for anime and resolve streaming URLs. On top of that, it layers rich metadata from your choice of provider:

```
AllAnime API  ──→  Search results, episode lists, stream URLs
Jikan / MAL   ──→  Posters, synopses, ratings, genres (default)
AniList       ──→  Posters, synopses, ratings, genres (alternative)
```

The TUI handles all interaction — no `fzf`, no bash, no piping. You get a navigable interface with keyboard controls.

## Screen Flow

```
┌──────────┐    ┌──────────────┐    ┌───────────────────┐    ┌──────────┐
│  Search  │───→│ Results List │───→│   Anime Detail    │───→│ Playing  │
│          │    │              │    │ ┌──────┬────────┐ │    │ (launches│
│  type to │    │  navigate    │    │ │poster│synopsis│ │    │  your    │
│  search  │    │  with j/k    │    │ ├──────┴────────┤ │    │  player) │
│          │    │              │    │ │ episode list  │ │    │          │
└──────────┘    └──────────────┘    │ └───────────────┘ │    └──────────┘
     ↑                              └───────────────────┘         │
     └────────────────────────────────────────────────────────────┘
                              Esc / back
```

## Installation

### Prerequisites

- A supported video player (mpv, iina, VLC, or QuickTime)

### macOS / Linux

```sh
curl -fsSL https://raw.githubusercontent.com/dmeim/ani-tui/master/install.sh | bash
```

This downloads the latest prebuilt binary for your platform and installs it to `/usr/local/bin/ani-tui`. No Rust toolchain required.

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/dmeim/ani-tui/master/install.ps1 | iex
```

Installs to `%LOCALAPPDATA%\ani-tui\bin\` and adds it to your user PATH.

### Build from source

If you prefer to build from source (requires Rust 1.85+):

```sh
git clone https://github.com/dmeim/ani-tui.git && cd ani-tui
cargo build --release
# copy target/release/ani-tui to a directory on your PATH
```

### Update

```sh
ani-tui --update
```

Downloads and installs the latest release from GitHub.

### Uninstall

```sh
ani-tui --uninstall
```

Removes the binary, config, and data directories after confirmation.

## Configuration

On first launch, ani-tui walks you through a setup wizard. Configuration is saved to `~/.config/ani-tui/config.toml` and can be hand-edited anytime.

### Example config

```toml
[general]
series_provider = "jikan"     # "jikan", "anilist", or "anidb"
episode_provider = "jikan"    # "jikan", "anilist", or "anidb"
poster_provider = "jikan"     # "jikan", "anilist", or "anidb"
default_mode = "sub"          # "sub" or "dub"

[player]
name = "mpv"                  # "mpv", "iina", "vlc", "quicktime", or "custom"
# custom_command = "/path/to/player"

[subtitles]
enabled = true
language = "en"
# opensubtitles_api_key = "..."

[anidb]
# client = "your-client-name"
# client_version = 1
```

### Metadata providers

You can mix-and-match providers for different data types (series info, episode details, poster images):

| Provider | Auth | Notes |
|----------|------|-------|
| **Jikan/MAL** (default) | None | REST API, good cover images, 3 req/sec rate limit |
| **AniList** | None | GraphQL API, modern, good cover images |
| **AniDB** | Client ID | XML API, comprehensive, 1 req/2 sec rate limit |

## Keyboard Controls

| Key | Action |
|-----|--------|
| `/` or `i` | Focus search input |
| `Enter` | Confirm selection |
| `j` / `k` or `↓` / `↑` | Navigate lists |
| `l` or `→` or `Enter` | Open detail / select episode |
| `h` or `←` or `Esc` | Go back |
| `q` | Quit |
| `?` | Show help |

## Architecture

```
src/
├── main.rs                 # Entry point (#[tokio::main])
├── app.rs                  # App state, mode transitions, action dispatch
├── tui.rs                  # Terminal lifecycle, event stream, tick/render loop
├── action.rs               # Action enum (Search, Select, Play, Quit, etc.)
├── config.rs               # Config loading/saving, first-run setup
├── api/
│   ├── allanime.rs         # AllAnime GraphQL (search, episodes, streams)
│   ├── jikan.rs            # Jikan/MAL REST client (metadata + images)
│   └── anilist.rs          # AniList GraphQL client (metadata + images)
├── player/
│   ├── mod.rs              # Player trait + factory (+ QuickTime, custom)
│   ├── mpv.rs              # mpv player
│   ├── iina.rs             # iina player (macOS)
│   └── vlc.rs              # VLC player
├── ui/
│   ├── search.rs           # Search input + results list
│   ├── detail.rs           # Poster + synopsis + episode list
│   └── setup.rs            # First-run setup wizard
└── model/
    ├── anime.rs            # Anime, Episode structs
    └── stream.rs           # StreamUrl, Quality enums
```

**Async action-channel pattern** (from ratatui async template): `tui.rs` drives the event loop (crossterm EventStream via tokio), actions decouple input from state mutations, and `app.rs` dispatches everything.

### Tech Stack

| Crate | Purpose |
|-------|---------|
| `ratatui` | Terminal UI framework |
| `ratatui-image` | Inline image rendering (posters) |
| `crossterm` | Terminal backend + event handling |
| `tokio` | Async runtime |
| `reqwest` | HTTP client |
| `serde` / `serde_json` | JSON serialization |
| `quick-xml` | XML parsing (AniDB) |
| `toml` | Config file parsing |
| `image` | Image decoding for posters |
| `dirs` | XDG directory resolution |
| `color-eyre` | Error handling |

## Roadmap

- [ ] OpenSubtitles integration (search + download subtitles)
- [ ] Watch history (continue where you left off)
- [ ] AniDB metadata provider
- [ ] Auto-advance to next episode
- [ ] Quality selection UI
- [ ] Fuzzy search / autocomplete
- [ ] Theming support

## License

MIT

## Credits

- [ani-cli](https://github.com/pystardust/ani-cli) — inspiration and content source discovery
- [ratatui](https://ratatui.rs) — terminal UI framework
- [Jikan](https://jikan.moe) — MyAnimeList unofficial API
- [AniList](https://anilist.co) — anime metadata API
- [AniDB](https://anidb.net) — anime metadata database
