//! Place search through Photon (photon.komoot.io, OpenStreetMap data). Every
//! lobby shares one paced, cached client, so players never hit Photon
//! themselves and the server stays well inside its fair use.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;

const DEFAULT_URL: &str = "https://photon.komoot.io/api/";
const USER_AGENT: &str = "tawks-outroads/0.1 (+https://tawks.app; lobby driving game)";
const SPACING: Duration = Duration::from_millis(1100);
const CACHE_FOR: Duration = Duration::from_secs(24 * 60 * 60);
const CACHE_ENTRIES: usize = 2_000;
const MAX_RESULTS: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Found {
    pub name: String,
    /// The town, region and country, for telling places apart.
    pub label: String,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(default)]
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    geometry: Geometry,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: Vec<f64>,
}

/// Reads a Photon answer into places, dropping any without a name or point.
pub fn parse(body: &str) -> Vec<Found> {
    let Ok(collection) = serde_json::from_str::<Collection>(body) else {
        return Vec::new();
    };
    let text = |f: &Feature, key: &str| f.properties.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let mut out: Vec<Found> = Vec::new();
    for f in &collection.features {
        let [lon, lat] = f.geometry.coordinates[..] else {
            continue;
        };
        let Some(name) = text(f, "name").or_else(|| {
            let street = text(f, "street")?;
            Some(match text(f, "housenumber") {
                Some(n) => format!("{n} {street}"),
                None => street,
            })
        }) else {
            continue;
        };
        let mut parts: Vec<String> = Vec::new();
        for key in ["city", "county", "state", "country"] {
            if let Some(part) = text(f, key)
                && part != name
                && !parts.contains(&part)
            {
                parts.push(part);
            }
        }
        let found = Found {
            name,
            label: parts.join(", "),
            lat,
            lon,
        };
        if !out.iter().any(|o| o.name == found.name && o.label == found.label) {
            out.push(found);
        }
        if out.len() >= MAX_RESULTS {
            break;
        }
    }
    out
}

/// The cache key: case and spacing do not make a new search, and a bias
/// point only counts to about ten kilometers.
pub fn cache_key(query: &str, near: Option<(f64, f64)>) -> String {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    match near {
        Some((lat, lon)) => format!("{}@{:.1},{:.1}", words.join(" "), lat, lon),
        None => words.join(" "),
    }
}

pub struct Geocoder {
    url: String,
    http: reqwest::Client,
    cache: Mutex<HashMap<String, (Instant, Vec<Found>)>>,
    next_slot: AsyncMutex<Instant>,
}

impl Geocoder {
    pub fn from_env() -> Geocoder {
        let url = std::env::var("OUTROADS_GEOCODER_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_URL.to_string());
        Geocoder::new(url)
    }

    pub fn new(url: String) -> Geocoder {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(8))
            .build()
            .expect("http client");
        Geocoder {
            url,
            http,
            cache: Mutex::new(HashMap::new()),
            next_slot: AsyncMutex::new(Instant::now()),
        }
    }

    fn cached(&self, key: &str) -> Option<Vec<Found>> {
        let cache = self.cache.lock().unwrap();
        cache
            .get(key)
            .filter(|(at, _)| at.elapsed() < CACHE_FOR)
            .map(|(_, found)| found.clone())
    }

    fn remember(&self, key: String, found: Vec<Found>) {
        let mut cache = self.cache.lock().unwrap();
        if cache.len() >= CACHE_ENTRIES {
            cache.retain(|_, (at, _)| at.elapsed() < CACHE_FOR);
            if cache.len() >= CACHE_ENTRIES
                && let Some(oldest) = cache.iter().min_by_key(|(_, (at, _))| *at).map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), found));
    }

    pub async fn search(&self, query: &str, near: Option<(f64, f64)>) -> Result<Vec<Found>, &'static str> {
        let query = query.trim();
        if query.chars().count() < 2 || query.len() > 200 {
            return Ok(Vec::new());
        }
        let key = cache_key(query, near);
        if let Some(found) = self.cached(&key) {
            return Ok(found);
        }
        {
            let mut slot = self.next_slot.lock().await;
            let wait = slot.saturating_duration_since(Instant::now());
            if wait > Duration::from_secs(10) {
                return Err("Search is busy. Try again in a moment.");
            }
            tokio::time::sleep(wait).await;
            *slot = Instant::now() + SPACING;
        }
        if let Some(found) = self.cached(&key) {
            return Ok(found);
        }
        let mut params: Vec<(&str, String)> = vec![("q", query.to_string()), ("limit", "8".into())];
        if let Some((lat, lon)) = near {
            params.push(("lat", format!("{lat:.3}")));
            params.push(("lon", format!("{lon:.3}")));
        }
        let response = self
            .http
            .get(&self.url)
            .query(&params)
            .send()
            .await
            .map_err(|_| "Search did not answer. Try again.")?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "place search refused a query");
            return Err("Search did not answer. Try again.");
        }
        let body = response.text().await.map_err(|_| "Search did not answer. Try again.")?;
        let found = parse(&body);
        self.remember(key, found.clone());
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn photon_answers_become_named_places() {
        let body = r#"{"type":"FeatureCollection","features":[
            {"geometry":{"coordinates":[-74.047455,40.7215682]},"properties":{"name":"Jersey City","county":"Hudson","state":"New Jersey","country":"United States"}},
            {"geometry":{"coordinates":[-74.0,40.7]},"properties":{"street":"Grand Street","housenumber":"10","city":"Jersey City","country":"United States"}},
            {"geometry":{"coordinates":[1.0]},"properties":{"name":"Broken"}},
            {"geometry":{"coordinates":[1.0,2.0]},"properties":{}},
            {"geometry":{"coordinates":[-74.047455,40.7215682]},"properties":{"name":"Jersey City","county":"Hudson","state":"New Jersey","country":"United States"}}
        ]}"#;
        let found = parse(body);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].label, "Hudson, New Jersey, United States");
        assert_eq!((found[0].lat, found[0].lon), (40.7215682, -74.047455));
        assert_eq!(found[1].name, "10 Grand Street");
        assert!(parse("not json").is_empty());
    }

    #[test]
    fn searches_that_differ_only_in_case_or_spacing_share_a_cache_entry() {
        assert_eq!(cache_key(" Times  Square ", None), cache_key("times square", None));
        assert_eq!(
            cache_key("cafe", Some((40.712, -74.006))),
            cache_key("Cafe", Some((40.74, -74.04)))
        );
    }
}
