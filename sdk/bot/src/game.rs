//! Installed-game providers. The bot defines the offered modes; invitations,
//! acceptance and per-person launch tickets are handled by Tawks.
pub use tawks_game_model::{GameMode, RegisterGame, RegisteredGame};

impl crate::TawksBot {
    pub async fn register_game(&self, game: &RegisterGame) -> anyhow::Result<RegisteredGame> {
        game.validate().map_err(anyhow::Error::msg)?;
        self.sdk.web_client.mutate("game_register", game).await
    }
}
