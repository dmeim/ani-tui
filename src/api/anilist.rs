use color_eyre::Result;
use reqwest::Client;
use serde_json::Value;

use crate::model::anime::Anime;

const ANILIST_API: &str = "https://graphql.anilist.co";

const SEARCH_QUERY: &str = r#"
query ($search: String, $page: Int, $perPage: Int) {
  Page(page: $page, perPage: $perPage) {
    media(search: $search, type: ANIME, sort: POPULARITY_DESC) {
      id
      title { romaji english native }
      description(asHtml: false)
      coverImage { large extraLarge }
      episodes
      genres
      averageScore
      status
    }
  }
}
"#;

const DETAIL_QUERY: &str = r#"
query ($id: Int) {
  Media(id: $id, type: ANIME) {
    id
    title { romaji english native }
    description(asHtml: false)
    coverImage { large extraLarge }
    bannerImage
    episodes
    genres
    averageScore
    status
    season
    seasonYear
    studios(isMain: true) { nodes { name } }
    relations { edges { relationType node { id title { romaji } type } } }
  }
}
"#;

#[derive(Clone)]
pub struct AniListClient {
    client: Client,
}

impl AniListClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search AniList for anime matching the given query.
    pub async fn search(&self, query: &str) -> Result<Vec<Anime>> {
        let body = serde_json::json!({
            "query": SEARCH_QUERY,
            "variables": {
                "search": query,
                "page": 1,
                "perPage": 20
            }
        });

        let resp: Value = self
            .client
            .post(ANILIST_API)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let media = resp["data"]["Page"]["media"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        Ok(media.iter().filter_map(parse_media).collect())
    }

    /// Fetch detailed info for a single anime by its AniList ID.
    pub async fn detail(&self, id: i64) -> Result<Anime> {
        let body = serde_json::json!({
            "query": DETAIL_QUERY,
            "variables": { "id": id }
        });

        let resp: Value = self
            .client
            .post(ANILIST_API)
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        parse_media(&resp["data"]["Media"])
            .ok_or_else(|| color_eyre::eyre::eyre!("Failed to parse AniList response"))
    }

    /// Download a poster image and return the raw bytes.
    pub async fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        let bytes = self.client.get(url).send().await?.bytes().await?;
        Ok(bytes.to_vec())
    }
}

fn parse_media(m: &Value) -> Option<Anime> {
    let id = m["id"].as_i64()?.to_string();

    let title = m["title"]["english"]
        .as_str()
        .or_else(|| m["title"]["romaji"].as_str())?
        .to_string();

    let synopsis = m["description"].as_str().map(|s| {
        // AniList sometimes returns HTML-escaped text even with asHtml: false
        s.replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<i>", "")
            .replace("</i>", "")
            .replace("<b>", "")
            .replace("</b>", "")
    });

    let poster_url = m["coverImage"]["extraLarge"]
        .as_str()
        .or_else(|| m["coverImage"]["large"].as_str())
        .map(|s| s.to_string());

    let episode_count = m["episodes"].as_u64().map(|n| n as u32);

    let genres = m["genres"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let rating = m["averageScore"].as_f64().map(|n| n as f32 / 10.0);

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
