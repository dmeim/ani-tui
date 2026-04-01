use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;

use crate::action::Action;
use crate::config::{AudioMode, Config, MetadataProvider, MinQuality, PlayerName};

/// Number of setting rows in the settings modal.
const SETTINGS_ROW_COUNT: usize = 6;
use crate::model::anime::{Anime, Episode};
use crate::model::stream::StreamUrl;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Search,
    Detail,
    Setup,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalKind {
    Settings,
    Player,
    Search,
}

pub struct App {
    pub should_quit: bool,
    pub screen: Screen,
    pub input_mode: InputMode,
    pub config: Config,

    // Search state
    pub search_input: String,
    pub cursor_position: usize,
    pub search_results: Vec<Anime>,
    pub selected_result: usize,
    pub search_loading: bool,
    pub search_error: Option<String>,

    // Detail state
    pub selected_anime: Option<Anime>,
    pub episodes: Vec<Episode>,
    pub selected_episode: usize,
    pub detail_scroll: u16,
    pub mal_id: Option<i64>,
    pub synopsis_requested: HashSet<i32>, // episode numbers we've already requested

    // Playing state
    pub streams: Vec<StreamUrl>,
    pub now_playing_title: Option<String>,
    pub now_playing_episode: Option<String>,
    pub play_error: Option<String>,

    // Stream prefetch cache: episode_str -> resolved streams
    pub stream_cache: HashMap<String, Vec<StreamUrl>>,

    // Setup wizard state
    pub setup_step: usize,
    pub setup_selected: usize,

    // Modal state
    pub active_modal: Option<ModalKind>,
    pub settings_cursor: usize,        // which setting row is focused (0..2)
    pub settings_editing: bool,        // whether the dropdown is open
    pub settings_option_cursor: usize, // cursor within the open dropdown

    // Poster image state
    pub picker: Option<Picker>,
    pub poster_cache: HashMap<String, (u32, u32, StatefulProtocol)>, // (img_w, img_h, protocol)
    pub poster_loading: Option<String>,

    // Animation
    pub tick_count: usize,

    // General errors
    pub error_message: Option<String>,
}

impl App {
    pub fn new(config: Config, picker: Option<Picker>) -> Self {
        let needs_setup = Config::needs_setup().unwrap_or(false);
        let start_with_search = !needs_setup;
        Self {
            should_quit: false,
            screen: if needs_setup { Screen::Setup } else { Screen::Search },
            input_mode: if start_with_search { InputMode::Editing } else { InputMode::Normal },
            config,
            search_input: String::new(),
            cursor_position: 0,
            search_results: Vec::new(),
            selected_result: 0,
            search_loading: false,
            search_error: None,
            selected_anime: None,
            episodes: Vec::new(),
            selected_episode: 0,
            detail_scroll: 0,
            mal_id: None,
            synopsis_requested: HashSet::new(),
            streams: Vec::new(),
            stream_cache: HashMap::new(),
            now_playing_title: None,
            now_playing_episode: None,
            play_error: None,
            setup_step: 0,
            setup_selected: 0,
            active_modal: None,
            settings_cursor: 0,
            settings_editing: false,
            settings_option_cursor: 0,
            picker,
            poster_cache: HashMap::new(),
            poster_loading: None,
            tick_count: 0,
            error_message: None,
        }
    }

    /// Returns a LoadPoster action if the currently selected anime has a poster_url
    /// and we haven't already cached or started loading it.
    fn load_selected_poster(&mut self) -> Option<Action> {
        let anime = self.search_results.get(self.selected_result)?;
        let url = anime.poster_url.as_ref()?;
        if self.poster_cache.contains_key(&anime.id) {
            return None;
        }
        if self.poster_loading.as_deref() == Some(&anime.id) {
            return None;
        }
        self.poster_loading = Some(anime.id.clone());
        Some(Action::LoadPoster(anime.id.clone(), url.clone()))
    }

