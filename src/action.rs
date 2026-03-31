use crossterm::event::KeyEvent;
use image::DynamicImage;

use crate::model::anime::{Anime, Episode};
use crate::model::stream::StreamUrl;

/// Wrapper around DynamicImage to implement Debug (image crate doesn't).
#[derive(Clone)]
pub struct DecodedImage(pub DynamicImage);

impl std::fmt::Debug for DecodedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (w, h) = (self.0.width(), self.0.height());
        write!(f, "DecodedImage({w}x{h})")
    }
}

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
    EpisodeDetailsLoaded(Vec<Episode>),
    SetMalId(i64),
    FetchEpisodeSynopsis(i64, i32),     // (mal_id, episode_number)
    EpisodeSynopsisLoaded(f32, String), // (episode_number, synopsis)

    // Playback flow
    SelectEpisode(usize),
    PlayLoading(String),
    StreamsResolved(Vec<StreamUrl>),
    Play,
    PlayError(String),

    // Poster loading
    LoadPoster(String, String), // anime_id, poster_url
    PosterLoaded(String, DecodedImage), // anime_id, decoded image

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
