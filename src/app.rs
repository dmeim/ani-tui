use crossterm::event::{KeyCode, KeyModifiers};

use crate::action::Action;
use crate::config::{AudioMode, Config, MetadataProvider, PlayerName};
use crate::model::anime::{Anime, Episode};
use crate::model::stream::StreamUrl;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Search,
    Detail,
    Playing,
    Setup,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
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

    // Playing state
    pub streams: Vec<StreamUrl>,
    pub now_playing_title: Option<String>,
    pub now_playing_episode: Option<String>,
    pub play_error: Option<String>,

    // Setup wizard state
    pub setup_step: usize,
    pub setup_selected: usize,

    // General errors
    pub error_message: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let needs_setup = Config::needs_setup().unwrap_or(false);
        Self {
            should_quit: false,
            screen: if needs_setup { Screen::Setup } else { Screen::Search },
            input_mode: InputMode::Normal,
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
            streams: Vec::new(),
            now_playing_title: None,
            now_playing_episode: None,
            play_error: None,
            setup_step: 0,
            setup_selected: 0,
            error_message: None,
        }
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
                None
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
                None
            }
            Action::EpisodesLoaded(eps) => {
                self.episodes = eps;
                None
            }

            // Playback
            Action::PlayLoading(info) => {
                self.screen = Screen::Playing;
                self.play_error = None;
                self.now_playing_title = Some(info);
                None
            }
            Action::StreamsResolved(streams) => {
                self.streams = streams;
                self.screen = Screen::Playing;
                self.play_error = None;
                if let Some(ref anime) = self.selected_anime {
                    self.now_playing_title = Some(anime.title.clone());
                }
                if let Some(ep) = self.episodes.get(self.selected_episode) {
                    self.now_playing_episode = Some(format!("{}", ep.number));
                }
                None
            }
            Action::PlayError(e) => {
                self.play_error = Some(e);
                self.screen = Screen::Playing;
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

        match self.input_mode {
            InputMode::Editing => self.handle_editing_key(code),
            InputMode::Normal => match self.screen {
                Screen::Search => self.handle_search_normal(code),
                Screen::Detail => self.handle_detail_normal(code),
                Screen::Playing => self.handle_playing_normal(code),
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
            KeyCode::Char('/') | KeyCode::Char('i') => {
                self.input_mode = InputMode::Editing;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.search_results.is_empty() {
                    self.selected_result = (self.selected_result + 1).min(self.search_results.len() - 1);
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_result = self.selected_result.saturating_sub(1);
                None
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
                self.input_mode = InputMode::Normal;
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
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                self.go_back();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.episodes.is_empty() {
                    self.selected_episode = (self.selected_episode + 1).min(self.episodes.len() - 1);
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_episode = self.selected_episode.saturating_sub(1);
                None
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

    // -- Playing screen --

    fn handle_playing_normal(&mut self, code: KeyCode) -> Option<Action> {
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
                None
            }
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
                self.go_back();
                None
            }
            KeyCode::Char('n') | KeyCode::Right => {
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

    // -- Setup wizard --

    fn handle_setup_normal(&mut self, code: KeyCode) -> Option<Action> {
        let max_items = match self.setup_step {
            0 => 2, // metadata provider: anilist, anidb
            1 => {   // players: detected count + custom
                let detected = crate::player::detect_installed();
                detected.len() + 1
            }
            2 => 2, // audio mode: sub, dub
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
                if self.setup_step >= 3 {
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
            0 => {
                self.config.general.metadata_provider = match self.setup_selected {
                    0 => MetadataProvider::Anilist,
                    _ => MetadataProvider::Anidb,
                };
            }
            1 => {
                let detected = crate::player::detect_installed();
                if self.setup_selected < detected.len() {
                    self.config.player.name = detected[self.setup_selected];
                } else {
                    self.config.player.name = PlayerName::Custom;
                }
            }
            2 => {
                self.config.general.default_mode = match self.setup_selected {
                    0 => AudioMode::Sub,
                    _ => AudioMode::Dub,
                };
            }
            _ => {}
        }
    }

    fn go_back(&mut self) {
        match self.screen {
            Screen::Detail => {
                self.screen = Screen::Search;
                self.selected_anime = None;
                self.episodes.clear();
            }
            Screen::Playing => {
                self.screen = Screen::Detail;
                self.play_error = None;
            }
            Screen::Search => {}
            Screen::Setup => {}
        }
    }
}
