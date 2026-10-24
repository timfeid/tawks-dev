//! Result cards for the daily games. The whole card travels in the link, so
//! the app can render its preview and image without storing anything:
//! `/share/<game>/<base64url JSON>`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Serialize;
use serde_json::{Value, json};

/// Colours a row of the card's grid can use, one letter per square: green,
/// yellow, blue, purple, orange, red, white, and `k` for an empty square.
pub const SQUARES: &str = "gybporwk";

#[derive(Serialize, Clone, Debug, Default)]
pub struct ShareCard {
    /// Game key, also the first path segment.
    #[serde(rename = "g")]
    pub game: String,
    #[serde(rename = "n")]
    pub name: String,
    /// The puzzle's day, `YYYY-MM-DD`.
    #[serde(rename = "d")]
    pub day: String,
    /// The big line: "3/3 solved", "Solved in 2:14".
    #[serde(rename = "h")]
    pub headline: String,
    #[serde(rename = "s", skip_serializing_if = "String::is_empty")]
    pub detail: String,
    #[serde(rename = "r", skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<String>,
    /// Short text rows shown under the headline, such as a lineup's lines.
    #[serde(rename = "l", skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<String>,
    /// Where "play" goes, e.g. `/niche/tawks/railyard`.
    #[serde(rename = "p")]
    pub play: String,
}

impl ShareCard {
    pub fn path(&self) -> String {
        let payload = serde_json::to_vec(self).unwrap_or_default();
        format!("/share/{}/{}", self.game, URL_SAFE_NO_PAD.encode(payload))
    }

    /// What a view puts under its share key for the `share_result` display.
    pub fn view(&self) -> Value {
        let mut text = format!("{} {}\n{}", self.name, pretty_day(&self.day), self.headline);
        if !self.detail.is_empty() {
            text.push_str(&format!(", {}", self.detail));
        }
        text.push_str("\nCan you beat it?");
        json!({ "path": self.path(), "title": format!("{} {}", self.name, pretty_day(&self.day)), "text": text })
    }
}

fn pretty_day(day: &str) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let mut parts = day.split('-');
    let (_, Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else {
        return day.to_string();
    };
    match (m.parse::<usize>(), d.parse::<u32>()) {
        (Ok(m), Ok(d)) if (1..=12).contains(&m) => format!("{} {d}", MONTHS[m - 1]),
        _ => day.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_card_round_trips_through_its_path() {
        let card = ShareCard {
            game: "railyard".into(),
            name: "Railyard".into(),
            day: "2026-09-27".into(),
            headline: "3/3 solved".into(),
            detail: "2 at par".into(),
            rows: vec!["ggy".into()],
            play: "/niche/tawks/railyard".into(),
            lines: Vec::new(),
        };
        let path = card.path();
        let encoded = path.strip_prefix("/share/railyard/").unwrap();
        let decoded: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).unwrap()).unwrap();
        assert_eq!(decoded["h"], "3/3 solved");
        assert_eq!(decoded["r"][0], "ggy");
        assert_eq!(card.view()["title"], "Railyard Sep 27");
        assert!(card.view()["text"].as_str().unwrap().starts_with("Railyard Sep 27\n3/3 solved, 2 at par"));
    }
}
