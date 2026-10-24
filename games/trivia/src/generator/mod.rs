use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod local;
pub mod openai;

#[async_trait]
pub trait Generator: Send + Sync {
    async fn generate(&self, topic: &str) -> Result<GeneratedTrivia>;
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GeneratedTrivia {
    pub title: String,
    pub topic: String,
    pub description: String,
    pub questions: Vec<GeneratedTriviaQuestion>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneratedTriviaQuestion {
    pub text: String,
    pub annotation_url: Option<String>,
    pub random_fact: Option<String>,
    pub incorrect_fluff: Option<String>,
    pub duration_seconds: i32,
    pub max_points: i64,
    pub options: Vec<GeneratedTriviaOptionItem>,
    pub explanation: Option<String>,
    pub tts: Option<GeneratedTriviaTts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTriviaTts {
    pub voice_id: String,
    pub text_hash: String,
    pub audio_url: String, // local path, s3 url, etc.
    pub duration_ms: i32,
    pub reveal_plan: Value, // {"version":1,"events":[...]}
}

#[derive(Debug, Clone, Serialize, Default, Deserialize)]
pub struct GeneratedTriviaOptionItem {
    pub label: String,
    pub is_correct: bool,
}
