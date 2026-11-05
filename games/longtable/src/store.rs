//! Where tables and chat orders live between restarts: one JSON document per
//! key in Postgres, or in memory for tests and local runs.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};

#[async_trait]
pub trait Store: Send + Sync {
    async fn load(&self, key: &str) -> anyhow::Result<Option<Value>>;
    async fn save(&self, key: &str, state: &Value) -> anyhow::Result<()>;
}

pub fn lobby_key(lobby_id: &str) -> String {
    format!("lobby:{lobby_id}")
}

pub fn chat_key(chat_id: &str) -> String {
    format!("chat:{chat_id}")
}

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS longtable_state (
    key TEXT PRIMARY KEY,
    state JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
)";

pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub async fn connect(url: &str) -> anyhow::Result<PgStore> {
        let pool = PgPoolOptions::new().max_connections(4).connect(url).await?;
        sqlx::query(SCHEMA).execute(&pool).await?;
        Ok(PgStore { pool })
    }
}

#[async_trait]
impl Store for PgStore {
    async fn load(&self, key: &str) -> anyhow::Result<Option<Value>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT state::text FROM longtable_state WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(match row {
            Some((text,)) => Some(serde_json::from_str(&text)?),
            None => None,
        })
    }

    async fn save(&self, key: &str, state: &Value) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO longtable_state (key, state, updated_at) VALUES ($1, $2::jsonb, now())
             ON CONFLICT (key) DO UPDATE SET state = EXCLUDED.state, updated_at = now()",
        )
        .bind(key)
        .bind(state.to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, Value>>);

#[async_trait]
impl Store for MemoryStore {
    async fn load(&self, key: &str) -> anyhow::Result<Option<Value>> {
        Ok(self.0.lock().unwrap().get(key).cloned())
    }

    async fn save(&self, key: &str, state: &Value) -> anyhow::Result<()> {
        self.0.lock().unwrap().insert(key.to_string(), state.clone());
        Ok(())
    }
}
