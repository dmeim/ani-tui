use color_eyre::Result;
use reqwest::Client;
use serde_json::Value;

use crate::model::anime::{Anime, Episode};

const KITSU_API: &str = "https://kitsu.io/api/edge";

#[derive(Clone)]
pub struct KitsuClient {
    client: Client,
}

impl KitsuClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search Kitsu for anime matching the given query.
    pub async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let resp: Value = self
            .client
            .get(format!("{KITSU_API}/anime"))
            .query(&[
                ("filter[text]", query),
                ("page[limit]", "20"),
                ("include", "genres"),
            ])
            .header("Accept", "application/vnd.api+json")
            .send()
            .await?
            .json()
            .await?;

        let genre_map = build_genre_map(&resp);

        let data = resp["data"].as_array().cloned().unwrap_or_default();
        Ok(data
            .iter()
            .filter_map(|item| parse_anime(item, &genre_map))
            .collect())
    }

    /// Fetch detailed info for a single anime by its Kitsu ID.
    pub async fn detail(&self, id: i64) -> Result<Anime> {
        let resp: Value = self
            .client
            .get(format!("{KITSU_API}/anime/{id}"))
            .query(&[("include", "genres")])
            .header("Accept", "application/vnd.api+json")
            .send()
            .await?
            .json()
            .await?;

        let genre_map = build_genre_map(&resp);

        parse_anime(&resp["data"], &genre_map)
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to parse Kitsu response"))
    }

    /// Fetch episode details for an anime. Kitsu paginates at 20 eps per page by default.
    pub async fn episodes(&self, kitsu_id: i64) -> Result<Vec<Episode>> {
        let mut all_episodes = Vec::new();
        let mut offset = 0u32;
        let limit = 25u32;

        loop {
            let resp: Value = self
                .client
                .get(format!("{KITSU_API}/anime/{kitsu_id}/episodes"))
                .query(&[
                    ("page[limit]", &limit.to_string()),
                    ("page[offset]", &offset.to_string()),
                ])
                .header("Accept", "application/vnd.api+json")
                .send()
                .await?
                .json()
                .await?;

            let data = resp["data"].as_array().cloned().unwrap_or_default();
            if data.is_empty() {
                break;
            }

            for ep in &data {
                let attrs = &ep["attributes"];
                let number = attrs["number"].as_f64().unwrap_or(0.0) as f32;
                let title = attrs["canonicalTitle"]
                    .as_str()
                    .map(|s| s.to_string());
                let synopsis = attrs["synopsis"].as_str().map(|s| s.to_string());
                let aired = attrs["airdate"].as_str().map(|s| s.to_string());

                all_episodes.push(Episode {
                    number,
                    title,
                    synopsis,
                    is_filler: false, // Kitsu doesn't provide filler flags
                    aired,
                });
            }

            // Check if there's a next page
            let has_next = resp["links"]["next"].as_str().is_some();
            if !has_next {
                break;
            }
            offset += limit;
        }

        Ok(all_episodes)
    }

    /// Download an image and return the raw bytes.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}

/// Build a mapping of genre resource IDs to genre names from the "included" array.
fn build_genre_map(resp: &Value) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    if let Some(included) = resp["included"].as_array() {
        for item in included {
            if item["type"].as_str() == Some("genres")
                && let (Some(id), Some(name)) = (
                    item["id"].as_str(),
                    item["attributes"]["name"].as_str(),
                )
            {
                map.insert(id.to_string(), name.to_string());
            }
        }
    }
    map
}

fn parse_anime(
    item: &Value,
    genre_map: &std::collections::HashMap<String, String>,
) -> Option<Anime> {
    let id = item["id"].as_str()?.to_string();
    let attrs = &item["attributes"];

    let title = attrs["titles"]["en"]
        .as_str()
        .or_else(|| attrs["titles"]["en_us"].as_str())
        .or_else(|| attrs["titles"]["en_jp"].as_str())
        .or_else(|| attrs["canonicalTitle"].as_str())?
        .to_string();

    let synopsis = attrs["synopsis"].as_str().map(|s| s.to_string());

    let poster_url = attrs["posterImage"]["large"]
        .as_str()
        .or_else(|| attrs["posterImage"]["original"].as_str())
        .map(|s| s.to_string());

    let episode_count = attrs["episodeCount"].as_u64().map(|n| n as u32);

    // Collect genres from the relationship IDs matched against the included genre map
    let genres: Vec<String> = item["relationships"]["genres"]["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| {
                    g["id"]
                        .as_str()
                        .and_then(|id| genre_map.get(id))
                        .cloned()
                })
                .collect()
        })
        .unwrap_or_default();

    // averageRating is a string like "84.05" (out of 100), convert to 0-10 scale
    let rating = attrs["averageRating"]
        .as_str()
        .and_then(|s| s.parse::<f32>().ok())
        .map(|n| n / 10.0);

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
