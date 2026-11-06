//! What each player keeps between visits: their controller bindings and
//! calibration, since the activity frame has no storage of its own. One JSON
//! file, rewritten on every change. Scores live in the tawks database.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

pub const MAX_SETTINGS_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlayerData {
    #[serde(default)]
    pub settings: Value,
}

pub struct Store {
    path: Option<PathBuf>,
    players: Mutex<HashMap<String, PlayerData>>,
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

    #[test]
    fn keeps_settings_between_visits() {
        let dir = std::env::temp_dir().join(format!("encore-store-{}", std::process::id()));
        let store = Store::open(Some(dir.clone()));
        store
            .set_settings("u1", serde_json::json!({ "offset": 30 }))
            .unwrap();
        let reopened = Store::open(Some(dir.clone()));
        assert_eq!(reopened.get("u1").settings["offset"], 30);
        assert!(
            store
                .set_settings("u1", Value::String("x".repeat(MAX_SETTINGS_BYTES)))
                .is_err()
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
