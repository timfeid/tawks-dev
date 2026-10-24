use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use s3::{Bucket, Region, creds::Credentials};
use serde_json::json;

/// Card and deck art: images from the OpenAI images API, stored in an S3
/// bucket served at a public base URL.
///
/// Reads OPENAI_API_KEY, S3_BUCKET_NAME, CARD_ART_BASE_URL and optionally
/// S3_REGION (us-east-1), plus the usual AWS credential variables.
pub struct ArtStudio {
    client: Client,
    api_key: String,
    bucket: Box<Bucket>,
    base_url: String,
}

impl ArtStudio {
    pub fn from_env() -> Result<Self> {
        let var = |key: &str| std::env::var(key).with_context(|| format!("{key} is required"));
        let region = match std::env::var("S3_REGION") {
            Ok(name) => name.parse()?,
            Err(_) => Region::UsEast1,
        };
        let credentials = Credentials::default().map_err(|e| anyhow!("S3 credentials: {e}"))?;
        let bucket = Bucket::new(&var("S3_BUCKET_NAME")?, region, credentials)?.with_path_style();
        Ok(Self {
            client: Client::builder().timeout(Duration::from_secs(60)).build()?,
            api_key: var("OPENAI_API_KEY")?,
            bucket,
            base_url: var("CARD_ART_BASE_URL")?.trim_end_matches('/').to_string(),
        })
    }

    /// True when the image already lives in this studio's storage.
    pub fn is_stored(&self, url: Option<&str>) -> bool {
        url.is_some_and(|u| u.starts_with(&self.base_url))
    }

    /// Returns a short-lived URL of the generated image.
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let body = json!({
            "model": "dall-e-3",
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024",
            "quality": "standard",
            "response_format": "url"
        });
        let resp = self
            .client
            .post("https://api.openai.com/v1/images/generations")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("OpenAI request failed")?;
        let status = resp.status();
        let data: serde_json::Value = resp.json().await.context("OpenAI response was not JSON")?;
        if !status.is_success() {
            let msg = data["error"]["message"].as_str().unwrap_or("unknown error");
            return Err(anyhow!("OpenAI API error: {msg}"));
        }
        data["data"][0]["url"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("OpenAI response had no image URL"))
    }

    /// Copies the image at `url` to `path` in the bucket and returns its public URL.
    pub async fn store(&self, url: &str, path: &str) -> Result<String> {
        let bytes = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;
        self.bucket
            .put_object_with_content_type(path, &bytes, "image/png")
            .await
            .context("S3 upload failed")?;
        Ok(format!("{}/{path}", self.base_url))
    }
}
