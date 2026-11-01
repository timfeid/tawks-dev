//! Installed-game providers. The bot defines the offered modes; invitations,
//! acceptance and per-person launch tickets are handled by Tawks.
pub use tawks_game_model::{
    GameMode, RegisterGame, RegisteredGame,
    pucklab_hut::{PuckLabHutClubReport, PuckLabHutMatchCheck, PuckLabHutMatchReport},
    pucklab_jersey::PuckLabJerseyReport,
    pucklab_leaders::PuckLabLeadersReport,
};

impl crate::TawksBot {
    pub async fn register_game(&self, game: &RegisterGame) -> anyhow::Result<RegisteredGame> {
        game.validate().map_err(anyhow::Error::msg)?;
        self.sdk.web_client.mutate("game_register", game).await
    }

    /// PuckLab HUT bot only: people whose club Tawks wants checked.
    pub async fn hut_club_checks(&self) -> anyhow::Result<Vec<String>> {
        self.sdk.web_client.query("game_hut_club_checks", &()).await
    }

    pub async fn report_hut_clubs(&self, reports: &[PuckLabHutClubReport]) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate("game_hut_club_report", &reports)
            .await
    }

    /// PuckLab HUT bot only: accepted invitations whose match Tawks follows.
    pub async fn hut_match_checks(&self) -> anyhow::Result<Vec<PuckLabHutMatchCheck>> {
        self.sdk.web_client.query("game_hut_match_checks", &()).await
    }

    pub async fn report_hut_matches(&self, reports: &[PuckLabHutMatchReport]) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate("game_hut_match_report", &reports)
            .await
    }

    /// The bot that sells PuckLab only: PuckLab jersey owners whose club design Tawks wants read.
    pub async fn pucklab_jersey_checks(&self) -> anyhow::Result<Vec<String>> {
        self.sdk.web_client.query("pucklab_jersey_checks", &()).await
    }

    /// Each owner's current club jersey, or none for the default PuckLab jersey. Returns how many changed.
    pub async fn report_pucklab_jerseys(&self, reports: &[PuckLabJerseyReport]) -> anyhow::Result<u32> {
        self.sdk
            .web_client
            .mutate("pucklab_jersey_report", &reports)
            .await
    }

    /// The bot that sells PuckLab only: who leads PuckLab's Online Ranked ladder.
    pub async fn report_pucklab_leaders(
        &self,
        report: &PuckLabLeadersReport,
    ) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate("pucklab_leaders_report", report)
            .await
    }
}
