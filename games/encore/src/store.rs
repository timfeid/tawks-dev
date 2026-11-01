//! What each player keeps between visits: their controller bindings and
//! calibration (the activity frame has no storage of its own) and their best
//! scores. One JSON file, rewritten on every change.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use crate::room::Finished;

pub const MAX_SETTINGS_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerData {
    #[serde(default)]
    pub settings: Value,
    /// Keyed by `song:instrument:difficulty`.
    #[serde(default)]
    pub bests: HashMap<String, u64>,
}

pub struct Store {
    path: Option<PathBuf>,
    players: Mutex<HashMap<String, PlayerData>>,
}

pub fn best_key(song_id: &str, instrument: &str, difficulty: &str) -> String {
    format!("{song_id}:{instrument}:{difficulty}")
}

impl Store {
    pub fn open(dir: Option<PathBuf>) -> Store {
        let path = dir.map(|dir| {
            let _ = std::fs::create_dir_all(&dir);
            dir.join("players.json")
        });
        let players = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Store {
            path,
            players: Mutex::new(players),
        }
    }

    pub fn get(&self, user_id: &str) -> PlayerData {
        self.players
            .lock()
            .unwrap()
            .get(user_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_settings(&self, user_id: &str, settings: Value) -> Result<(), String> {
        if settings.to_string().len() > MAX_SETTINGS_BYTES {
            return Err("Those settings are too large".into());
        }
        self.players
            .lock()
            .unwrap()
            .entry(user_id.to_string())
            .or_default()
            .settings = settings;
        self.save();
        Ok(())
    }

    pub fn record(&self, finished: &[Finished]) {
        if finished.is_empty() {
            return;
        }
        {
            let mut players = self.players.lock().unwrap();
            for part in finished {
                let key = best_key(
                    &part.song_id,
                    part.part.instrument.key(),
                    part.part.difficulty.key(),
                );
                let best = players
                    .entry(part.user_id.clone())
                    .or_default()
                    .bests
                    .entry(key)
                    .or_insert(0);
                *best = (*best).max(part.score);
            }
        }
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        let json = match serde_json::to_vec(&*self.players.lock().unwrap()) {
            Ok(json) => json,
            Err(error) => return warn!(%error, "encore players did not serialize"),
        };
        let temp = path.with_extension("json.tmp");
        if let Err(error) = std::fs::write(&temp, json).and_then(|_| std::fs::rename(&temp, path)) {
            warn!(%error, "encore players did not save");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{Difficulty, Instrument};
    use crate::room::PartChoice;

    #[test]
    fn keeps_settings_and_the_best_score() {
        let dir = std::env::temp_dir().join(format!("encore-store-{}", std::process::id()));
        let store = Store::open(Some(dir.clone()));
        store
            .set_settings("u1", serde_json::json!({ "offset": 30 }))
            .unwrap();
        let part = PartChoice {
            instrument: Instrument::Bass,
            difficulty: Difficulty::Hard,
        };
        let finished = |score| Finished {
            user_id: "u1".into(),
            song_id: "s".into(),
            part,
            score,
            progress: Default::default(),
            band_size: 1,
            official: false,
        };
        store.record(&[finished(500)]);
        store.record(&[finished(300)]);

        let reopened = Store::open(Some(dir.clone()));
        let data = reopened.get("u1");
        assert_eq!(data.settings["offset"], 30);
        assert_eq!(data.bests["s:bass:hard"], 500);
        assert!(
            store
                .set_settings("u1", Value::String("x".repeat(MAX_SETTINGS_BYTES)))
                .is_err()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
