use std::path::PathBuf;

use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub player: PlayerConfig,
    pub subtitles: SubtitleConfig,
    pub anidb: AnidbConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub series_provider: MetadataProvider,
    pub episode_provider: MetadataProvider,
    pub poster_provider: MetadataProvider,
    pub default_mode: AudioMode,
    pub min_quality: MinQuality,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayerConfig {
    pub name: PlayerName,
    pub custom_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleConfig {
    pub enabled: bool,
    pub language: String,
    pub opensubtitles_api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct AnidbConfig {
    pub client: Option<String>,
    pub client_version: Option<u32>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MetadataProvider {
    #[default]
    Jikan,
    Anilist,
    Anidb,
    Kitsu,
    Notify,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioMode {
    #[default]
    Sub,
    Dub,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerName {
    #[default]
    Mpv,
    Iina,
    Vlc,
    Quicktime,
    Custom,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MinQuality {
    /// Accept any quality
    Any,
    #[serde(rename = "360p")]
    P360,
    #[serde(rename = "480p")]
    P480,
    #[serde(rename = "720p")]
    P720,
    /// Default: prefer 1080p
    #[default]
    #[serde(rename = "1080p")]
    P1080,
}

impl MinQuality {
    /// Check if a stream quality meets this minimum.
    /// `Quality::Unknown` always passes (we can't reject streams without resolution info).
    pub fn accepts(self, quality: crate::model::stream::Quality) -> bool {
        use crate::model::stream::Quality;
        match self {
            MinQuality::Any => true,
            MinQuality::P360 => quality >= Quality::P360 || quality == Quality::Unknown,
            MinQuality::P480 => quality >= Quality::P480 || quality == Quality::Unknown,
            MinQuality::P720 => quality >= Quality::P720 || quality == Quality::Unknown,
            MinQuality::P1080 => quality >= Quality::P1080 || quality == Quality::Unknown,
        }
    }
}

// Defaults

impl Default for SubtitleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            language: "en".to_string(),
            opensubtitles_api_key: None,
        }
    }
}


impl Config {
    /// Returns the config file path: `~/.config/ani-tui/config.toml`
    pub fn path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine config directory"))?;
        Ok(config_dir.join("ani-tui").join("config.toml"))
    }

    /// Load config from disk, or return defaults if the file doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = std::fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save the current config to disk, creating the directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&path, contents)?;
        Ok(())
    }

    /// Returns true if this is a fresh config (no file on disk yet).
    pub fn needs_setup() -> Result<bool> {
        let path = Self::path()?;
        Ok(!path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: Config = toml::from_str(&serialized).unwrap();

        assert_eq!(deserialized.general.series_provider, MetadataProvider::Jikan);
        assert_eq!(deserialized.general.episode_provider, MetadataProvider::Jikan);
        assert_eq!(deserialized.general.poster_provider, MetadataProvider::Jikan);
        assert_eq!(deserialized.general.default_mode, AudioMode::Sub);
        assert_eq!(deserialized.player.name, PlayerName::Mpv);
        assert!(deserialized.player.custom_command.is_none());
        assert!(deserialized.subtitles.enabled);
        assert_eq!(deserialized.subtitles.language, "en");
    }

    #[test]
    fn deserializes_partial_config() {
        let partial = r#"
[general]
series_provider = "anidb"
episode_provider = "anilist"
"#;
        let config: Config = toml::from_str(partial).unwrap();
        assert_eq!(config.general.series_provider, MetadataProvider::Anidb);
        assert_eq!(config.general.episode_provider, MetadataProvider::Anilist);
        assert_eq!(config.general.poster_provider, MetadataProvider::Jikan); // default
        assert_eq!(config.general.default_mode, AudioMode::Sub);
        assert_eq!(config.player.name, PlayerName::Mpv);
    }

    #[test]
    fn deserializes_new_providers() {
        let partial = r#"
[general]
series_provider = "kitsu"
episode_provider = "kitsu"
poster_provider = "notify"
"#;
        let config: Config = toml::from_str(partial).unwrap();
        assert_eq!(config.general.series_provider, MetadataProvider::Kitsu);
        assert_eq!(config.general.episode_provider, MetadataProvider::Kitsu);
        assert_eq!(config.general.poster_provider, MetadataProvider::Notify);
    }

    #[test]
    fn serialized_format_matches_readme() {
        let config = Config::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(serialized.contains("series_provider = \"jikan\""));
        assert!(serialized.contains("episode_provider = \"jikan\""));
        assert!(serialized.contains("poster_provider = \"jikan\""));
        assert!(serialized.contains("default_mode = \"sub\""));
        assert!(serialized.contains("[player]"));
        assert!(serialized.contains("name = \"mpv\""));
    }
}