    /// If the currently selected episode has no synopsis and we have a MAL ID,
    /// return an action to fetch it from Jikan.
    fn maybe_load_episode_synopsis(&mut self) -> Option<Action> {
        let mal_id = self.mal_id?;
        let ep = self.episodes.get(self.selected_episode)?;
        if ep.synopsis.is_some() {
            return None;
        }
        let ep_num = ep.number;
        // Only fetch for integer episode numbers (Jikan doesn't support fractional)
        if ep_num != ep_num.floor() {
            return None;
        }
        let ep_int = ep_num as i32;
        // Don't re-fetch episodes we've already requested
        if !self.synopsis_requested.insert(ep_int) {
            return None;
        }
        Some(Action::FetchEpisodeSynopsis(mal_id, ep_int))
    }

    fn maybe_prefetch_streams(&self) -> Option<Action> {
        let anime = self.selected_anime.as_ref()?;
        let ep = self.episodes.get(self.selected_episode)?;
        let ep_str = if ep.number == ep.number.floor() {
            format!("{}", ep.number as i32)
        } else {
            format!("{}", ep.number)
        };
        if self.stream_cache.contains_key(&ep_str) {
            return None;
        }
        Some(Action::PrefetchStreams(
            anime.id.clone(),
            ep_str,
            self.mode_str().to_string(),
        ))
    }

    pub fn mode_str(&self) -> &str {
        match self.config.general.default_mode {
            AudioMode::Sub => "sub",
            AudioMode::Dub => "dub",
        }
    }

    /// Handle an action and optionally return a follow-up action to dispatch asynchronously.
    pub fn handle_action(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::Quit => {
                self.should_quit = true;
                None
            }
            Action::Key(key) => self.handle_key(key.code, key.modifiers),
            Action::Back => {
                self.go_back();
                None
            }

            // Search results
            Action::SearchLoading => {
                self.search_loading = true;
                self.search_error = None;
                None
            }
            Action::SearchResults(results) => {
                self.search_loading = false;
                self.search_results = results;
                self.selected_result = 0;
                self.poster_cache.clear();
                // Dismiss search modal when results arrive
                if !self.search_results.is_empty() {
                    self.active_modal = None;
                    self.input_mode = InputMode::Normal;
                }
                self.load_selected_poster()
            }
            Action::SearchError(e) => {
                self.search_loading = false;
                self.search_error = Some(e);
                None
            }

            // Detail
            Action::AnimeDetail(anime) => {
                self.selected_anime = Some(*anime);
                self.screen = Screen::Detail;
                self.selected_episode = 0;
                self.detail_scroll = 0;
                self.stream_cache.clear();
                None
            }
            Action::EpisodesLoaded(eps) => {
                self.episodes = eps;
                // Trigger background prefetch of stream URLs for all episodes
                if let Some(anime) = &self.selected_anime {
                    let ep_strs: Vec<String> = self.episodes.iter().map(|ep| {
                        if ep.number == ep.number.floor() {
                            format!("{}", ep.number as i32)
                        } else {
                            format!("{}", ep.number)
                        }
                    }).collect();
                    if !ep_strs.is_empty() {
                        return Some(Action::PrefetchAllStreams(
                            anime.id.clone(),
                            ep_strs,
                            self.mode_str().to_string(),
                        ));
                    }
                }
                None
            }
            Action::SetMalId(id) => {
                self.mal_id = Some(id);
                // Now that we have the MAL ID, try to load synopsis for the current episode
                self.maybe_load_episode_synopsis()
            }
            Action::EpisodeDetailsLoaded(details) => {
                // Merge Jikan episode details into existing episodes by number
                for ep in &mut self.episodes {
                    if let Some(detail) = details.iter().find(|d| {
                        (d.number - ep.number).abs() < 0.01
                    }) {
                        if ep.title.is_none() {
                            ep.title = detail.title.clone();
                        }
                        if ep.synopsis.is_none() {
                            ep.synopsis = detail.synopsis.clone();
                        }
                        if !ep.is_filler && detail.is_filler {
                            ep.is_filler = detail.is_filler;
                        }
                        if ep.aired.is_none() {
                            ep.aired = detail.aired.clone();
                        }
                    }
                }
                // Trigger synopsis load for the currently selected episode
                self.maybe_load_episode_synopsis()
            }
            Action::EpisodeSynopsisLoaded(ep_num, synopsis) => {
                if let Some(ep) = self.episodes.iter_mut().find(|e| {
                    (e.number - ep_num).abs() < 0.01
                }) {
                    ep.synopsis = Some(synopsis);
                }
                None
            }

            // Playback
            Action::PlayLoading(info) => {
                self.active_modal = Some(ModalKind::Player);
                self.play_error = None;
                self.streams.clear();
                self.now_playing_title = Some(info);
                None
            }
            Action::StreamsResolved(streams) => {
                self.streams = streams;
                self.play_error = None;
                self.active_modal = Some(ModalKind::Player);
                if let Some(ref anime) = self.selected_anime {
                    self.now_playing_title = Some(anime.title.clone());
                }
                if let Some(ep) = self.episodes.get(self.selected_episode) {
                    self.now_playing_episode = Some(format!("{}", ep.number));
                }
                None
            }
            Action::StreamsPrefetched(ep_str, streams) => {
                self.stream_cache.insert(ep_str, streams);
                None
            }
            Action::PlayError(e) => {
                self.play_error = Some(e);
                self.active_modal = Some(ModalKind::Player);
                None
            }

            // Poster
            Action::PosterLoaded(anime_id, decoded) => {
                self.poster_loading = None;
                let (img_w, img_h) = (decoded.0.width(), decoded.0.height());
                if let Some(ref mut picker) = self.picker {
                    let protocol = picker.new_resize_protocol(decoded.0);
                    self.poster_cache.insert(anime_id, (img_w, img_h, protocol));
                }
                None
            }
            Action::LoadPoster(_, _) => {
                // Handled by dispatch_action in main.rs
                None
            }

            Action::Error(e) => {
                self.error_message = Some(e);
                None
            }

            _ => None,
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Option<Action> {
        // Ctrl+C always quits
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            self.should_quit = true;
            return None;
        }

        // Modal keys take priority
        if self.active_modal.is_some() {
            return self.handle_modal_key(code);
        }

        match self.input_mode {
            InputMode::Editing => self.handle_editing_key(code),
            InputMode::Normal => match self.screen {
                Screen::Search => self.handle_search_normal(code),
                Screen::Detail => self.handle_detail_normal(code),
                Screen::Setup => self.handle_setup_normal(code),
            },
        }
    }

