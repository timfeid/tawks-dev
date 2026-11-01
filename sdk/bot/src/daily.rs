//! Daily-game bots tell Tawks when a player finishes the day's game; Tawks
//! decides what that pays and pays each game at most once a day.
use std::sync::Arc;
use std::time::Duration;

use tawks_game_model::daily_finish::DailyFinishReport;
use tracing::warn;

use crate::web_client::WebClient;

const ATTEMPTS: u32 = 3;

#[derive(Clone)]
pub struct DailyFinishes {
    web_client: Arc<WebClient>,
    game: &'static str,
}

impl std::fmt::Debug for DailyFinishes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DailyFinishes")
            .field("game", &self.game)
            .finish_non_exhaustive()
    }
}

impl crate::TawksBot {
    pub fn daily_finishes(&self, game: &'static str) -> DailyFinishes {
        DailyFinishes {
            web_client: self.sdk.web_client.clone(),
            game,
        }
    }
}

impl DailyFinishes {
    /// Reports in the background so the player's move is not held up. A
    /// repeated report pays nothing, so a failed one is simply sent again.
    /// `day` is the puzzle day as `YYYY-MM-DD`, or None for Tawks' current day.
    pub fn report(&self, user_id: &str, day: Option<String>) {
        let web_client = self.web_client.clone();
        let report = DailyFinishReport {
            user_id: user_id.to_string(),
            game: self.game.to_string(),
            day,
        };
        tokio::spawn(async move {
            for attempt in 1..=ATTEMPTS {
                match web_client
                    .mutate::<_, Option<i32>>("dailies_finished", &report)
                    .await
                {
                    Ok(_) => return,
                    Err(error) if attempt == ATTEMPTS => {
                        warn!(%error, game = %report.game, user_id = %report.user_id, "could not report a daily finish")
                    }
                    Err(_) => tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await,
                }
            }
        });
    }
}
