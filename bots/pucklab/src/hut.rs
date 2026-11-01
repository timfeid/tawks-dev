//! Provider metadata/capabilities only. No registration, invitation messages,
//! grant exchange or executable dispatch is enabled before menu packaging.
use tawks_bot::workflow::ui::TextRole;
use crate::club::text;
use anyhow::{Result, ensure};
use reqwest::Url;
use serde_json::json;
use tawks_bot::workflow::{
    CreateWorkflowDefinition,
    builder::{WorkflowDefinitionBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder},
};
use crate::client::{ApiError, PuckLab};
use tawks_bot::{TawksBot, game::RegisterGame};
use tawks_game_model::pucklab_hut::{
    PuckLabHutClubReport, PuckLabHutMatchCheck, PuckLabHutMatchReport, PuckLabHutMatchState,
    PuckLabHutProvider,
};

pub const TYPED_ACCOUNT_MENU_PACKAGED: bool = false;

#[derive(Clone, Debug)]
pub struct HutCapabilities {
    pub provider: PuckLabHutProvider,
    pub exchange_url: String,
    pub cancel_url: String,
}
impl HutCapabilities {
    pub fn new(bot: String, niche: String, exchange_origin: &str) -> Result<Self> {
        Self::new_with(bot, niche, exchange_origin, cfg!(debug_assertions))
    }
    fn new_with(
        bot: String,
        niche: String,
        exchange_origin: &str,
        allow_loopback: bool,
    ) -> Result<Self> {
        let provider = PuckLabHutProvider::new(bot, niche).map_err(anyhow::Error::msg)?;
        let mut origin = Url::parse(exchange_origin)?;
        ensure!(
            origin.username().is_empty()
                && origin.password().is_none()
                && origin.query().is_none()
                && origin.fragment().is_none()
                && origin.path() == "/"
                && origin.host_str().is_some(),
            "Configure a PuckLab HUT origin without credentials, path, query or fragment"
        );
        ensure!(
            origin.scheme() == "https"
                || (allow_loopback
                    && origin.scheme() == "http"
                    && matches!(origin.host_str(), Some("127.0.0.1" | "localhost"))),
            "PuckLab HUT requires HTTPS (loopback HTTP only in debug builds)"
        );
        origin.set_path("/v1/icelab/native/tawks/prepare");
        let exchange_url = origin.to_string();
        origin.set_path("/v1/icelab/native/tawks/cancel");
        Ok(Self {
            provider,
            exchange_url,
            cancel_url: origin.to_string(),
        })
    }
    /// Misconfiguration disables only HUT; the rest of the bot keeps running.
    pub fn from_env(default_origin: &str) -> Option<Self> {
        Self::from_vars(
            std::env::var("PUCKLAB_HUT_BOT_ID").ok(),
            std::env::var("PUCKLAB_HUT_NICHE_ID").ok(),
            std::env::var("PUCKLAB_HUT_EXCHANGE_ORIGIN").ok(),
            default_origin,
        )
        .unwrap_or_else(|error| {
            tracing::error!(%error, "PuckLab HUT disabled: invalid configuration");
            None
        })
    }
    fn from_vars(
        bot: Option<String>,
        niche: Option<String>,
        origin: Option<String>,
        default_origin: &str,
    ) -> Result<Option<Self>> {
        let Some(bot) = bot else {
            return Ok(None);
        };
        let niche = niche.ok_or_else(|| {
            anyhow::anyhow!("PUCKLAB_HUT_BOT_ID is set but PUCKLAB_HUT_NICHE_ID is missing")
        })?;
        let origin = origin.unwrap_or_else(|| default_origin.into());
        Self::new(bot, niche, &origin).map(Some)
    }
    pub fn registration(&self) -> Option<RegisterGame> {
        // An environment toggle cannot expose a launcher absent from installer.
        TYPED_ACCOUNT_MENU_PACKAGED.then(|| self.provider.metadata())
    }
    pub fn workflow(&self) -> CreateWorkflowDefinition {
        let visual=WorkflowVisualBuilder::new("capabilities").page(tawks_bot::workflow::PageWidth::Normal,vec![
            text(TextRole::Heading,"Lab Rivals"),
            text(TextRole::Body,"Your existing club, cards and jerseys. Tawks chat challenges keep the host in control of choosing an opponent."),
            text(TextRole::Fine,"Account-backed fixture exchange is implemented. Playing from a Tawks invite still requires the packaged account-menu launcher and release verification."),
        ]).build();
        WorkflowDefinitionBuilder::new("pucklab_hut_capabilities", "Lab Rivals status")
            .description("Implementation status; installed-game invitations are not enabled.")
            .multi_instance_per_user()
            .response("provider", json!(self.provider.metadata()))
            .response("game_id", json!(self.provider.id()))
            .response("launch_available", json!(false))
            .response("exchange_url", json!(self.exchange_url))
            .response("cancel_url", json!(self.cancel_url))
            .response(
                "exchange_auth",
                json!("normal_pucklab_account_and_own_tawks_ticket"),
            )
            .add_node(
                "capabilities",
                WorkflowNodeBuilder::new("Capabilities")
                    .visual(visual)
                    .build(),
            )
            .build()
    }
}

