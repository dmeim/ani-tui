use crossterm::event::KeyEvent;

use crate::model::anime::{Anime, Episode};
use crate::model::stream::StreamUrl;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Render,
    Quit,
    Key(KeyEvent),
    Resize(u16, u16),

    // Search flow
    Search(String),
    SearchResults(Vec<Anime>),
    SearchError(String),
    SearchLoading,

    // Detail flow
    SelectAnime(usize),
    AnimeDetail(Box<Anime>),
    EpisodesLoaded(Vec<Episode>),

    // Playback flow
    SelectEpisode(usize),
    PlayLoading(String),
    StreamsResolved(Vec<StreamUrl>),
    Play,
    PlayError(String),

    // Navigation
    Back,
    ScrollUp,
    ScrollDown,

    // Setup wizard
    SetupNext,
    SetupPrev,
    SetupSelect(usize),

    Error(String),
}
