// src/trivia/generator/openai.rs

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use crate::Generator;
use crate::generator::GeneratedTrivia;

pub struct OpenAiGenerator {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAiGenerator {
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let client = Client::builder()
            .timeout(Duration::from_secs(45))
            .build()
            .context("build reqwest client")?;

        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

#[async_trait]
impl Generator for OpenAiGenerator {
    async fn generate(&self, topic: &str) -> Result<GeneratedTrivia> {
        let topic = topic.trim();
        if topic.is_empty() {
            return Err(anyhow!("topic is empty"));
        }

        let system = r#"
You are a trivia quiz generator.
Return ONLY valid JSON (no markdown, no commentary).
Make questions medium difficulty, factual, and unambiguous.
Avoid subjective opinions and trick wording.
"#;

        let user = format!(
            r#"Generate a 5-question multiple-choice trivia quiz about: "{}".
Requirements:
- Minimum 10 questions.
- Each question has exactly 4 options.
- Exactly 1 option is correct.
- duration_seconds should be 15-30.
- max_points must be >= 100.
- Optional fields may be null (annotation_url, random_fact, incorrect_fluff, explanation).
Return ONLY JSON.
"#,
            topic
        );

        // IMPORTANT: With strict schema, OpenAI requires that `required` includes EVERY key in `properties`.
        // "Optional" fields must be represented by allowing `null` in their type, not by omitting them.
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "title": { "type": "string" },
                "topic": { "type": "string" },
                "description": { "type": "string" },
                "questions": {
                    "type": "array",
                    "minItems": 10,
                    "maxItems": 15,
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "text": { "type": "string" },
                            "annotation_url": { "type": ["string", "null"] },
                            "random_fact": { "type": ["string", "null"] },
                            "incorrect_fluff": { "type": ["string", "null"] },
                            "duration_seconds": { "type": "integer", "minimum": 5, "maximum": 120 },
                            "max_points": { "type": "integer", "minimum": 100 },
                            "options": {
                                "type": "array",
                                "minItems": 4,
                                "maxItems": 4,
                                "items": {
                                    "type": "object",
                                    "additionalProperties": false,
                                    "properties": {
                                        "label": { "type": "string" },
                                        "is_correct": { "type": "boolean" }
                                    },
                                    "required": ["label", "is_correct"]
                                }
                            },
                            "explanation": { "type": ["string", "null"] }
                        },
                        "required": [
                            "text",
                            "annotation_url",
                            "random_fact",
                            "incorrect_fluff",
                            "duration_seconds",
                            "max_points",
                            "options",
                            "explanation"
                        ]
                    }
                }
            },
            "required": ["title", "topic", "description", "questions"]
        });

        let body = json!({
            "model": self.model,
            "input": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "temperature": 0.7,
            "max_output_tokens": 1200,
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": "generated_trivia",
                    "strict": true,
                    "schema": schema
                }
            }
        });

        let resp = self
            .client
            .post("https://api.openai.com/v1/responses")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("request to OpenAI failed")?;

        let status = resp.status();
        let raw: serde_json::Value = resp.json().await.context("parse OpenAI response")?;

        if !status.is_success() {
            let msg = raw
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            return Err(anyhow!("OpenAI API error ({}): {}", status, msg));
        }

        let content = extract_output_text(&raw)
            .ok_or_else(|| anyhow!("missing model output text in Responses API payload"))?;

        let mut quiz: GeneratedTrivia = serde_json::from_str(&content)
            .map_err(|e| anyhow!("model did not return valid JSON: {e}\nraw:\n{content}"))?;

        // Sanitize + validate app invariants (beyond schema)
        sanitize_quiz(&mut quiz, topic);
        validate_quiz(&quiz)?;

        Ok(quiz)
    }
}

fn extract_output_text(raw: &serde_json::Value) -> Option<String> {
    // Typical Responses API shape:
    // { "output":[ { "type":"message", "role":"assistant", "content":[{"type":"output_text","text":"..."}]} ] }
    let output = raw.get("output")?.as_array()?;
    for item in output {
        if item.get("type")?.as_str()? != "message" {
            continue;
        }
        let content = item.get("content")?.as_array()?;
        for c in content {
            if c.get("type")?.as_str()? == "output_text" {
                if let Some(t) = c.get("text").and_then(|t| t.as_str()) {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

fn sanitize_quiz(quiz: &mut GeneratedTrivia, requested_topic: &str) {
    quiz.title = quiz.title.trim().to_string();
    quiz.topic = if quiz.topic.trim().is_empty() {
        requested_topic.to_string()
    } else {
        quiz.topic.trim().to_string()
    };
    quiz.description = quiz.description.trim().to_string();

    if quiz.title.is_empty() {
        quiz.title = format!("Trivia: {}", quiz.topic);
    }
    if quiz.description.is_empty() {
        quiz.description = format!("A 5-question trivia quiz about {}.", quiz.topic);
    }

    for q in &mut quiz.questions {
        q.text = q.text.trim().to_string();
        if q.duration_seconds <= 0 {
            q.duration_seconds = 20;
        }
        if q.max_points < 100 {
            q.max_points = 100;
        }
        for o in &mut q.options {
            o.label = o.label.trim().to_string();
        }
        // If the model returns empty strings for optional fields, normalize to None
        if matches!(q.annotation_url.as_deref(), Some(s) if s.trim().is_empty()) {
            q.annotation_url = None;
        }
        if matches!(q.random_fact.as_deref(), Some(s) if s.trim().is_empty()) {
            q.random_fact = None;
        }
        if matches!(q.incorrect_fluff.as_deref(), Some(s) if s.trim().is_empty()) {
            q.incorrect_fluff = None;
        }
        if matches!(q.explanation.as_deref(), Some(s) if s.trim().is_empty()) {
            q.explanation = None;
        }
    }
}

fn validate_quiz(quiz: &GeneratedTrivia) -> Result<()> {
    if quiz.questions.len() < 5 {
        return Err(anyhow!(
            "expected >= 5 questions, got {}",
            quiz.questions.len()
        ));
    }

    for (i, q) in quiz.questions.iter().enumerate() {
        if q.text.is_empty() {
            return Err(anyhow!("question {} text is empty", i));
        }
        if q.options.len() != 4 {
            return Err(anyhow!("question {} must have 4 options", i));
        }
        if q.max_points < 100 {
            return Err(anyhow!("question {} max_points must be >= 100", i));
        }

        let correct_count = q.options.iter().filter(|o| o.is_correct).count();
        if correct_count != 1 {
            return Err(anyhow!(
                "question {} must have exactly 1 correct option (got {})",
                i,
                correct_count
            ));
        }

        if q.options.iter().any(|o| o.label.is_empty()) {
            return Err(anyhow!("question {} has an empty option label", i));
        }
    }

    Ok(())
}
