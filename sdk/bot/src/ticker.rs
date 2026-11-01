use serde::Serialize;
use serde_json::{json, Value};

use crate::TawksBot;

/// One item in a niche's top bar, such as a game's score. Set it again with
/// the same key to update it.
#[derive(Debug, Clone, Serialize)]
pub struct TickerItem {
    pub key: String,
    pub niche_id: String,
    pub title: String,
    pub detail: Option<String>,
    pub url: Option<String>,
    pub label: Option<String>,
    pub color: Option<String>,
    pub background: Option<String>,
    pub image: Option<String>,
    pub channel_ids: Vec<String>,
    pub live: bool,
    pub position: i32,
    pub expires_in: Option<i64>,
    pub starts_at: Option<String>,
}

impl TickerItem {
    pub fn new(key: impl Into<String>, niche_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            niche_id: niche_id.into(),
            title: title.into(),
            detail: None,
            url: None,
            label: None,
            color: None,
            background: None,
            image: None,
            channel_ids: Vec::new(),
            live: false,
            position: 0,
            expires_in: None,
            starts_at: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// An in-app path, starting with `/`.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// What the item is, shown first: `Live`, `Final`, `Trending`. Up to 12 characters.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// An `https` picture shown small before the label, such as a team's logo.
    pub fn image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }

    /// Each team's `https` logo for an "away - home" game, drawn beside its name.
    pub fn logos(mut self, away: impl Into<String>, home: impl Into<String>) -> Self {
        self.image = Some(format!("{},{}", away.into(), home.into()));
        self
    }

    /// The channels the item is about; people following one can put it first.
    pub fn channels(mut self, channel_ids: Vec<String>) -> Self {
        self.channel_ids = channel_ids;
        self
    }

    /// A `#rrggbb` colour for the label.
    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// A dark `#rrggbb` background for the whole tab, or two as
    /// `#rrggbb,#rrggbb` to split it on a slant.
    pub fn background(mut self, background: impl Into<String>) -> Self {
        self.background = Some(background.into());
        self
    }

    /// Shows a red live dot.
    pub fn live(mut self, live: bool) -> Self {
        self.live = live;
        self
    }

    pub fn position(mut self, position: i32) -> Self {
        self.position = position;
        self
    }

    pub fn expires_in(mut self, seconds: i64) -> Self {
        self.expires_in = Some(seconds);
        self
    }

    /// When the event starts, as RFC 3339, such as `2026-10-27T23:00:00Z`.
    /// The bar runs live items first, then in start order.
    pub fn starts_at(mut self, at: impl Into<String>) -> Self {
        self.starts_at = Some(at.into());
        self
    }
}

/// A burst across the top bar, such as `GOAL`, for a niche's members or for
/// the people who follow one of its channels.
#[derive(Debug, Clone, Serialize)]
pub struct Flash {
    pub niche_id: String,
    pub title: String,
    pub text: Option<String>,
    pub style: String,
    pub url: Option<String>,
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub look: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<Value>,
}

impl Flash {
    pub fn new(niche_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            niche_id: niche_id.into(),
            title: title.into(),
            text: None,
            style: "info".into(),
            url: None,
            channel_id: None,
            look: None,
            actions: Vec::new(),
        }
    }

    /// Overrides parts of the style's look, for example
    /// `json!({ "background": "stripes", "colors": ["#003087", "#c8102e"], "font": "display" })`.
    /// Fields: `background` (stripes, solid, pulse, strobe), `colors` (one or
    /// two `#rrggbb`), `window` (sweep, center, slam, expand, full), `window_color`, `font`
    /// (mono, display, sans), `text_color` and `blink`.
    pub fn look(mut self, look: Value) -> Self {
        self.look = Some(look);
        self
    }

    /// A button that opens an in-app path. Up to two; a flash with buttons waits for a choice.
    pub fn action(mut self, label: impl Into<String>, url: impl Into<String>) -> Self {
        self.actions.push(json!({ "label": label.into(), "url": url.into() }));
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// `goal`, `alert`, `info` or `serious`.
    pub fn style(mut self, style: impl Into<String>) -> Self {
        self.style = style.into();
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Only people who follow this channel.
    pub fn for_followers_of(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }
}

impl TawksBot {
    pub async fn set_ticker(&self, item: &TickerItem) -> anyhow::Result<()> {
        self.sdk.web_client.mutate::<_, Value>("ticker_set", item).await?;
        Ok(())
    }

    pub async fn remove_ticker(&self, key: &str) -> anyhow::Result<()> {
        self.sdk.web_client.mutate::<_, Value>("ticker_remove", &json!({ "key": key })).await?;
        Ok(())
    }

    /// Returns how many people it reached.
    pub async fn flash(&self, flash: &Flash) -> anyhow::Result<usize> {
        Ok(self.sdk.web_client.mutate::<_, usize>("ticker_flash", flash).await?)
    }
}
