//! Reports finished games to Tawks, which owns what they pay.
use std::sync::{Arc, RwLock};

use serde_json::{Value, json};
use tawks_bot::web_client::WebClient;

pub const GAME: &str = "cards";

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

/// The winner places first and everyone else second. AI seats are left out,
/// so a practice game reports at most one person.
pub(crate) fn players<'a>(
    winner: Option<&str>,
    seats: impl IntoIterator<Item = (&'a str, bool)>,
) -> Vec<Value> {
    let mut humans: Vec<&str> = seats
        .into_iter()
        .filter(|(id, is_ai)| !is_ai && *id != "system")
        .map(|(id, _)| id)
        .collect();
    humans.sort_unstable();
    humans.dedup();
    humans
        .into_iter()
        .map(|id| json!({ "user_id": id, "place": if Some(id) == winner { 1 } else { 2 } }))
        .collect()
}

pub(crate) fn reference_id(lobby_id: &str, game_seed: u64) -> String {
    format!("{GAME}:{lobby_id}:{game_seed:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winner_first_and_ai_seats_left_out() {
        let out = players(Some("ann"), [("bob", false), ("ann", false), ("cpu", true)]);
        assert_eq!(
            out,
            vec![
                json!({ "user_id": "ann", "place": 1 }),
                json!({ "user_id": "bob", "place": 2 }),
            ]
        );
    }

    #[test]
    fn an_ai_winner_places_nobody_first() {
        let out = players(Some("cpu"), [("ann", false), ("cpu", true)]);
        assert_eq!(out, vec![json!({ "user_id": "ann", "place": 2 })]);
    }

    #[test]
    fn rematches_get_new_references() {
        assert_ne!(reference_id("l1", 1), reference_id("l1", 2));
        assert_eq!(reference_id("l1", 255), "cards:l1:ff");
    }
}
