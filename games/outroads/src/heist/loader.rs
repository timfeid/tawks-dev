//! Fetches the vector tiles the referee needs, a few at a time with a cache
//! shared by every lobby, and adds them to a game's map as they arrive.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use tokio::sync::{Mutex as AsyncMutex, Semaphore};

use super::geo::Frame;
use super::map::Map;
use super::mvt;

const TILEJSON: &str = "https://tiles.openfreemap.org/planet";
const LAYERS: &[&str] = &["transportation", "transportation_name", "building"];
const KEEP: usize = 400;

/// Raw tiles shared by every game on this server.
struct Shared {
    http: reqwest::Client,
    template: AsyncMutex<Option<String>>,
    cache: Mutex<(HashMap<(u32, u32), Arc<Vec<u8>>>, VecDeque<(u32, u32)>)>,
    slots: Semaphore,
}

fn shared() -> &'static Shared {
    static SHARED: OnceLock<Shared> = OnceLock::new();
    SHARED.get_or_init(|| Shared {
        http: reqwest::Client::builder()
            .user_agent("tawks-outroads/0.1 (+https://tawks.app; heist referee)")
            .timeout(Duration::from_secs(15))
            .build()
            .expect("http client"),
        template: AsyncMutex::new(None),
        cache: Mutex::new((HashMap::new(), VecDeque::new())),
        slots: Semaphore::new(4),
    })
}

async fn template() -> Option<String> {
    if let Ok(url) = std::env::var("OUTROADS_TILES_URL")
        && !url.is_empty()
    {
        return Some(url);
    }
    let s = shared();
    let mut t = s.template.lock().await;
    if t.is_none() {
        let json: serde_json::Value = s.http.get(TILEJSON).send().await.ok()?.json().await.ok()?;
        *t = json["tiles"][0].as_str().map(str::to_string);
    }
    t.clone()
}

async fn fetch(x: u32, y: u32) -> Option<Arc<Vec<u8>>> {
    let s = shared();
    if let Some(hit) = s.cache.lock().unwrap().0.get(&(x, y)).cloned() {
        return Some(hit);
    }
    let _slot = s.slots.acquire().await.ok()?;
    let url = template()
        .await?
        .replace("{z}", "14")
        .replace("{x}", &x.to_string())
        .replace("{y}", &y.to_string());
    let mut body = None;
    for attempt in 0..3u64 {
        match s.http.get(&url).send().await {
            Ok(r) if r.status().is_success() => {
                body = r.bytes().await.ok().map(|b| Arc::new(b.to_vec()));
                break;
            }
            Ok(r) if r.status().as_u16() == 404 => {
                body = Some(Arc::new(Vec::new()));
                break;
            }
            _ => tokio::time::sleep(Duration::from_millis(600 * (attempt + 1))).await,
        }
    }
    let body = body?;
    let mut cache = s.cache.lock().unwrap();
    if cache.0.insert((x, y), body.clone()).is_none() {
        cache.1.push_back((x, y));
    }
    while cache.1.len() > KEEP {
        if let Some(old) = cache.1.pop_front() {
            cache.0.remove(&old);
        }
    }
    Some(body)
}

/// One game's map, filled in the background.
#[derive(Clone)]
pub struct Loader {
    pub frame: Frame,
    pub map: Arc<Mutex<Map>>,
    asked: Arc<Mutex<HashSet<(u32, u32)>>>,
    offline: bool,
}

impl Loader {
    pub fn new(frame: Frame) -> Loader {
        Loader {
            frame,
            map: Arc::new(Mutex::new(Map::default())),
            asked: Arc::new(Mutex::new(HashSet::new())),
            offline: false,
        }
    }

    /// A loader over a map already in hand, that never fetches.
    pub fn offline(frame: Frame, map: Map) -> Loader {
        Loader {
            frame,
            map: Arc::new(Mutex::new(map)),
            asked: Arc::new(Mutex::new(HashSet::new())),
            offline: true,
        }
    }

    /// Asks for the tile under a world point and, with `ring`, its neighbours.
    pub fn want_around(&self, x: f64, z: f64, ring: i64) {
        let (tx, ty) = self.frame.tile_at(x, z);
        for dy in -ring..=ring {
            for dx in -ring..=ring {
                self.want((tx as i64 + dx) as u32, (ty as i64 + dy) as u32);
            }
        }
    }

    /// Every tile touching a circle.
    pub fn want_circle(&self, x: f64, z: f64, r: f64) {
        let size = self.frame.meters_per_tile;
        let (ax, az) = self.frame.tile_at(x - r, z - r);
        let (bx, bz) = self.frame.tile_at(x + r, z + r);
        let _ = size;
        for ty in az..=bz {
            for tx in ax..=bx {
                self.want(tx, ty);
            }
        }
    }

    pub fn want(&self, x: u32, y: u32) {
        if self.offline || !self.asked.lock().unwrap().insert((x, y)) {
            return;
        }
        let me = self.clone();
        tokio::spawn(async move {
            let Some(body) = fetch(x, y).await else {
                me.asked.lock().unwrap().remove(&(x, y));
                return;
            };
            let frame = me.frame;
            let map = me.map.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let layers = mvt::decode(&body, LAYERS);
                map.lock().unwrap().add_tile(&frame, x, y, &layers);
            })
            .await;
        });
    }

    /// Whether every tile asked for so far has arrived.
    pub fn settled(&self) -> bool {
        let asked = self.asked.lock().unwrap();
        let map = self.map.lock().unwrap();
        asked.iter().all(|t| map.tiles.contains(t))
    }
}
