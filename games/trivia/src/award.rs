//! Reports finished quizzes and lobby games to Tawks, which owns what they pay.
use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use tawks_bot::web_client::WebClient;

pub const QUIZ_GAME: &str = "trivia-quiz";
pub const LOBBY_GAME: &str = "trivia";

static WEB: OnceLock<Arc<WebClient>> = OnceLock::new();

pub fn install(web: Arc<WebClient>) {
    let _ = WEB.set(web);
}

/// Fire and forget: a quiz or game never waits on, or fails with, the award.
pub(crate) fn report(args: Value) {
    let Some(web) = WEB.get().cloned() else {
        return;
    };
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    runtime.spawn(async move {
        if let Err(error) = web.mutate::<_, Value>("game_award", &args).await {
            tracing::warn!(%error, game = %args["game"], "game_award failed");
        }
    });
}

pub(crate) fn accuracy(score: i64, max_score: i64) -> f64 {
    if max_score <= 0 {
        return 0.0;
    }
    (score as f64 / max_score as f64).clamp(0.0, 1.0)
}

/// One payout per person per quiz, ever: the quiz id is the reference.
pub(crate) fn quiz_args(
    quiz_id: &str,
    user_id: &str,
    accuracy: f64,
    author_id: Option<&str>,
    questions: usize,
) -> Value {
    json!({
        "game": QUIZ_GAME,
        "reference_id": quiz_id,
        "players": [{ "user_id": user_id, "accuracy": accuracy }],
        "author_id": author_id,
        "questions": questions,
    })
}

/// Highest total first; people on the same total share a place.
pub(crate) fn placements(totals: &[(String, i64)]) -> Vec<Vec<String>> {
    let mut sorted: Vec<&(String, i64)> = totals.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut groups: Vec<(i64, Vec<String>)> = Vec::new();
    for (user_id, total) in sorted {
        match groups.last_mut() {
            Some((last, group)) if last == total => group.push(user_id.clone()),
            _ => groups.push((*total, vec![user_id.clone()])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

/// Games hosted in a channel rather than a lobby carry no lobby id.
pub(crate) fn lobby_game_args(
    lobby_key: &str,
    game_id: &str,
    trivia_id: Option<&str>,
    totals: &[(String, i64)],
) -> Value {
    let lobby_id = (!lobby_key.is_empty() && !lobby_key.starts_with('#')).then_some(lobby_key);
    json!({
        "game": LOBBY_GAME,
        "reference_id": format!("{LOBBY_GAME}:{lobby_key}:{game_id}"),
        "lobby_id": lobby_id,
        "placements": placements(totals),
        "meta": { "trivia_id": trivia_id },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(rows: &[(&str, i64)]) -> Vec<(String, i64)> {
        rows.iter().map(|(u, t)| (u.to_string(), *t)).collect()
    }

    #[test]
    fn accuracy_is_the_share_of_the_best_possible_score() {
        assert_eq!(accuracy(50, 100), 0.5);
        assert_eq!(accuracy(0, 100), 0.0);
        assert_eq!(accuracy(120, 100), 1.0);
        assert_eq!(accuracy(10, 0), 0.0);
    }

    #[test]
    fn ties_share_a_place_and_the_next_group_follows() {
        let out = placements(&totals(&[("c", 10), ("a", 30), ("b", 30), ("d", 0)]));
        assert_eq!(
            out,
            vec![
                vec!["a".to_string(), "b".to_string()],
                vec!["c".to_string()],
                vec!["d".to_string()],
            ]
        );
        assert!(placements(&[]).is_empty());
    }

    #[test]
    fn quiz_reference_is_the_quiz() {
        let args = quiz_args("q1", "ann", 0.8, Some("bob"), 10);
        assert_eq!(args["game"], "trivia-quiz");
        assert_eq!(args["reference_id"], "q1");
        assert_eq!(args["players"][0]["user_id"], "ann");
        assert_eq!(args["author_id"], "bob");
        assert_eq!(args["questions"], 10);
        assert!(quiz_args("q1", "ann", 0.8, None, 10)["author_id"].is_null());
    }

    #[test]
    fn channel_games_send_no_lobby() {
        let t = totals(&[("a", 1)]);
        let lobby = lobby_game_args("L1", "g1", Some("q"), &t);
        assert_eq!(lobby["lobby_id"], "L1");
        assert_eq!(lobby["reference_id"], "trivia:L1:g1");
        let channel = lobby_game_args("#channel-c1", "g1", None, &t);
        assert!(channel["lobby_id"].is_null());
    }
}
