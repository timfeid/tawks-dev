use serde_json::Value;
use tawks_bot::share::ShareCard;

pub const PLAY_PATH: &str = "/niche/tawks/trivia";

/// The finished quiz as a share card: right answers out of the total, the
/// points, and one square per question.
pub fn card(title: &str, audit: &[Value], score: i64, day: &str) -> Option<ShareCard> {
    if audit.is_empty() {
        return None;
    }
    let right = audit.iter().filter(|a| a.get("is_correct").and_then(Value::as_bool) == Some(true)).count();
    let squares: String = audit
        .iter()
        .map(|a| if a.get("is_correct").and_then(Value::as_bool) == Some(true) { 'g' } else { 'r' })
        .collect();
    let rows = squares
        .as_bytes()
        .chunks(5)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect();
    let detail = if title.trim().is_empty() {
        format!("{score} points")
    } else {
        format!("{score} points on {}", title.trim())
    };
    Some(ShareCard {
        game: "trivia".into(),
        name: "Trivia".into(),
        day: day.into(),
        headline: format!("{right}/{} correct", audit.len()),
        detail,
        rows,
        play: PLAY_PATH.into(),
        lines: Vec::new(),
    })
}

pub fn view(title: &str, audit: &[Value], score: i64, day: &str) -> Value {
    card(title, audit, score, day).map(|c| c.view()).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_finished_quiz_counts_right_answers_and_draws_a_square_each() {
        let audit: Vec<Value> = [true, false, true, true, false, true, true]
            .iter()
            .map(|c| json!({ "is_correct": c }))
            .collect();
        let card = card("World Capitals", &audit, 640, "2026-09-28").unwrap();
        assert_eq!(card.headline, "5/7 correct");
        assert_eq!(card.detail, "640 points on World Capitals");
        assert_eq!(card.rows, vec!["grggr".to_string(), "gg".to_string()]);
        assert_eq!(card.play, PLAY_PATH);
        assert!(card.path().starts_with("/share/trivia/"));
    }

    #[test]
    fn nothing_answered_means_nothing_to_share() {
        assert_eq!(view("Quiz", &[], 0, "2026-09-28"), Value::Null);
    }
}
