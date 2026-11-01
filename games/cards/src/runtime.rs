use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::Utc;
use game_framework::engine::GameEngine;
use sqlx::PgPool;
use tawks_bot::{Product, TawksBot};
use workflow::builder::{WorkflowDefinitionBuilder, WorkflowNodeBuilder};
use workflow::server_action::ServerActionResult;

use crate::pack::PackInstance;
use crate::repository::CardRepository;
use crate::state::{CardGameInitArgs, CardGameState};
use crate::CardGameActionIds;

pub async fn register_runtime(bot: &TawksBot, pool: Arc<PgPool>) -> Result<CardGameActionIds> {
    let engine = Arc::new(GameEngine::<CardGameState>::new(
        CardGameInitArgs {
            pool: Some(pool.clone()),
            ..Default::default()
        },
        bot.clone(),
    ));
    let action_ids = crate::register_routes(engine, "games.cardgame").await?;
    crate::workflows::register(bot, &action_ids).await?;
    register_pack_fulfillment(bot, pool).await?;
    Ok(action_ids)
}

async fn register_pack_fulfillment(bot: &TawksBot, pool: Arc<PgPool>) -> Result<()> {
    let seller = bot.clone();
    let grant_pack = bot
        .register_server_action(
            "cardgame.grant_pack",
            Arc::new(Box::new(move |ctx| {
                let db = pool.clone();
                let seller = seller.clone();
                Box::pin(async move {
                    let repo = CardRepository::new(db);

                    let purchase_id = ctx
                        .workflow
                        .responses
                        .get("purchase_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let payload = ctx
                        .workflow
                        .responses
                        .get("fulfillment_payload")
                        .cloned()
                        .unwrap_or(serde_json::json!({}));

                    let pack_type_id = payload
                        .get("pack_type_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("standard")
                        .to_string();

                    let user_id = ctx.workflow.user_id.clone();
                    let pack_instance_id = ulid::Ulid::new().to_string();
                    let pack = PackInstance {
                        pack_instance_id: pack_instance_id.clone(),
                        pack_type_id: pack_type_id.clone(),
                        owner_id: user_id.clone(),
                        source: "store".to_string(),
                        source_ref: Some(purchase_id.clone()),
                        opened: false,
                        opened_at: None,
                        created_at: Utc::now(),
                    };

                    let stored_id = match repo.create_pack_instance(&pack).await {
                        Ok(id) => id,
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                purchase_id = %purchase_id,
                                "Failed to create pack instance"
                            );
                            settle(&seller, &purchase_id, Err("The pack could not be created"))
                                .await;
                            return Ok(ServerActionResult::Error(format!(
                                "Failed to create pack: {e}"
                            )));
                        }
                    };
                    settle(&seller, &purchase_id, Ok(&stored_id)).await;
                    if stored_id != pack_instance_id {
                        return Ok(ServerActionResult::CompleteWorkflow {
                            responses: HashMap::from([
                                ("pack_instance_id".to_string(), serde_json::json!(stored_id)),
                                ("status".to_string(), serde_json::json!("fulfilled")),
                            ]),
                            close_active_modal: false,
                            message: "Pack granted successfully".to_string(),
                        });
                    }

                    repo.create_pack_audit(
                        &pack_instance_id,
                        "granted",
                        &user_id,
                        Some(serde_json::json!({
                            "source": "store",
                            "purchase_id": purchase_id,
                            "pack_type_id": pack_type_id,
                        })),
                    )
                    .await
                    .ok();

                    tracing::info!(
                        pack_instance_id = %pack_instance_id,
                        user_id = %user_id,
                        purchase_id = %purchase_id,
                        "Granted booster pack"
                    );

                    Ok(ServerActionResult::CompleteWorkflow {
                        responses: HashMap::from([
                            (
                                "pack_instance_id".to_string(),
                                serde_json::json!(pack_instance_id),
                            ),
                            ("status".to_string(), serde_json::json!("fulfilled")),
                        ]),
                        close_active_modal: false,
                        message: "Pack granted successfully".to_string(),
                    })
                })
            })),
        )
        .await?;

    let workflow = WorkflowDefinitionBuilder::new("cardgame.grant_pack", "Grant Booster Pack")
        .description("Fulfills a store purchase by granting a booster pack")
        .starting_server_action(&grant_pack)
        .add_node(
            "fulfilling",
            WorkflowNodeBuilder::new("Fulfilling purchase...").build(),
        )
        .build();
    let workflow_id = bot.register_workflow(workflow).await?;

    for product in [
        Product::new("booster-standard", "Standard Booster Pack", 5, &workflow_id)
            .description("10 cards with a chance at rare pulls")
            .payload(serde_json::json!({ "pack_type_id": "standard-pack" })),
        Product::new("booster-premium", "Premium Booster Pack", 12, &workflow_id)
            .description("5 high-quality cards with a guaranteed rare")
            .payload(serde_json::json!({ "pack_type_id": "premium-pack" })),
    ] {
        let product = product
            .product_type("booster_pack")
            .icon("package")
            .active(packs_on_sale());
        if let Err(error) = bot.register_product(&product).await {
            tracing::warn!(%error, product = %product.product_id, "store product was not registered");
        }
    }

    Ok(())
}

/// The packs stay off the store until CARDS_STORE=on.
fn packs_on_sale() -> bool {
    std::env::var("CARDS_STORE").is_ok_and(|value| value == "on")
}

/// Packs granted outside the store (manage_packs) have no purchase to settle.
async fn settle(bot: &TawksBot, purchase_id: &str, outcome: Result<&str, &str>) {
    if !purchase_id.starts_with("PUR-") {
        return;
    }
    let result = match outcome {
        Ok(pack_instance_id) => bot.confirm_purchase(purchase_id, pack_instance_id).await,
        Err(reason) => bot.fail_purchase(purchase_id, reason).await,
    };
    if let Err(error) = result {
        tracing::error!(%error, %purchase_id, "store purchase was not settled");
    }
}
