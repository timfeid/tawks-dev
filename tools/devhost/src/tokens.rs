//! Sandbox bot tokens. Each token is a bot of its own; the store keeps only a
//! hash of it, with the bot's name and the tawks account that made it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

/// How many sandbox bots one tawks account may have.
pub const PER_ACCOUNT: usize = 5;

#[derive(Clone, Serialize, Deserialize)]
pub struct Entry {
    hash: String,
    pub bot: String,
    pub owner_id: String,
    pub owner_name: String,
}

pub struct Tokens {
    path: PathBuf,
    entries: Mutex<Vec<Entry>>,
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Bot names become part of workflow ids, so they are short and URL-safe.
pub fn valid_name(name: &str) -> bool {
    (3..=24).contains(&name.len())
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && name.starts_with(|c: char| c.is_ascii_lowercase())
}

impl Tokens {
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let entries = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error.into()),
        };
        Ok(Tokens { path, entries: Mutex::new(entries) })
    }

    pub async fn bot_for(&self, token: &str) -> Option<String> {
        let wanted = hash(token);
        self.entries.lock().await.iter().find(|e| e.hash == wanted).map(|e| e.bot.clone())
    }

    pub async fn bots_of(&self, owner_id: &str) -> Vec<String> {
        self.entries.lock().await.iter().filter(|e| e.owner_id == owner_id).map(|e| e.bot.clone()).collect()
    }

    /// Makes a token for a new bot. The token is shown once and never stored.
    pub async fn create(&self, owner_id: &str, owner_name: &str, bot: &str) -> Result<String, String> {
        if !valid_name(bot) {
            return Err("Bot names are 3 to 24 lowercase letters, digits or underscores, starting with a letter.".into());
        }
        let mut entries = self.entries.lock().await;
        if entries.iter().any(|e| e.bot == bot) {
            return Err(format!("The name {bot} is taken. Pick another."));
        }
        if entries.iter().filter(|e| e.owner_id == owner_id).count() >= PER_ACCOUNT {
            return Err(format!("You have {PER_ACCOUNT} sandbox bots already. Delete one to make another."));
        }
        let token = format!("sbx_{}{}", ulid::Ulid::new(), ulid::Ulid::new()).to_lowercase();
        entries.push(Entry { hash: hash(&token), bot: bot.into(), owner_id: owner_id.into(), owner_name: owner_name.into() });
        self.save(&entries)?;
        Ok(token)
    }

    pub async fn delete(&self, owner_id: &str, bot: &str) -> Result<(), String> {
        let mut entries = self.entries.lock().await;
        let before = entries.len();
        entries.retain(|e| !(e.owner_id == owner_id && e.bot == bot));
        if entries.len() == before {
            return Err(format!("You have no sandbox bot called {bot}."));
        }
        self.save(&entries)
    }

    fn save(&self, entries: &[Entry]) -> Result<(), String> {
        let text = serde_json::to_string_pretty(entries).map_err(|e| e.to_string())?;
        let temp = self.path.with_extension("tmp");
        std::fs::write(&temp, text).and_then(|_| std::fs::rename(&temp, &self.path)).map_err(|e| format!("could not save tokens: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tokens_are_stored_hashed_and_limited_per_account() {
        let dir = std::env::temp_dir().join(format!("sandbox-tokens-{}", ulid::Ulid::new()));
        std::fs::create_dir_all(&dir).unwrap();
        let tokens = Tokens::load(dir.join("tokens.json")).unwrap();

        let token = tokens.create("u1", "ana", "anas_bot").await.unwrap();
        assert_eq!(tokens.bot_for(&token).await.as_deref(), Some("anas_bot"));
        assert!(!std::fs::read_to_string(dir.join("tokens.json")).unwrap().contains(&token));
        assert!(tokens.create("u2", "ben", "anas_bot").await.is_err(), "names are unique");
        assert!(tokens.create("u1", "ana", "Bad Name").await.is_err());

        for n in 1..PER_ACCOUNT {
            tokens.create("u1", "ana", &format!("ana_{n}")).await.unwrap();
        }
        assert!(tokens.create("u1", "ana", "one_too_many").await.is_err());

        let reloaded = Tokens::load(dir.join("tokens.json")).unwrap();
        assert_eq!(reloaded.bot_for(&token).await.as_deref(), Some("anas_bot"));
        tokens.delete("u1", "anas_bot").await.unwrap();
        assert_eq!(tokens.bot_for(&token).await, None);
    }
}
