#[derive(Debug, Clone)]
pub struct StreamUrl {
    pub url: String,
    pub quality: Quality,
    pub provider: String,
    pub referer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Quality {
    P360,
    P480,
    P720,
    P1080,
    Unknown,
}
