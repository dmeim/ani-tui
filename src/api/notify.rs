use color_eyre::Result;
use reqwest::Client;
use serde_json::Value;

use crate::model::anime::Anime;

const NOTIFY_API: &str = "https://notify.moe/api";
const NOTIFY_SEARCH: &str = "https://notify.moe/_/anime-search";
const NOTIFY_MEDIA: &str = "https://media.notify.moe/images/anime";

#[derive(Clone)]
pub struct NotifyClient {
    client: Client,
}

impl NotifyClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search Notify.moe for anime matching the given query.
    ///
    /// Notify.moe's search returns HTML, so we parse anime IDs from the
    /// href attributes, then fetch details for each match.
    pub async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        // Percent-encode the query for use in the URL path
        let encoded_query: String = query
            .bytes()
            .flat_map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    vec![b as char]
                }
                b' ' => vec!['%', '2', '0'],
                _ => {
                    let hi = b >> 4;
                    let lo = b & 0x0f;
                    let to_hex = |n: u8| -> char {
                        if n < 10 { (b'0' + n) as char } else { (b'A' + n - 10) as char }
                    };
                    vec!['%', to_hex(hi), to_hex(lo)]
                }
            })
            .collect();
        let html = self
            .client
            .get(format!("{NOTIFY_SEARCH}/{encoded_query}"))
            .send()
            .await?
            .text()
            .await?;

        // Extract anime IDs from href='/anime/{id}' patterns
        let ids: Vec<&str> = html
            .match_indices("href='/anime/")
            .filter_map(|(i, _)| {
                let rest = &html[i + 13..]; // skip "href='/anime/"
                rest.find('\'').map(|end| &rest[..end])
            })
            .take(15) // Limit to avoid too many requests
            .collect();

        let mut results = Vec::with_capacity(ids.len());
        for id in ids {
            if let Ok(anime) = self.detail_by_id(id).await {
                results.push(anime);
            }
        }

        Ok(results)
    }

    /// Fetch detailed info for a single anime by its Notify.moe ID.
    pub async fn detail_by_id(&self, id: &str) -> Result<Anime> {
        let resp: Value = self
            .client
            .get(format!("{NOTIFY_API}/anime/{id}"))
            .send()
            .await?
            .json()
            .await?;

        parse_anime(&resp)
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to parse Notify.moe response"))
    }

    /// Download an image and return the raw bytes.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}

fn parse_anime(m: &Value) -> Option<Anime> {
    let id = m["id"].as_str()?.to_string();

    let title = m["title"]["english"]
        .as_str()
        .or_else(|| m["title"]["canonical"].as_str())
        .or_else(|| m["title"]["romaji"].as_str())?
        .to_string();

    let synopsis = m["summary"].as_str().map(|s| {
        // Notify.moe uses \r\n line breaks
        s.replace("\r\n", "\n")
    });

    // Image URL: https://media.notify.moe/images/anime/large/{id}{extension}?{lastModified}
    let poster_url = m["image"]["extension"].as_str().map(|ext| {
        let last_modified = m["image"]["lastModified"]
            .as_i64()
            .unwrap_or(0);
        format!("{NOTIFY_MEDIA}/large/{id}{ext}?{last_modified}")
    });

    let episode_count = m["episodeCount"].as_u64().map(|n| n as u32);

    let genres = m["genres"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // rating.overall is 0-10
    let rating = m["rating"]["overall"].as_f64().map(|n| n as f32);

    Some(Anime {
        id,
        title,
        synopsis,
        poster_url,
        episode_count,
        genres,
        rating,
    })
}
