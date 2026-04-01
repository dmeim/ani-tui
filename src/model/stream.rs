#[derive(Debug, Clone)]
pub struct StreamUrl {
    pub url: String,
    pub quality: Quality,
    pub provider: String,
    pub referer: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Quality {
    Unknown,
    P360,
    P480,
    P720,
    P1080,
}

impl std::fmt::Display for Quality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quality::Unknown => write!(f, "Auto"),
            Quality::P360 => write!(f, "360p"),
            Quality::P480 => write!(f, "480p"),
            Quality::P720 => write!(f, "720p"),
            Quality::P1080 => write!(f, "1080p"),
        }
    }
}
