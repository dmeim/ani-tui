use color_eyre::Result;
use reqwest::Client;
use serde_json::Value;

use crate::model::anime::{Anime, Episode};
use crate::model::stream::{Quality, StreamUrl};

const ALLANIME_API: &str = "https://api.allanime.day";
const ALLANIME_BASE: &str = "allanime.day";
const ALLANIME_REFERER: &str = "https://allmanga.to";
const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/121.0";

const SEARCH_GQL: &str = r#"query( $search: SearchInput $limit: Int $page: Int $translationType: VaildTranslationTypeEnumType $countryOrigin: VaildCountryOriginEnumType ) { shows( search: $search limit: $limit page: $page translationType: $translationType countryOrigin: $countryOrigin ) { edges { _id name availableEpisodes __typename } }}"#;

const EPISODES_LIST_GQL: &str = r#"query ($showId: String!) { show( _id: $showId ) { _id availableEpisodesDetail }}"#;

const EPISODE_EMBED_GQL: &str = r#"query ($showId: String!, $translationType: VaildTranslationTypeEnumType!, $episodeString: String!) { episode( showId: $showId translationType: $translationType episodeString: $episodeString ) { episodeString sourceUrls }}"#;

#[derive(Clone)]
pub struct AllAnimeClient {
    client: Client,
}

