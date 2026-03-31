# ani-tui

A terminal user interface for browsing and watching anime, powered by the same content sources as [ani-cli](https://github.com/pystardust/ani-cli). Built with Rust and [ratatui](https://ratatui.rs).

ani-tui gives you a rich, interactive experience — search anime, view posters and descriptions, pick episodes, and launch them in your preferred video player — all without leaving the terminal.

## Features

- **Search & browse** anime with an interactive TUI
- **Metadata enrichment** — posters, synopses, ratings, and episode info from AniDB or AniList (your choice)
- **Inline poster display** — renders anime cover art directly in the terminal (Kitty, Sixel, iTerm2, or halfblock fallback)
- **Subtitle support** — fetch subtitles from OpenSubtitles and pass them to your player
- **Multiple video players** — iina, mpv, VLC, QuickTime, or any custom player
- **First-run setup wizard** — choose your metadata provider, preferred player, subtitle language, and more
- **Watch history** — track what you've watched and continue where you left off

## How It Works

ani-tui uses the same AllAnime GraphQL API that ani-cli uses to search for anime and resolve streaming URLs. On top of that, it layers rich metadata from your choice of provider:

```
AllAnime API ──→ Search results, episode lists, stream URLs
AniDB / AniList ──→ Posters, synopses, ratings, genres
OpenSubtitles ──→ Subtitle files (.srt/.ass)
```

The TUI handles all interaction — no `fzf`, no bash, no piping. You get a proper navigable interface with keyboard controls.

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

## First-Run Setup

On first launch, ani-tui walks you through configuration:

1. **Metadata provider** — Choose between:
   - **AniList** — free, no account needed, modern GraphQL API, great cover images
   - **AniDB** — comprehensive anime database, requires free client registration
2. **Video player** — Pick your preferred player:
   - iina (macOS)
   - mpv
   - VLC
   - QuickTime (macOS)
   - Custom (provide a command)
3. **Subtitle language** — Default language for OpenSubtitles lookups
4. **Dub vs Sub** — Default audio preference

Configuration is saved to `~/.config/ani-tui/config.toml` and can be changed anytime.

### Example config

```toml
[general]
metadata_provider = "anilist"  # or "anidb"
default_mode = "sub"           # or "dub"

[player]
name = "iina"
# custom_command = "/path/to/player"  # for custom players

[subtitles]
enabled = true
language = "en"
# opensubtitles_api_key = "..."  # optional, for higher rate limits

[anidb]
# client = "your-client-name"   # required if using anidb
# client_version = 1
```

## Architecture

```
src/
├── main.rs                 # Entry point (#[tokio::main])
├── app.rs                  # App state, mode transitions, action dispatch
├── tui.rs                  # Terminal lifecycle, event stream, tick/render loop
├── action.rs               # Action enum (Search, Select, Play, Quit, etc.)
├── config.rs               # Config loading/saving, first-run setup
├── api/
│   ├── mod.rs
│   ├── anilist.rs          # AniList GraphQL client (metadata + images)
│   ├── anidb.rs            # AniDB HTTP API client (metadata + images)
│   ├── allanime.rs         # AllAnime GraphQL client (search, episodes, streams)
│   └── opensubtitles.rs    # OpenSubtitles v2 REST client
├── player/
│   ├── mod.rs              # Player trait + factory
│   ├── mpv.rs
│   ├── iina.rs
│   ├── vlc.rs
│   └── quicktime.rs
├── ui/
│   ├── mod.rs
│   ├── search.rs           # Search input + results list
│   ├── detail.rs           # Poster + synopsis + episode list
│   ├── playing.rs          # Now-playing status bar
│   └── setup.rs            # First-run setup wizard
└── model/
    ├── mod.rs
    ├── anime.rs            # Anime, Episode structs
    └── stream.rs           # StreamUrl, Quality enums
```

### Tech Stack

| Crate | Purpose |
|-------|---------|
| `ratatui` | Terminal UI framework |
| `ratatui-image` | Inline image rendering (posters) |
| `crossterm` | Terminal backend + event handling |
| `tokio` | Async runtime for concurrent API calls |
| `reqwest` | HTTP client for all API calls |
| `serde` / `serde_json` | JSON serialization (AniList, AllAnime, OpenSubtitles) |
| `quick-xml` | XML parsing (AniDB responses) |
| `toml` | Config file parsing |
| `image` | Image decoding for poster rendering |
| `dirs` | XDG config directory resolution |
| `color-eyre` | Error handling and reporting |

## Building

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- A supported video player (mpv, iina, VLC, or QuickTime)

### Build & Run

```sh
git clone https://github.com/youruser/ani-tui.git
cd ani-tui
cargo build --release
./target/release/ani-tui
```

### Install from source

```sh
cargo install --path .
```

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

## Roadmap

### v0.1 — MVP
- [x] Project scaffolding
- [ ] AllAnime search + episode listing (port of ani-cli's GraphQL queries)
- [ ] AniList metadata integration (posters, synopses)
- [ ] Search screen with results list
- [ ] Detail screen with inline poster + synopsis
- [ ] Episode selection + stream URL resolution
- [ ] Player launching (mpv, iina, VLC, QuickTime)
- [ ] First-run setup wizard
- [ ] Config file support

### v0.2 — Enrichment
- [ ] AniDB as alternative metadata provider
- [ ] OpenSubtitles integration
- [ ] Watch history + continue watching
- [ ] Quality selection

### v0.3 — Polish
- [ ] Fuzzy search / autocomplete
- [ ] Anime recommendations
- [ ] Seasonal anime calendar
- [ ] Theming support

## License

MIT

## Credits

- [ani-cli](https://github.com/pystardust/ani-cli) — inspiration and content source discovery
- [ratatui](https://ratatui.rs) — terminal UI framework
- [AniList](https://anilist.co) — anime metadata API
- [AniDB](https://anidb.net) — anime metadata database
- [OpenSubtitles](https://opensubtitles.com) — subtitle database
