#[derive(Debug, Clone)]
pub struct Anime {
    pub id: String,
    pub title: String,
    pub synopsis: Option<String>,
    pub poster_url: Option<String>,
    pub episode_count: Option<u32>,
    pub genres: Vec<String>,
    pub rating: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Episode {
    pub number: f32,
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub is_filler: bool,
    pub aired: Option<String>,
}