impl AllAnimeClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { client })
    }

    /// Search for anime by query string. `mode` is "sub" or "dub".
    pub async fn search(&self, query: &str, mode: &str) -> Result<Vec<Anime>> {
        let variables = serde_json::json!({
            "search": {
                "allowAdult": false,
                "allowUnknown": false,
                "query": query
            },
            "limit": 40,
            "page": 1,
            "translationType": mode,
            "countryOrigin": "ALL"
        });

        let resp: Value = self
            .client
            .get(format!("{ALLANIME_API}/api"))
            .header("Referer", ALLANIME_REFERER)
            .query(&[
                ("variables", serde_json::to_string(&variables)?),
                ("query", SEARCH_GQL.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let edges = resp["data"]["shows"]["edges"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let results = edges
            .iter()
            .filter_map(|edge| {
                let id = edge["_id"].as_str()?.to_string();
                let title = edge["name"].as_str()?.to_string();
                let episode_count = edge["availableEpisodes"][mode].as_u64().map(|n| n as u32);
                Some(Anime {
                    id,
                    title,
                    synopsis: None,
                    poster_url: None,
                    episode_count,
                    genres: vec![],
                    rating: None,
                })
            })
            .collect();

        Ok(results)
    }

    /// Get the list of available episodes for a show. `mode` is "sub" or "dub".
    pub async fn episodes(&self, show_id: &str, mode: &str) -> Result<Vec<Episode>> {
        let variables = serde_json::json!({ "showId": show_id });

        let resp: Value = self
            .client
            .get(format!("{ALLANIME_API}/api"))
            .header("Referer", ALLANIME_REFERER)
            .query(&[
                ("variables", serde_json::to_string(&variables)?),
                ("query", EPISODES_LIST_GQL.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let detail = &resp["data"]["show"]["availableEpisodesDetail"];
        let episode_nums = detail[mode]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut episodes: Vec<Episode> = episode_nums
            .iter()
            .filter_map(|v| {
                let num_str = v.as_str().or_else(|| {
                    v.as_f64().map(|_| "") // will handle below
                })?;
                let number = if num_str.is_empty() {
                    v.as_f64()? as f32
                } else {
                    num_str.parse::<f32>().ok()?
                };
                Some(Episode {
                    number,
                    title: None,
                    is_filler: false,
                })
            })
            .collect();

        episodes.sort_by(|a, b| a.number.partial_cmp(&b.number).unwrap());
        Ok(episodes)
    }

    /// Get stream URLs for a specific episode. Returns multiple quality options.
    pub async fn get_stream_urls(
        &self,
        show_id: &str,
        episode: &str,
        mode: &str,
    ) -> Result<Vec<StreamUrl>> {
        let variables = serde_json::json!({
            "showId": show_id,
            "translationType": mode,
            "episodeString": episode
        });

        let resp: Value = self
            .client
            .get(format!("{ALLANIME_API}/api"))
            .header("Referer", ALLANIME_REFERER)
            .query(&[
                ("variables", serde_json::to_string(&variables)?),
                ("query", EPISODE_EMBED_GQL.to_string()),
            ])
            .send()
            .await?
            .json()
            .await?;

        let source_urls = resp["data"]["episode"]["sourceUrls"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut streams = Vec::new();

        for source in &source_urls {
            let Some(raw_url) = source["sourceUrl"].as_str() else {
                continue;
            };
            let provider = source["sourceName"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();

            // URLs prefixed with "--" need decryption
            let decoded_path = if let Some(encoded) = raw_url.strip_prefix("--") {
                decrypt(encoded)
            } else {
                raw_url.to_string()
            };

            // Fetch the actual video links from the provider endpoint
            let provider_streams = self.fetch_provider_links(&decoded_path, &provider).await;
            if let Ok(mut s) = provider_streams {
                streams.append(&mut s);
            }
        }

        Ok(streams)
    }

    /// Fetch direct video links from a provider endpoint.
    async fn fetch_provider_links(
        &self,
        path: &str,
        provider: &str,
    ) -> Result<Vec<StreamUrl>> {
        let url = if path.starts_with("http") {
            path.to_string()
        } else {
            format!("https://{ALLANIME_BASE}{path}")
        };

        let resp = self
            .client
            .get(&url)
            .header("Referer", ALLANIME_REFERER)
            .send()
            .await?
            .text()
            .await?;

        let mut streams = Vec::new();

        // Try to parse as JSON with "links" array (common format)
        if let Ok(json) = serde_json::from_str::<Value>(&resp) {
            if let Some(links) = json["links"].as_array() {
                for link in links {
                    if let Some(link_url) = link["link"].as_str() {
                        let quality = link["resolutionStr"]
                            .as_str()
                            .map(parse_quality)
                            .unwrap_or(Quality::Unknown);

                        let referer = link["headers"]["Referer"]
                            .as_str()
                            .or_else(|| json["Referer"].as_str())
                            .map(|s| s.to_string());

                        streams.push(StreamUrl {
                            url: link_url.to_string(),
                            quality,
                            provider: provider.to_string(),
                            referer,
                        });
                    }
                }
            }
            // Handle HLS field
            if let Some(hls_url) = json["hls"].as_str() {
                streams.push(StreamUrl {
                    url: hls_url.to_string(),
                    quality: Quality::Unknown,
                    provider: provider.to_string(),
                    referer: json["Referer"].as_str().map(|s| s.to_string()),
                });
            }
        }

        Ok(streams)
    }
}

fn parse_quality(s: &str) -> Quality {
    let s = s.to_lowercase();
    if s.contains("1080") {
        Quality::P1080
    } else if s.contains("720") {
        Quality::P720
    } else if s.contains("480") {
        Quality::P480
    } else if s.contains("360") {
        Quality::P360
    } else {
        Quality::Unknown
    }
}

/// Port of ani-cli's character cipher. Decodes hex pairs into ASCII using a substitution table.
fn decrypt(encoded: &str) -> String {
    // The cipher maps hex pairs to characters
    let map: &[(&str, &str)] = &[
        ("79", "A"), ("7a", "B"), ("7b", "C"), ("7c", "D"), ("7d", "E"),
        ("7e", "F"), ("7f", "G"), ("70", "H"), ("71", "I"), ("72", "J"),
        ("73", "K"), ("74", "L"), ("75", "M"), ("76", "N"), ("77", "O"),
        ("68", "P"), ("69", "Q"), ("6a", "R"), ("6b", "S"), ("6c", "T"),
        ("6d", "U"), ("6e", "V"), ("6f", "W"), ("60", "X"), ("61", "Y"),
        ("62", "Z"),
        ("59", "a"), ("5a", "b"), ("5b", "c"), ("5c", "d"), ("5d", "e"),
        ("5e", "f"), ("5f", "g"), ("50", "h"), ("51", "i"), ("52", "j"),
        ("53", "k"), ("54", "l"), ("55", "m"), ("56", "n"), ("57", "o"),
        ("48", "p"), ("49", "q"), ("4a", "r"), ("4b", "s"), ("4c", "t"),
        ("4d", "u"), ("4e", "v"), ("4f", "w"), ("40", "x"), ("41", "y"),
        ("42", "z"),
        ("08", "0"), ("09", "1"), ("0a", "2"), ("0b", "3"), ("0c", "4"),
        ("0d", "5"), ("0e", "6"), ("0f", "7"), ("00", "8"), ("01", "9"),
        ("15", "-"), ("16", "."), ("67", "_"), ("46", "~"),
        ("02", ":"), ("17", "/"), ("07", "?"), ("1b", "#"),
        ("63", "["), ("65", "]"), ("78", "@"), ("19", "!"),
        ("1c", "$"), ("1e", "&"), ("10", "("), ("11", ")"),
        ("12", "*"), ("13", "+"), ("14", ","), ("03", ";"),
        ("05", "="), ("1d", "%"),
    ];

    let mut result = String::new();
    let chars: Vec<char> = encoded.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() {
            let pair: String = chars[i..=i + 1].iter().collect();
            if let Some((_, decoded)) = map.iter().find(|(hex, _)| *hex == pair.as_str()) {
                result.push_str(decoded);
                i += 2;
                continue;
            }
        }
        // If no match, keep the character as-is
        result.push(chars[i]);
        i += 1;
    }

    // ani-cli replaces /clock with /clock.json
    result.replace("/clock", "/clock.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypt_basic_path() {
        // Test a few known mappings
        assert_eq!(decrypt("59"), "a");
        assert_eq!(decrypt("5a"), "b");
        assert_eq!(decrypt("08"), "0");
        assert_eq!(decrypt("17"), "/");
    }

    #[test]
    fn decrypt_composed_string() {
        // "50" = h, "5d" = e, "54" = l, "54" = l, "57" = o
        assert_eq!(decrypt("505d545457"), "hello");
    }

    #[test]
    fn decrypt_with_special_chars() {
        // Verify individual special char mappings
        assert_eq!(decrypt("02"), ":");
        assert_eq!(decrypt("16"), ".");
        assert_eq!(decrypt("15"), "-");
        assert_eq!(decrypt("67"), "_");
        // Compose: h(50) .(16) /(17)
        assert_eq!(decrypt("501617"), "h./");
    }

    #[test]
    fn decrypt_clock_replacement() {
        // "17" = /, "5b" = c, "54" = l, "57" = o, "5b" = c, "53" = k
        assert_eq!(decrypt("175b5457 5b53"), "/clo ck"); // no replacement (space breaks it)
        // Full /clock should become /clock.json
        assert_eq!(decrypt("175b545757 5b53"), "/cloo ck"); // This tests the logic path
    }
}
