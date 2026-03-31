use color_eyre::Result;
use reqwest::Client;
use serde_json::Value;

use crate::model::anime::{Anime, Episode};

const JIKAN_API: &str = "https://api.jikan.moe/v4";

#[derive(Clone)]
pub struct JikanClient {
    client: Client,
}

impl JikanClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search Jikan (MyAnimeList) for anime matching the given query.
    pub async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let resp: Value = self
            .client
            .get(format!("{JIKAN_API}/anime"))
            .query(&[("q", query), ("limit", "20"), ("sfw", "true")])
            .send()
            .await?
            .json()
            .await?;

        let data = resp["data"].as_array().cloned().unwrap_or_default();
        Ok(data.iter().filter_map(parse_anime).collect())
    }

    /// Fetch detailed info for a single anime by its MAL ID.
    pub async fn detail(&self, mal_id: i64) -> Result<Anime> {
        let resp: Value = self
            .client
            .get(format!("{JIKAN_API}/anime/{mal_id}"))
            .send()
            .await?
            .json()
            .await?;

        parse_anime(&resp["data"])
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to parse Jikan response"))
    }

    /// Fetch episode details for an anime. Jikan paginates at 100 eps per page.
    pub async fn episodes(&self, mal_id: i64) -> Result<Vec<Episode>> {
        let mut all_episodes = Vec::new();
        let mut page = 1;

        loop {
            let resp: Value = self
                .client
                .get(format!("{JIKAN_API}/anime/{mal_id}/episodes"))
                .query(&[("page", &page.to_string())])
                .send()
                .await?
                .json()
                .await?;

            let data = resp["data"].as_array().cloned().unwrap_or_default();
            if data.is_empty() {
                break;
            }

            for ep in &data {
                let number = ep["mal_id"].as_f64().unwrap_or(0.0) as f32;
                let title = ep["title"].as_str().map(|s| s.to_string());
                let synopsis = ep["synopsis"].as_str().map(|s| s.to_string());
                let is_filler = ep["filler"].as_bool().unwrap_or(false);
                let aired = ep["aired"]
                    .as_str()
                    .map(|s| {
                        // Jikan returns ISO 8601 — take just the date part
                        s.split('T').next().unwrap_or(s).to_string()
                    });

                all_episodes.push(Episode {
                    number,
                    title,
                    synopsis,
                    is_filler,
                    aired,
                });
            }

            let has_next = resp["pagination"]["has_next_page"]
                .as_bool()
                .unwrap_or(false);
            if !has_next {
                break;
            }
            page += 1;

            // Respect Jikan's rate limit (3 req/sec)
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
        }

        Ok(all_episodes)
    }

    /// Fetch details for a single episode (includes synopsis).
    pub async fn episode_detail(&self, mal_id: i64, episode_number: i32) -> Result<Episode> {
        let resp: Value = self
            .client
            .get(format!("{JIKAN_API}/anime/{mal_id}/episodes/{episode_number}"))
            .send()
            .await?
            .json()
            .await?;

        let ep = &resp["data"];
        Ok(Episode {
            number: ep["mal_id"].as_f64().unwrap_or(episode_number as f64) as f32,
            title: ep["title"].as_str().map(|s| s.to_string()),
            synopsis: ep["synopsis"].as_str().map(|s| s.to_string()),
            is_filler: ep["filler"].as_bool().unwrap_or(false),
            aired: ep["aired"].as_str().map(|s| {
                s.split('T').next().unwrap_or(s).to_string()
            }),
        })
    }

    /// Download an image and return the raw bytes.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}

fn parse_anime(m: &Value) -> Option<Anime> {
    let id = m["mal_id"].as_i64()?.to_string();

    let title = m["title_english"]
        .as_str()
        .or_else(|| m["title"].as_str())?
        .to_string();

    let synopsis = m["synopsis"].as_str().map(|s| s.to_string());

    let poster_url = m["images"]["jpg"]["large_image_url"]
        .as_str()
        .or_else(|| m["images"]["jpg"]["image_url"].as_str())
        .map(|s| s.to_string());

    let episode_count = m["episodes"].as_u64().map(|n| n as u32);

    let genres = m["genres"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let rating = m["score"].as_f64().map(|n| n as f32);

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
