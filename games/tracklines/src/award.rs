//! Reports finished games to Tawks, which owns what they pay.
use std::sync::{Arc, RwLock};

use serde_json::{Value, json};
use tawks_bot::web_client::WebClient;

pub const GAME: &str = "rails";

static WEB: RwLock<Option<Arc<WebClient>>> = RwLock::new(None);

/// Called on every bot connection, so a reconnect replaces the client.
pub fn install(web: Arc<WebClient>) {
    if let Ok(mut slot) = WEB.write() {
        *slot = Some(web);
    }
}

/// Fire and forget: the game never waits on, or fails with, the award.
pub(crate) fn report(args: Value) {
    let Some(web) = WEB.read().ok().and_then(|slot| slot.clone()) else {
        return;
    };
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    runtime.spawn(async move {
        if let Err(error) = web.mutate::<_, Value>("game_award", &args).await {
            tracing::warn!(%error, game = GAME, "game_award failed");
        }
    });
}

pub(crate) struct Standing<'a> {
    pub id: &'a str,
    pub is_ai: bool,
    /// Higher is better; equal ranks tie.
    pub rank: (i32, usize),
}

/// Full standings with tied places shared. AI seats keep their places, so a
/// person behind one is not moved up, but they are not reported.
pub(crate) fn players(standings: &[Standing]) -> Vec<Value> {
    let mut sorted: Vec<&Standing> = standings.iter().collect();
    sorted.sort_by(|a, b| b.rank.cmp(&a.rank).then_with(|| a.id.cmp(b.id)));
    sorted
        .iter()
        .filter(|s| !s.is_ai)
        .map(|s| {
            let place = 1 + sorted.iter().filter(|o| o.rank > s.rank).count();
            json!({ "user_id": s.id, "place": place })
        })
        .collect()
}

pub(crate) fn reference_id(lobby_id: &str, game_seed: u64) -> String {
    format!("{GAME}:{lobby_id}:{game_seed:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(id: &str, is_ai: bool, score: i32) -> Standing<'_> {
        Standing { id, is_ai, rank: (score, 0) }
    }

    #[test]
    fn standings_share_tied_places_and_skip_ai_seats() {
        let out = players(&[
            seat("bob", false, 40),
            seat("cpu", true, 90),
            seat("ann", false, 40),
            seat("cy", false, 10),
        ]);
        assert_eq!(
            out,
            vec![
                json!({ "user_id": "ann", "place": 2 }),
                json!({ "user_id": "bob", "place": 2 }),
                json!({ "user_id": "cy", "place": 4 }),
            ]
        );
    }

    #[test]
    fn ticket_count_breaks_score_ties_like_the_winner_pick() {
        let out = players(&[
            Standing { id: "ann", is_ai: false, rank: (50, 3) },
            Standing { id: "bob", is_ai: false, rank: (50, 4) },
        ]);
        assert_eq!(out[0], json!({ "user_id": "bob", "place": 1 }));
        assert_eq!(out[1], json!({ "user_id": "ann", "place": 2 }));
    }

    #[test]
    fn rematches_get_new_references() {
        assert_ne!(reference_id("l1", 7), reference_id("l1", 8));
    }
}
