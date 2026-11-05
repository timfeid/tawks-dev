//! Reports finished games to Tawks, which owns what they pay.
use std::sync::{Arc, RwLock};

use serde_json::{Value, json};
use tawks_bot::web_client::WebClient;

pub const GAME: &str = "midnight";

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

/// Winners place first together; everyone else played without a place.
pub(crate) fn players<'a>(results: impl IntoIterator<Item = (&'a str, bool)>) -> Vec<Value> {
    let mut results: Vec<(&str, bool)> = results.into_iter().collect();
    results.sort_unstable();
    results
        .into_iter()
        .map(|(id, won)| {
            if won {
                json!({ "user_id": id, "place": 1 })
            } else {
                json!({ "user_id": id })
            }
        })
        .collect()
}

pub(crate) fn reference_id(lobby_id: &str, started_at_ms: i64) -> String {
    format!("{GAME}:{lobby_id}:{started_at_ms}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winners_share_first_and_the_rest_are_unplaced() {
        let out = players([("cy", false), ("ann", true), ("bob", true)]);
        assert_eq!(
            out,
            vec![
                json!({ "user_id": "ann", "place": 1 }),
                json!({ "user_id": "bob", "place": 1 }),
                json!({ "user_id": "cy" }),
            ]
        );
    }

    #[test]
    fn each_game_started_gets_its_own_reference() {
        assert_ne!(reference_id("l1", 1000), reference_id("l1", 2000));
    }
}
