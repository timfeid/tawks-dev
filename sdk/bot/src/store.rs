use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::TawksBot;

/// Something for sale in the Tawks store, priced in tickets. Each purchase
/// starts `workflow_id` for the buyer with `purchase_id` and
/// `fulfillment_payload` inputs; the bot delivers it, then calls
/// `confirm_purchase`, or `fail_purchase` to refund the buyer.
#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub product_id: String,
    pub name: String,
    pub description: String,
    pub price: i32,
    pub product_type: String,
    pub workflow_id: String,
    pub payload: Value,
    pub icon: Option<String>,
    pub active: bool,
}

impl Product {
    /// `workflow_id` is the id `register_workflow` returned.
    pub fn new(
        product_id: impl Into<String>,
        name: impl Into<String>,
        price: i32,
        workflow_id: impl Into<String>,
    ) -> Self {
        Self {
            product_id: product_id.into(),
            name: name.into(),
            description: String::new(),
            price,
            product_type: "general".into(),
            workflow_id: workflow_id.into(),
            payload: json!({}),
            icon: None,
            active: true,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn product_type(mut self, product_type: impl Into<String>) -> Self {
        self.product_type = product_type.into();
        self
    }

    /// Handed to the workflow as `fulfillment_payload`.
    pub fn payload(mut self, payload: Value) -> Self {
        self.payload = payload;
        self
    }

    /// A Lucide icon name.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// A game bought once, with `access_days` of online access. Tawks
    /// delivers it itself; the workflow is not started. Ask who owns it with
    /// `product_owners`.
    pub fn game_license(
        product_id: impl Into<String>,
        name: impl Into<String>,
        price: i32,
        access_days: i32,
        workflow_id: impl Into<String>,
    ) -> Self {
        Self::new(product_id, name, price, workflow_id)
            .product_type("game-license")
            .payload(json!({ "access_days": access_days }))
    }

    /// Adds `access_days` of online access to an owned `license`, from its
    /// expiry, or from now once it has lapsed. Only owners can buy it.
    pub fn access_pass(
        product_id: impl Into<String>,
        name: impl Into<String>,
        price: i32,
        license: &str,
        access_days: i32,
        workflow_id: impl Into<String>,
    ) -> Self {
        Self::new(product_id, name, price, workflow_id)
            .product_type("game-access")
            .payload(json!({ "access_days": access_days, "license": license }))
    }
}

/// Someone's hold on a game license.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProductOwner {
    pub user_id: String,
    pub owns: bool,
    /// RFC 3339.
    pub access_expires_at: Option<String>,
    pub online_access: bool,
}

impl TawksBot {
    pub async fn register_product(&self, product: &Product) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, Value>("store_register_product", product)
            .await?;
        tracing::info!("Registered store product {}", product.product_id);
        Ok(())
    }

    /// `reference` names what was delivered, such as the granted item's id.
    /// Confirming again with the same reference is harmless.
    pub async fn confirm_purchase(&self, purchase_id: &str, reference: &str) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, Value>(
                "store_confirm_fulfillment",
                &json!({ "purchase_id": purchase_id, "fulfillment_ref": reference }),
            )
            .await?;
        Ok(())
    }

    /// For a game license this bot sells: each of `user_ids` (at most 500),
    /// owner or not, in the order asked.
    pub async fn product_owners(
        &self,
        product_id: &str,
        user_ids: &[String],
    ) -> anyhow::Result<Vec<ProductOwner>> {
        self.sdk
            .web_client
            .query(
                "store_product_owners",
                &json!({ "product_id": product_id, "user_ids": user_ids }),
            )
            .await
    }

    /// Every current owner of a game license this bot sells, a page at a time
    /// in user id order: pass the last `user_id` seen as `after`.
    pub async fn list_product_owners(
        &self,
        product_id: &str,
        after: Option<&str>,
        limit: i32,
    ) -> anyhow::Result<Vec<ProductOwner>> {
        self.sdk
            .web_client
            .query(
                "store_product_owners",
                &json!({ "product_id": product_id, "after": after, "limit": limit }),
            )
            .await
    }

    /// Refunds a delivered game license or access pass: the tickets go back,
    /// a license is revoked and a pass's days come off the expiry.
    pub async fn refund_purchase(&self, purchase_id: &str, reason: &str) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, Value>(
                "store_refund_purchase",
                &json!({ "purchase_id": purchase_id, "reason": reason }),
            )
            .await?;
        Ok(())
    }

    /// Refunds the buyer's tickets.
    pub async fn fail_purchase(&self, purchase_id: &str, error: &str) -> anyhow::Result<()> {
        self.sdk
            .web_client
            .mutate::<_, Value>(
                "store_fail_fulfillment",
                &json!({ "purchase_id": purchase_id, "error": error }),
            )
            .await?;
        Ok(())
    }
}
