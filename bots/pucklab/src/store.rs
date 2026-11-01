//! PuckLab in the Tawks store. Tickets buy the game once, online play included
//! for good. Tickets never buy coins or gameplay. PuckLab asks Tawks who owns it.
use crate::club::text;
use crate::client::{ApiError, PuckLab};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use tawks_bot::{
    TawksBot,
    store::{Product, ProductOwner},
    workflow::{
        CreateWorkflowDefinition, PageWidth,
        builder::{WorkflowDefinitionBuilder, WorkflowNodeBuilder, WorkflowVisualBuilder},
        ui::TextRole,
    },
};

pub const GAME: &str = "pucklab";
/// The retired monthly online pass, kept registered only so it stays off sale.
pub const ONLINE_MONTH: &str = "pucklab-month";
pub const GAME_PRICE: i32 = 30;
const ONLINE_MONTH_PRICE: i32 = 10;
// Tawks requires a license term; PuckLab ignores it because owning the game is online for good.
const ACCESS_DAYS: i32 = 366;

fn workflow() -> CreateWorkflowDefinition {
    let visual = WorkflowVisualBuilder::new("owned")
        .page(PageWidth::Normal, vec![
            text(TextRole::Heading, "PuckLab"),
            text(TextRole::Body, "PuckLab is yours to download, with online play included for good."),
        ])
        .build();
    WorkflowDefinitionBuilder::new("pucklab_store", "PuckLab")
        .description("The PuckLab game, bought once with tickets.")
        .multi_instance_per_user()
        .add_node("owned", WorkflowNodeBuilder::new("PuckLab").visual(visual).build())
        .build()
}

pub fn products(workflow_id: &str, on_sale: bool) -> [Product; 2] {
    [
        Product::game_license(GAME, "PuckLab", GAME_PRICE, ACCESS_DAYS, workflow_id)
            .description("The PuckLab hockey game. Buy it once, online play included.")
            .icon("gamepad-2")
            .active(on_sale),
        Product::access_pass(ONLINE_MONTH, "PuckLab online · 30 days", ONLINE_MONTH_PRICE, GAME, 30, workflow_id)
            .description("Retired: owning PuckLab now includes online play.")
            .icon("wifi")
            .active(false),
    ]
}

/// Off the store until PUCKLAB_STORE=on.
fn on_sale() -> bool {
    std::env::var("PUCKLAB_STORE").is_ok_and(|value| value == "on")
}

pub async fn register(bot: &TawksBot) -> Result<()> {
    let workflow_id = bot.register_workflow(workflow()).await?;
    for product in products(&workflow_id, on_sale()) {
        bot.register_product(&product).await?;
    }
    Ok(())
}

/// PuckLab access as last reported to PuckLab, by Tawks user: whether they own
/// it, and 0 for no online expiry.
pub type Reported = HashMap<String, (bool, i64)>;

/// Tells PuckLab each owner's access, but only when it changed since the last pass.
/// Users who have not linked a PuckLab account yet are retried on later passes.
pub async fn sync_entitlements(bot: &TawksBot, api: &PuckLab, reported: &mut Reported) -> Result<usize> {
    let mut after: Option<String> = None;
    let mut sent = 0;
    loop {
        let page = bot.list_product_owners(GAME, after.as_deref(), 200).await?;
        for owner in &page {
            let state = entitlement(owner);
            if reported.get(&owner.user_id) == Some(&state) {
                continue;
            }
            let body = json!({"owns_game": state.0, "online_until": state.1});
            match api.post(&owner.user_id, "/v1/bot/entitlement", &body, None).await {
                Ok(_) => {
                    reported.insert(owner.user_id.clone(), state);
                    sent += 1;
                }
                Err(error) if error.downcast_ref::<ApiError>().is_some_and(|e| e.pucklab_link_required) => {}
                Err(error) => tracing::warn!(%error, "PuckLab entitlement report failed"),
            }
        }
        if page.len() < 200 {
            return Ok(sent);
        }
        after = page.last().map(|owner| owner.user_id.clone());
    }
}

fn entitlement(owner: &ProductOwner) -> (bool, i64) {
    (owner.owns, 0)
}

pub async fn owners(bot: &TawksBot, user_ids: &[String]) -> Result<Vec<ProductOwner>> {
    let mut owners = Vec::with_capacity(user_ids.len());
    for chunk in user_ids.chunks(500) {
        owners.extend(bot.product_owners(GAME, chunk).await?);
    }
    Ok(owners)
}

#[cfg(test)]
mod tests {
    #[test]
    fn owners_have_online_play_with_no_expiry() {
        let owner = |owns, at: Option<&str>| ProductOwner {
            user_id: "u".into(),
            owns,
            access_expires_at: at.map(Into::into),
            online_access: owns,
        };
        assert_eq!(entitlement(&owner(true, Some("2026-11-28T00:00:00Z"))), (true, 0));
        assert_eq!(entitlement(&owner(true, None)), (true, 0));
        assert_eq!(entitlement(&owner(false, Some("2026-11-28T00:00:00Z"))), (false, 0));
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn the_game_sells_once_for_tickets_and_the_pass_is_retired() {
        let [game, month] = products("wf-pucklab_store-pucklab-bot", true);
        assert_eq!((game.product_id.as_str(), game.price, game.product_type.as_str()), ("pucklab", 30, "game-license"));
        assert!(game.active);
        assert_eq!(month.product_id, "pucklab-month");
        assert!(!month.active);
        assert!(products("wf", false).iter().all(|product| !product.active));
        for product in products("wf", true) {
            let text = format!("{} {}", product.name, product.description).to_lowercase();
            assert!(!text.contains("coin") && !text.contains("legacy"), "{text}");
        }
    }

    #[test]
    fn owner_rows_read_the_tawks_shape() {
        let rows: Vec<ProductOwner> = serde_json::from_value(json!([
            {"user_id": "alice", "owns": true, "access_expires_at": "2026-11-28T12:00:00Z", "online_access": true},
            {"user_id": "bob", "owns": false, "access_expires_at": null, "online_access": false},
        ]))
        .unwrap();
        assert!(rows[0].owns && rows[0].online_access);
        assert_eq!(rows[1].access_expires_at, None);
    }
}