pub async fn register_capabilities(
    bot: &TawksBot,
    capabilities: &HutCapabilities,
) -> Result<String> {
    // Deliberately NOT register_game(). No public provider/discovery/channel,
    // external message or launch grant is published by this workflow.
    bot.register_workflow(capabilities.workflow()).await
}

/// Ok(None) means PuckLab could not answer, so the check stays pending.
pub async fn club_ready(api: &PuckLab, roster: &str, subject: &str) -> Result<Option<bool>> {
    let read = async {
        let hash = api.roster_hash(subject, roster).await?;
        api.get(subject, "/v1/icelab/state", &[("roster", roster)], Some(&hash))
            .await
    };
    match read.await {
        Ok(state) => Ok(Some(club_in(&state))),
        Err(error)
            if error
                .downcast_ref::<ApiError>()
                .is_some_and(|e| e.pucklab_link_required) =>
        {
            Ok(Some(false))
        }
        Err(error) => {
            tracing::warn!(%error, "PuckLab HUT club check failed");
            Ok(None)
        }
    }
}

fn club_in(state: &serde_json::Value) -> bool {
    state.get("club").is_some_and(|club| !club.is_null())
}

/// One pass over the people Tawks asked about; reports only real answers.
pub async fn sync_clubs(bot: &TawksBot, api: &PuckLab, roster: &str) -> Result<usize> {
    let mut reports = Vec::new();
    for user_id in bot.hut_club_checks().await? {
        if let Some(ready) = club_ready(api, roster, &user_id).await? {
            reports.push(PuckLabHutClubReport { user_id, ready });
        }
    }
    if !reports.is_empty() {
        bot.report_hut_clubs(&reports).await?;
    }
    Ok(reports.len())
}

/// None when PuckLab could not answer or knows no match yet; Tawks keeps asking.
pub async fn match_report(
    api: &PuckLab,
    roster: &str,
    check: &PuckLabHutMatchCheck,
) -> Result<Option<PuckLabHutMatchReport>> {
    let status = api
        .get(
            &check.host_user_id,
            "/v1/icelab/native/tawks/status",
            &[("roster", roster), ("session_id", &check.session_id)],
            None,
        )
        .await;
    match status {
        Ok(status) => Ok(report_from(&check.session_id, &status)),
        Err(error) => {
            tracing::warn!(%error, "PuckLab HUT match check failed");
            Ok(None)
        }
    }
}

fn report_from(session_id: &str, status: &serde_json::Value) -> Option<PuckLabHutMatchReport> {
    let (state, reason) = match status.get("state").and_then(|s| s.as_str())? {
        "open" | "none" => (PuckLabHutMatchState::Open, None),
        "final" => (PuckLabHutMatchState::Final, None),
        "disputed" => (PuckLabHutMatchState::Disputed, None),
        "cancelled" => (
            PuckLabHutMatchState::Cancelled,
            Some(
                status
                    .get("reason")
                    .and_then(|r| r.as_str())
                    .unwrap_or("pucklab_cancelled")
                    .to_owned(),
            ),
        ),
        _ => return None,
    };
    let report = PuckLabHutMatchReport {
        session_id: session_id.into(),
        state,
        reason,
    };
    report.validate().is_ok().then_some(report)
}

