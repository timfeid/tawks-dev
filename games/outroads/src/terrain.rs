//! Elevation tiles for the client, passed through from the Terrain Tiles on
//! AWS Open Data. The bucket sends no CORS headers, and activity frames have
//! no origin, so the game fetches them here; a small cache keeps repeat
//! drives in the same place off the bucket.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

const DEFAULT_URL: &str = "https://s3.amazonaws.com/elevation-tiles-prod/terrarium";
const KEEP: usize = 400;
/// The client only asks for zoom 13; a little room either side for later.
const ZOOMS: std::ops::RangeInclusive<u8> = 10..=14;

pub struct TerrainProxy {
    url: String,
    http: reqwest::Client,
    cache: Mutex<(HashMap<(u8, u32, u32), Bytes>, VecDeque<(u8, u32, u32)>)>,
}

/// Reads `{z}/{x}/{y}.png` into tile coordinates, refusing anything outside the world.
pub fn parse_tile(z: &str, x: &str, file: &str) -> Option<(u8, u32, u32)> {
    let z: u8 = z.parse().ok()?;
    if !ZOOMS.contains(&z) {
        return None;
    }
    let x: u32 = x.parse().ok()?;
    let y: u32 = file.strip_suffix(".png")?.parse().ok()?;
    let side = 1u32 << z;
    (x < side && y < side).then_some((z, x, y))
}

impl TerrainProxy {
    pub fn from_env() -> TerrainProxy {
        let url = std::env::var("OUTROADS_TERRAIN_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_URL.to_string());
        TerrainProxy {
            url: url.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .user_agent("tawks-outroads/0.1 (+https://tawks.app)")
                .timeout(Duration::from_secs(10))
                .build()
                .expect("http client"),
            cache: Mutex::new((HashMap::new(), VecDeque::new())),
        }
    }

    async fn tile(&self, key: (u8, u32, u32)) -> Result<Bytes, StatusCode> {
        if let Some(hit) = self.cache.lock().unwrap().0.get(&key).cloned() {
            return Ok(hit);
        }
        let (z, x, y) = key;
        let response = self
            .http
            .get(format!("{}/{z}/{x}/{y}.png", self.url))
            .send()
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        if !response.status().is_success() {
            return Err(if response.status() == reqwest::StatusCode::NOT_FOUND {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_GATEWAY
            });
        }
        let body = response.bytes().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
        let mut cache = self.cache.lock().unwrap();
        if cache.0.insert(key, body.clone()).is_none() {
            cache.1.push_back(key);
        }
        while cache.1.len() > KEEP {
            if let Some(old) = cache.1.pop_front() {
                cache.0.remove(&old);
            }
        }
        Ok(body)
    }
}

pub async fn handler(
    State(server): State<crate::OutroadsServer>,
    Path((z, x, file)): Path<(String, String, String)>,
) -> Response {
    let Some(key) = parse_tile(&z, &x, &file) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match server.terrain().tile(key).await {
        Ok(body) => {
            let mut response = body.into_response();
            let headers = response.headers_mut();
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("image/png"));
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=2592000, immutable"),
            );
            response
        }
        Err(status) => {
            let mut response = status.into_response();
            response
                .headers_mut()
                .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
            response
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_tiles_are_asked_for() {
        assert_eq!(parse_tile("13", "1309", "3166.png"), Some((13, 1309, 3166)));
        assert_eq!(parse_tile("13", "8192", "1.png"), None);
        assert_eq!(parse_tile("13", "1", "1.jpg"), None);
        assert_eq!(parse_tile("20", "1", "1.png"), None);
        assert_eq!(parse_tile("13", "-1", "1.png"), None);
        assert_eq!(parse_tile("13", "1", "../x.png"), None);
    }
}