    // -- Search screen (normal mode) --

    fn handle_search_normal(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                None
            }
            KeyCode::Char('s') | KeyCode::Char('i') => {
                self.active_modal = Some(ModalKind::Search);
                self.input_mode = InputMode::Editing;
                None
            }
            KeyCode::Char('/') => {
                self.open_settings_modal();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.search_results.is_empty() {
                    self.selected_result = (self.selected_result + 1).min(self.search_results.len() - 1);
                }
                self.load_selected_poster()
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_result = self.selected_result.saturating_sub(1);
                self.load_selected_poster()
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                if !self.search_results.is_empty() {
                    Some(Action::SelectAnime(self.selected_result))
                } else {
                    None
                }
            }
            KeyCode::Esc => {
                self.should_quit = true;
                None
            }
            _ => None,
        }
    }

    // -- Search screen (editing mode) --

    fn handle_editing_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                if self.search_results.is_empty() {
                    self.should_quit = true;
                } else {
                    self.input_mode = InputMode::Normal;
                }
                None
            }
            KeyCode::Tab => {
                self.open_settings_modal();
                None
            }
            KeyCode::Char(c) => {
                self.search_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                None
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.search_input.remove(self.cursor_position);
                }
                None
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.search_input.len() {
                    self.cursor_position += 1;
                }
                None
            }
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    self.input_mode = InputMode::Normal;
                    Some(Action::Search(self.search_input.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // -- Detail screen --

    fn handle_detail_normal(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                None
            }
            KeyCode::Char('/') => {
                self.open_settings_modal();
                None
            }
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                self.go_back();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.episodes.is_empty() {
                    self.selected_episode = (self.selected_episode + 1).min(self.episodes.len() - 1);
                }
                self.maybe_load_episode_synopsis()
                    .or_else(|| self.maybe_prefetch_streams())
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_episode = self.selected_episode.saturating_sub(1);
                self.maybe_load_episode_synopsis()
                    .or_else(|| self.maybe_prefetch_streams())
            }
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                if !self.episodes.is_empty() {
                    Some(Action::SelectEpisode(self.selected_episode))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    // -- Modal key handling --

    fn handle_modal_key(&mut self, code: KeyCode) -> Option<Action> {
        match self.active_modal {
            Some(ModalKind::Player) => self.handle_player_modal_key(code),
            Some(ModalKind::Settings) => self.handle_settings_modal_key(code),
            Some(ModalKind::Search) => self.handle_search_modal_key(code),
            None => None,
        }
    }

    fn handle_search_modal_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Esc => {
                if self.search_results.is_empty() {
                    self.should_quit = true;
                } else {
                    self.active_modal = None;
                    self.input_mode = InputMode::Normal;
                }
                None
            }
            KeyCode::Char(c) => {
                self.search_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                None
            }
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.search_input.remove(self.cursor_position);
                }
                None
            }
            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
                None
            }
            KeyCode::Right => {
                if self.cursor_position < self.search_input.len() {
                    self.cursor_position += 1;
                }
                None
            }
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    Some(Action::Search(self.search_input.clone()))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_player_modal_key(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                None
            }
            KeyCode::Esc => {
                // Dismiss the player modal (go back to Detail)
                self.active_modal = None;
                self.play_error = None;
                None
            }
            KeyCode::Char('n') => {
                // Next episode
                if self.selected_episode + 1 < self.episodes.len() {
                    self.selected_episode += 1;
                    Some(Action::SelectEpisode(self.selected_episode))
                } else {
                    None
                }
            }
            KeyCode::Char('p') => {
                // Previous episode
                if self.selected_episode > 0 {
                    self.selected_episode -= 1;
                    Some(Action::SelectEpisode(self.selected_episode))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn handle_settings_modal_key(&mut self, code: KeyCode) -> Option<Action> {
        if self.settings_editing {
            // Dropdown is open — navigate options
            let max_options = self.settings_option_count(self.settings_cursor);
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.settings_option_cursor =
                        (self.settings_option_cursor + 1).min(max_options - 1);
                    None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.settings_option_cursor =
                        self.settings_option_cursor.saturating_sub(1);
                    None
                }
                KeyCode::Enter | KeyCode::Esc => {
                    // Apply the selected option and close the dropdown
                    self.apply_settings_option(self.settings_cursor, self.settings_option_cursor);
                    let _ = self.config.save();
                    self.settings_editing = false;
                    None
                }
                _ => None,
            }
        } else {
            // Navigate between setting rows
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.settings_cursor = (self.settings_cursor + 1).min(SETTINGS_ROW_COUNT - 1);
                    None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.settings_cursor = self.settings_cursor.saturating_sub(1);
                    None
                }
                KeyCode::Enter => {
                    // Open the dropdown for this setting, pre-select current value
                    self.settings_editing = true;
                    self.settings_option_cursor = self.current_option_index(self.settings_cursor);
                    None
                }
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('/') => {
                    self.active_modal = None;
                    None
                }
                _ => None,
            }
        }
    }

    fn open_settings_modal(&mut self) {
        self.active_modal = Some(ModalKind::Settings);
        self.settings_cursor = 0;
        self.settings_editing = false;
        self.settings_option_cursor = 0;
    }

    /// How many options does a given setting row have?
    fn settings_option_count(&self, row: usize) -> usize {
        match row {
            0 => 3, // series provider: Jikan, AniList, AniDB
            1 => 3, // episode provider: Jikan, AniList, AniDB
            2 => 3, // poster provider: Jikan, AniList, AniDB
            3 => crate::player::detect_installed().len() + 1, // players + Custom
            4 => 2, // audio: Sub, Dub
            5 => 5, // min quality: Any, 360p, 480p, 720p, 1080p
            _ => 1,
        }
    }

    /// The index of the currently-active option for a given setting row.
    fn current_option_index(&self, row: usize) -> usize {
        match row {
            0 => provider_to_index(self.config.general.series_provider),
            1 => provider_to_index(self.config.general.episode_provider),
            2 => provider_to_index(self.config.general.poster_provider),
            3 => {
                let detected = crate::player::detect_installed();
                detected
                    .iter()
                    .position(|p| *p == self.config.player.name)
                    .unwrap_or(detected.len()) // falls through to Custom
            }
            4 => match self.config.general.default_mode {
                AudioMode::Sub => 0,
                AudioMode::Dub => 1,
            },
            5 => min_quality_to_index(self.config.general.min_quality),
            _ => 0,
        }
    }

    fn apply_settings_option(&mut self, row: usize, option: usize) {
        match row {
            0 => self.config.general.series_provider = index_to_provider(option),
            1 => self.config.general.episode_provider = index_to_provider(option),
            2 => self.config.general.poster_provider = index_to_provider(option),
            3 => {
                let detected = crate::player::detect_installed();
                if option < detected.len() {
                    self.config.player.name = detected[option];
                } else {
                    self.config.player.name = PlayerName::Custom;
                }
            }
            4 => {
                self.config.general.default_mode = match option {
                    0 => AudioMode::Sub,
                    _ => AudioMode::Dub,
                };
            }
            5 => self.config.general.min_quality = index_to_min_quality(option),
            _ => {}
        }
    }

    // -- Setup wizard --

    fn handle_setup_normal(&mut self, code: KeyCode) -> Option<Action> {
        let max_items = match self.setup_step {
            0 => 3, // series provider: jikan, anilist, anidb
            1 => 3, // episode provider
            2 => 3, // poster provider
            3 => {   // players: detected count + custom
                let detected = crate::player::detect_installed();
                detected.len() + 1
            }
            4 => 2, // audio mode: sub, dub
            5 => 5, // min quality: Any, 360p, 480p, 720p, 1080p
            _ => 1,
        };

        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.setup_selected = (self.setup_selected + 1).min(max_items - 1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.setup_selected = self.setup_selected.saturating_sub(1);
                None
            }
            KeyCode::Enter => {
                self.apply_setup_selection();
                self.setup_step += 1;
                self.setup_selected = 0;
                if self.setup_step >= 6 {
                    // Setup complete
                    let _ = self.config.save();
                    self.screen = Screen::Search;
                }
                None
            }
            KeyCode::Esc => {
                if self.setup_step > 0 {
                    self.setup_step -= 1;
                    self.setup_selected = 0;
                }
                None
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                None
            }
            _ => None,
        }
    }

    fn apply_setup_selection(&mut self) {
        match self.setup_step {
            0 => self.config.general.series_provider = index_to_provider(self.setup_selected),
            1 => self.config.general.episode_provider = index_to_provider(self.setup_selected),
            2 => self.config.general.poster_provider = index_to_provider(self.setup_selected),
            3 => {
                let detected = crate::player::detect_installed();
                if self.setup_selected < detected.len() {
                    self.config.player.name = detected[self.setup_selected];
                } else {
                    self.config.player.name = PlayerName::Custom;
                }
            }
            4 => {
                self.config.general.default_mode = match self.setup_selected {
                    0 => AudioMode::Sub,
                    _ => AudioMode::Dub,
                };
            }
            5 => {
                self.config.general.min_quality = index_to_min_quality(self.setup_selected);
            }
            _ => {}
        }
    }

    fn go_back(&mut self) {
        // Dismiss any active modal first
        if self.active_modal.is_some() {
            self.active_modal = None;
            self.play_error = None;
            return;
        }
        match self.screen {
            Screen::Detail => {
                self.screen = Screen::Search;
                self.selected_anime = None;
                self.episodes.clear();
                self.mal_id = None;
                self.synopsis_requested.clear();
            }
            Screen::Search => {}
            Screen::Setup => {}
        }
    }
}

fn provider_to_index(p: MetadataProvider) -> usize {
    match p {
        MetadataProvider::Jikan => 0,
        MetadataProvider::Anilist => 1,
        MetadataProvider::Anidb => 2,
    }
}

fn index_to_provider(i: usize) -> MetadataProvider {
    match i {
        0 => MetadataProvider::Jikan,
        1 => MetadataProvider::Anilist,
        _ => MetadataProvider::Anidb,
    }
}

fn min_quality_to_index(q: MinQuality) -> usize {
    match q {
        MinQuality::Any => 0,
        MinQuality::P360 => 1,
        MinQuality::P480 => 2,
        MinQuality::P720 => 3,
        MinQuality::P1080 => 4,
    }
}

fn index_to_min_quality(i: usize) -> MinQuality {
    match i {
        0 => MinQuality::Any,
        1 => MinQuality::P360,
        2 => MinQuality::P480,
        3 => MinQuality::P720,
        _ => MinQuality::P1080,
    }
}
