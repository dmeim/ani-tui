# ani-tui — Project To-Do

> Source of truth for project progress. Updated as we go.
> - [x] = done
> - [ ] = to do
> - [~] = in progress

---

## 1. Project Setup

### 1.1 Scaffold
- [x] `cargo init` the project
- [x] Add all dependencies to `Cargo.toml`
- [x] Set up module structure (`api/`, `ui/`, `player/`, `model/`)
- [x] Verify it compiles clean

### 1.2 Core Boilerplate
- [x] Set up `main.rs` with `#[tokio::main]` entry point
- [x] Create `tui.rs` — terminal init/restore, crossterm event stream, tick/render loop
- [x] Create `action.rs` — `Action` enum (Search, Select, Back, Play, Quit, Tick, Render, etc.)
- [x] Create `app.rs` — `App` struct with screen state machine, action dispatch
- [x] Verify the app starts, renders a blank screen, and quits cleanly on `q`

---

## 2. Data Models

### 2.1 Core Structs
- [x] `model/anime.rs` — `Anime` (id, title, synopsis, poster_url, episode_count, genres, rating)
- [x] `model/anime.rs` — `Episode` (number, title, is_filler)
- [x] `model/stream.rs` — `StreamUrl` (url, quality, provider, referer)
- [x] `model/stream.rs` — `Quality` enum (360p, 480p, 720p, 1080p)

### 2.2 Config Model
- [x] `config.rs` — `Config` struct matching the TOML schema in README
- [x] Load from `~/.config/ani-tui/config.toml`
- [x] Save/update config file
- [x] Defaults when no config exists

---

## 3. API Clients

### 3.1 AllAnime (Content Source)
- [x] `api/allanime.rs` — search anime (GraphQL query, same as ani-cli)
- [x] `api/allanime.rs` — get episode list by show ID
- [x] `api/allanime.rs` — resolve stream URLs (provider fetch + decryption)
- [x] Port ani-cli's `decrypt_allanime` character cipher to Rust
- [x] Handle sub vs dub mode toggle

### 3.2 AniList (Metadata Provider — Option A)
- [x] `api/anilist.rs` — search by title (GraphQL)
- [x] `api/anilist.rs` — fetch detail by ID (synopsis, cover image URL, genres, rating, status)
- [ ] `api/anilist.rs` — fetch episode info if available
- [x] Image download + caching for posters

### 3.3 AniDB (Metadata Provider — Option B)
- [ ] `api/anidb.rs` — download + parse `anime-titles.xml.gz` for local search
- [ ] `api/anidb.rs` — fetch anime detail by AID (HTTP API, XML response)
- [ ] `api/anidb.rs` — poster image URL extraction from XML
- [ ] Respect 1 req / 2 sec rate limit
- [ ] Client registration handling in config

### 3.4 OpenSubtitles
- [ ] `api/opensubtitles.rs` — search subtitles by title + episode
- [ ] `api/opensubtitles.rs` — download subtitle file
- [ ] JWT auth flow (login, token caching)
- [ ] Save downloaded .srt/.ass to temp dir for player consumption

---

## 4. Player Integration

### 4.1 Player Trait
- [x] `player/mod.rs` — `Player` trait: `launch(url, title, subtitle_path) -> Result`
- [x] `player/mod.rs` — player factory: config → correct `Player` impl

### 4.2 Player Implementations
- [x] `player/mpv.rs` — launch with `--force-media-title`, subtitle, referrer flags
- [x] `player/iina.rs` — launch iina-cli with `--mpv-*` flags
- [x] `player/vlc.rs` — launch with `--meta-title`, `--http-referrer`, subtitle flags
- [x] `player/quicktime.rs` — launch via `open -a "QuickTime Player"`
- [x] Custom player support (raw command from config)

### 4.3 Player Detection
- [x] Auto-detect installed players on first run
- [x] Validate chosen player exists before launching

---

## 5. UI Screens

### 5.1 Search Screen
- [x] Text input widget for search query
- [x] Trigger search on Enter
- [x] Loading spinner while fetching results
- [x] Display results as a navigable list (title + episode count)
- [x] Empty state when no results

### 5.2 Detail Screen
- [x] Layout: poster (left) + synopsis (right)
- [x] Poster rendering via `ratatui-image` with protocol auto-detection
- [x] Scrollable synopsis text
- [x] Genre / rating / status info bar
- [x] Episode list below (navigable)
- [x] Graceful fallback when poster can't render (text-only mode)

### 5.3 Playing Screen
- [x] "Now playing" status bar (anime title, episode number)
- [x] Controls hint (next episode, previous, back to detail)
- [ ] Auto-advance to next episode option

### 5.4 Setup Wizard
- [x] Step 1: metadata provider selection (AniList / AniDB)
- [x] Step 2: player selection (auto-detect installed, let user pick)
- [ ] Step 3: subtitle language preference
- [x] Step 4: sub vs dub default
- [x] Write config to disk on completion
- [x] Skip wizard if config already exists

---

## 6. Navigation & Input

### 6.1 Keybindings
- [ ] Global: `q` quit, `?` help, `Esc` back
- [ ] Lists: `j`/`k`/`↑`/`↓` navigate, `Enter`/`l`/`→` select, `h`/`←` back
- [ ] Search: `/` or `i` to focus input, `Esc` to unfocus
- [ ] Detail: scroll synopsis, navigate episode list

### 6.2 Screen State Machine
- [ ] `App` tracks current screen enum (Search, Detail, Playing, Setup)
- [ ] Clean transitions between screens
- [ ] Back stack (Esc always returns to previous screen)

---

## 7. Watch History

- [ ] Store history in `~/.local/share/ani-tui/history.json`
- [ ] Record: anime ID, title, last episode watched, timestamp
- [ ] "Continue watching" option on search screen
- [ ] Update history after each episode plays

---

## 8. Quality & Polish

### 8.1 Error Handling
- [ ] Graceful error display in TUI (network errors, player not found, etc.)
- [ ] No panics — all errors surfaced to user
- [ ] Retry option for failed network requests

### 8.2 Performance
- [ ] Cache metadata responses (in-memory for session, on-disk for posters)
- [ ] Debounce search input (don't fire on every keystroke)
- [ ] Async image loading (don't block UI while fetching posters)

### 8.3 UX
- [ ] Responsive layout (adapt to terminal size)
- [ ] Color scheme that works on light and dark terminals
- [ ] Help overlay (`?` key)
- [ ] Status bar with hints for available actions

---

## 9. Testing & CI

- [ ] Unit tests for AllAnime URL decryption
- [ ] Unit tests for config parsing
- [ ] Integration tests for API clients (mock responses)
- [ ] CI pipeline (GitHub Actions: build, clippy, test)
- [ ] `cargo fmt` + `cargo clippy` clean

---

## 10. Packaging & Release

- [ ] Finalize README with real repo URL and screenshots
- [ ] `cargo publish` prep (license, metadata in Cargo.toml)
- [ ] Homebrew formula or install script
- [ ] Release binary builds (macOS arm64/x86, Linux)
- [ ] AUR package (optional)