/// One pass over the accepted invitations Tawks follows.
pub async fn sync_matches(bot: &TawksBot, api: &PuckLab, roster: &str) -> Result<usize> {
    let mut reports = Vec::new();
    for check in bot.hut_match_checks().await? {
        if let Some(report) = match_report(api, roster, &check).await? {
            reports.push(report);
        }
    }
    if !reports.is_empty() {
        bot.report_hut_matches(&reports).await?;
    }
    Ok(reports.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pucklab_match_states_map_to_reports() {
        let report = |value| report_from("GS-1", &value).map(|r| (r.state, r.reason));
        assert_eq!(
            report(json!({"state":"none"})),
            Some((PuckLabHutMatchState::Open, None))
        );
        assert_eq!(
            report(json!({"state":"final"})),
            Some((PuckLabHutMatchState::Final, None))
        );
        assert_eq!(
            report(json!({"state":"cancelled","reason":"tawks_hut_abandoned_by_away"})),
            Some((
                PuckLabHutMatchState::Cancelled,
                Some("tawks_hut_abandoned_by_away".into())
            ))
        );
        assert_eq!(
            report(json!({"state":"cancelled"})),
            Some((
                PuckLabHutMatchState::Cancelled,
                Some("pucklab_cancelled".into())
            ))
        );
        assert_eq!(
            report(json!({"state":"cancelled","reason":"Not A Code"})),
            None
        );
        assert_eq!(report(json!({"state":"mystery"})), None);
    }

    #[test]
    fn only_a_present_club_object_counts_as_ready() {
        assert!(club_in(&json!({"club":{"name":"Fixture club"}})));
        assert!(!club_in(&json!({"club":null})));
        assert!(!club_in(&json!({})));
    }
    #[test]
    fn provider_remains_unregistered_and_workflow_never_claims_playable_launch() {
        let config = HutCapabilities::new(
            "pucklab-bot".into(),
            "actual-hockey-id".into(),
            "https://playpucklab.com",
        )
        .unwrap();
        assert!(config.registration().is_none());
        assert_eq!(
            config.exchange_url,
            "https://playpucklab.com/v1/icelab/native/tawks/prepare"
        );
        let workflow = serde_json::to_value(config.workflow()).unwrap();
        assert_eq!(workflow["responses"]["launch_available"], false);
        let encoded = workflow.to_string();
        assert!(!encoded.contains("\"ticket\":"));
    }
    #[test]
    fn target_origin_cannot_supply_credentials_query_redirect_or_nonloopback_http() {
        for origin in [
            "http://example.com",
            "https://user:pass@example.com",
            "https://example.com/path",
            "https://example.com?url=https://evil.example",
            "https://example.com#fragment",
            "file:///tmp/game",
        ] {
            assert!(
                HutCapabilities::new("bot".into(), "hockey-id".into(), origin).is_err(),
                "{origin}"
            );
        }
        assert!(HutCapabilities::new("bot".into(), "hockey-id".into(), "https://playpucklab.com").is_ok());
    }
    #[test]
    fn loopback_http_origin_is_debug_build_only() {
        for origin in ["http://localhost:1234", "http://127.0.0.1:1234"] {
            let new = |allow| HutCapabilities::new_with("bot".into(), "hockey-id".into(), origin, allow);
            assert!(new(true).is_ok(), "{origin}");
            assert!(new(false).is_err(), "{origin}");
            assert_eq!(
                HutCapabilities::new("bot".into(), "hockey-id".into(), origin).is_ok(),
                cfg!(debug_assertions)
            );
        }
    }
    #[test]
    fn bot_id_without_niche_is_a_loud_hut_only_error() {
        let default = "https://playpucklab.com";
        assert!(HutCapabilities::from_vars(None, None, None, default).unwrap().is_none());
        let error = HutCapabilities::from_vars(Some("bot".into()), None, None, default).unwrap_err();
        assert!(error.to_string().contains("PUCKLAB_HUT_NICHE_ID"));
        assert!(HutCapabilities::from_vars(Some("bot".into()), Some("hockey-id".into()), None, default)
            .unwrap()
            .is_some());
    }
}
