//! Where finished runs go and leaderboards come from. Scores live in the
//! tawks database behind bot-only procedures, so friends can be resolved
//! there; the frame only ever reads them through this server.

use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::room::RunRecord;

#[derive(Clone, Debug, PartialEq)]
pub struct ChartQuery {
    pub viewer: String,
    pub song_id: String,
    pub instrument: String,
    pub difficulty: String,
    pub everyone: bool,
}

#[async_trait]
pub trait ScoreBackend: Send + Sync {
    /// Keeps the runs and returns, per run, the previous and current best and the player's ranks.
    async fn record(&self, runs: &[RunRecord]) -> Result<Vec<Value>>;
    async fn chart(&self, query: &ChartQuery) -> Result<Value>;
    async fn bests(&self, user_id: &str) -> Result<Value>;
}

pub struct ApiScores {
    bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>,
}

impl ApiScores {
    pub fn new(bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>) -> Self {
        ApiScores { bot }
    }

    fn sdk(&self) -> Result<Arc<tawks_bot::TawksSdk>> {
        self.bot
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow!("Encore is still connecting to tawks"))
    }
}

#[async_trait]
impl ScoreBackend for ApiScores {
    async fn record(&self, runs: &[RunRecord]) -> Result<Vec<Value>> {
        self.sdk()?
            .web_client
            .mutate("encore_run_record", &json!({ "runs": runs }))
            .await
    }

    async fn chart(&self, query: &ChartQuery) -> Result<Value> {
        self.sdk()?
            .web_client
            .query(
                "encore_chart_scores",
                &json!({
                    "viewer": query.viewer,
                    "song_id": query.song_id,
                    "instrument": query.instrument,
                    "difficulty": query.difficulty,
                    "everyone": query.everyone,
                }),
            )
            .await
    }

    async fn bests(&self, user_id: &str) -> Result<Value> {
        self.sdk()?
            .web_client
            .query("encore_bests", &json!({ "user_id": user_id }))
            .await
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryScores {
    pub recorded: std::sync::Mutex<Vec<RunRecord>>,
    pub asked: std::sync::Mutex<Vec<ChartQuery>>,
}

#[cfg(test)]
#[async_trait]
impl ScoreBackend for MemoryScores {
    async fn record(&self, runs: &[RunRecord]) -> Result<Vec<Value>> {
        self.recorded.lock().unwrap().extend(runs.iter().cloned());
        Ok(runs
            .iter()
            .map(|run| json!({ "user_id": run.user_id, "new_best": true, "best": { "score": run.score } }))
            .collect())
    }

    async fn chart(&self, query: &ChartQuery) -> Result<Value> {
        self.asked.lock().unwrap().push(query.clone());
        Ok(json!({ "entries": [], "mine": null, "players": 0 }))
    }

    async fn bests(&self, user_id: &str) -> Result<Value> {
        let recorded = self.recorded.lock().unwrap();
        Ok(json!(
            recorded
                .iter()
                .filter(|run| run.user_id == user_id)
                .map(|run| json!({ "song_id": run.song_id, "score": run.score }))
                .collect::<Vec<_>>()
        ))
    }
}
