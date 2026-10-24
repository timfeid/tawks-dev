use anyhow::Result;
use cardgame::repository::CardRepository;
use std::env;
use std::sync::Arc;
use ulid::Ulid;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let _ = dotenvy::dotenv();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "seed-defaults" => seed_defaults().await?,
        "list-types" => list_types().await?,
        "sync-catalog" => sync_catalog(&args[2..]).await?,
        "grant" => {
            let user_id = args
                .get(2)
                .expect("Usage: manage_packs grant <user_id> <pack_type_id> [count]");
            let pack_type_id = args
                .get(3)
                .expect("Usage: manage_packs grant <user_id> <pack_type_id> [count]");
            let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1);
            grant_packs(user_id, pack_type_id, count).await?;
        }
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"
Pack Management Tool

USAGE:
    manage_packs <COMMAND>

COMMANDS:
    seed-defaults                               Create default pack types + store products
    list-types                                  List all pack types
    grant <user_id> <pack_type_id> [count]      Grant packs directly to a user
    sync-catalog [card_id ...]                  Write catalog cards (all, or the ones named) to the database

ENVIRONMENT:
    CARDGAME_DATABASE_URL    PostgreSQL connection string (cardgame DB)
    TAWKS_DATABASE_URL       Optional tawks database, for store products
"#
    );
}

fn cardgame_pool() -> impl std::future::Future<Output = Result<sqlx::PgPool>> {
    async {
        let url = env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        Ok(pool)
    }
}

/// The tawks database holds the store, which a playground does not have.
async fn tawks_pool() -> Result<Option<sqlx::PgPool>> {
    let Ok(url) = env::var("TAWKS_DATABASE_URL") else {
        return Ok(None);
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await?;
    Ok(Some(pool))
}

/// The server only seeds decks it has never seen, so an edited catalog card
/// reaches an existing database through this.
async fn sync_catalog(ids: &[String]) -> Result<()> {
    use cardgame::card_catalog::*;
    let repo = CardRepository::new(Arc::new(cardgame_pool().await?));
    let cards = [
        bio::all_cards(),
        chrome::all_cards(),
        glitch::all_cards(),
        multicolor::all_cards(),
        neon::all_cards(),
        slag::all_cards(),
        syndicate::all_cards(),
    ];
    let mut written = 0;
    for (card, _) in cards.into_iter().flatten() {
        if ids.is_empty() || ids.contains(&card.id) {
            repo.create_card(&card).await?;
            written += 1;
            println!("synced {}", card.id);
        }
    }
    println!("{written} cards written");
    Ok(())
}

async fn seed_defaults() -> Result<()> {
    println!("Connecting to databases...");
    let cg_pool = cardgame_pool().await?;
    let tw_pool = tawks_pool().await?;

    println!("Seeding default pack types...\n");

    // Standard Pack: 10 slots (6 Public, 3 Private, 1 Restricted+)
    let standard_id = "standard-pack";
    upsert_pack_type(
        &cg_pool,
        standard_id,
        "Standard Pack",
        "10 cards with a chance at rare pulls",
    )
    .await?;
    let standard_slots: Vec<(i32, i32, i32, i32, i32, i32)> = vec![
        // (slot_index, public, private, restricted, classified, singularity)
        (0, 80, 15, 4, 1, 0),
        (1, 80, 15, 4, 1, 0),
        (2, 80, 15, 4, 1, 0),
        (3, 80, 15, 4, 1, 0),
        (4, 80, 15, 4, 1, 0),
        (5, 80, 15, 4, 1, 0),
        (6, 30, 50, 15, 4, 1),
        (7, 30, 50, 15, 4, 1),
        (8, 30, 50, 15, 4, 1),
        (9, 0, 20, 50, 25, 5), // guaranteed uncommon+
    ];
    upsert_pack_slots(&cg_pool, standard_id, &standard_slots).await?;
    println!("  Standard Pack: 10 slots");

    // Draft Pack: 19 slots with 2 guaranteed rare+ pulls, not sold in the store
    let draft_id = "draft-pack";
    upsert_pack_type(
        &cg_pool,
        draft_id,
        "Draft Pack",
        "19-card draft-only pack with 1 guaranteed rare+ pulls and 1 uncommon+",
    )
    .await?;
    let draft_slots: Vec<(i32, i32, i32, i32, i32, i32)> = vec![
        (0, 80, 15, 4, 1, 0),
        (1, 80, 15, 4, 1, 0),
        (2, 80, 15, 4, 1, 0),
        (3, 80, 15, 4, 1, 0),
        (4, 80, 15, 4, 1, 0),
        (5, 80, 15, 4, 1, 0),
        (6, 80, 15, 4, 1, 0),
        (7, 80, 15, 4, 1, 0),
        (8, 80, 15, 4, 1, 0),
        (9, 80, 15, 4, 1, 0),
        (10, 80, 15, 4, 1, 0),
        (11, 30, 50, 15, 4, 1),
        (12, 30, 50, 15, 4, 1),
        (13, 30, 50, 15, 4, 1),
        (14, 30, 50, 15, 4, 1),
        (15, 30, 50, 15, 4, 1),
        (16, 30, 50, 15, 4, 1),
        (17, 0, 20, 50, 25, 5), // guaranteed uncommon+
        (18, 0, 0, 65, 28, 7),  // guaranteed rare+
    ];
    upsert_pack_slots(&cg_pool, draft_id, &draft_slots).await?;
    println!("  Draft Pack: 19 slots");

    // Premium Pack: 5 slots (1 Public, 2 Private, 1 Restricted, 1 Classified+)
    let premium_id = "premium-pack";
    upsert_pack_type(
        &cg_pool,
        premium_id,
        "Premium Pack",
        "5 high-quality cards with guaranteed rare",
    )
    .await?;
    let premium_slots: Vec<(i32, i32, i32, i32, i32, i32)> = vec![
        (0, 40, 40, 15, 4, 1),
        (1, 10, 50, 30, 8, 2),
        (2, 10, 50, 30, 8, 2),
        (3, 0, 15, 50, 30, 5),
        (4, 0, 0, 30, 50, 20), // guaranteed rare+
    ];
    upsert_pack_slots(&cg_pool, premium_id, &premium_slots).await?;
    println!("  Premium Pack: 5 slots");

    // Starter Pack: 15 slots (all Public, reward-only)
    let starter_id = "starter-pack";
    upsert_pack_type(
        &cg_pool,
        starter_id,
        "Starter Pack",
        "15 common cards to get you started",
    )
    .await?;
    let starter_slots: Vec<(i32, i32, i32, i32, i32, i32)> = vec![
        (0, 100, 0, 0, 0, 0),
        (1, 100, 0, 0, 0, 0),
        (2, 100, 0, 0, 0, 0),
        (3, 100, 0, 0, 0, 0),
        (4, 100, 0, 0, 0, 0),
        (5, 100, 0, 0, 0, 0),
        (6, 100, 0, 0, 0, 0),
        (7, 100, 0, 0, 0, 0),
        (8, 100, 0, 0, 0, 0),
        (9, 100, 0, 0, 0, 0),
        (10, 100, 0, 0, 0, 0),
        (11, 100, 0, 0, 0, 0),
        (12, 100, 0, 0, 0, 0),
        (13, 100, 0, 0, 0, 0),
        (14, 100, 0, 0, 0, 0),
    ];
    upsert_pack_slots(&cg_pool, starter_id, &starter_slots).await?;
    println!("  Starter Pack: 15 slots");

    let Some(tw_pool) = tw_pool else {
        println!("\nTAWKS_DATABASE_URL is not set; skipping store products.");
        return Ok(());
    };

    // Only Standard and Premium are purchasable.
    println!("\nSeeding store products...\n");

    upsert_store_product(
        &tw_pool,
        "booster-standard",
        "Standard Booster Pack",
        "10 cards with a chance at rare pulls",
        100,
        "booster_pack",
        "cardgame.grant_pack",
        serde_json::json!({"pack_type_id": "standard-pack"}),
    )
    .await?;
    println!("  Standard Booster: 100 score");

    upsert_store_product(
        &tw_pool,
        "booster-premium",
        "Premium Booster Pack",
        "5 high-quality cards with guaranteed rare",
        300,
        "booster_pack",
        "cardgame.grant_pack",
        serde_json::json!({"pack_type_id": "premium-pack"}),
    )
    .await?;
    println!("  Premium Booster: 300 score");

    println!("\nDone! Pack types and store products seeded.");
    Ok(())
}

async fn upsert_pack_type(
    pool: &sqlx::PgPool,
    id: &str,
    name: &str,
    description: &str,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO pack_types (pack_type_id, name, description)
        VALUES ($1, $2, $3)
        ON CONFLICT (pack_type_id) DO UPDATE SET name = $2, description = $3, updated_at = now()
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_pack_slots(
    pool: &sqlx::PgPool,
    pack_type_id: &str,
    slots: &[(i32, i32, i32, i32, i32, i32)],
) -> Result<()> {
    // Delete existing slots for this pack type, then re-insert
    sqlx::query("DELETE FROM pack_slots WHERE pack_type_id = $1")
        .bind(pack_type_id)
        .execute(pool)
        .await?;

    for (slot_index, public, private, restricted, classified, singularity) in slots {
        let slot_id = format!("{}-slot-{}", pack_type_id, slot_index);
        sqlx::query(
            r#"
            INSERT INTO pack_slots (slot_id, pack_type_id, slot_index, weight_public, weight_private, weight_restricted, weight_classified, weight_singularity)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&slot_id)
        .bind(pack_type_id)
        .bind(slot_index)
        .bind(public)
        .bind(private)
        .bind(restricted)
        .bind(classified)
        .bind(singularity)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn upsert_store_product(
    pool: &sqlx::PgPool,
    product_id: &str,
    name: &str,
    description: &str,
    price: i32,
    product_type: &str,
    fulfillment_action: &str,
    fulfillment_payload: serde_json::Value,
) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO store_products (product_id, name, description, price, product_type, fulfillment_action, fulfillment_payload)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (product_id) DO UPDATE
        SET name = $2, description = $3, price = $4, product_type = $5,
            fulfillment_action = $6, fulfillment_payload = $7, updated_at = now()
        "#,
    )
    .bind(product_id)
    .bind(name)
    .bind(description)
    .bind(price)
    .bind(product_type)
    .bind(fulfillment_action)
    .bind(&fulfillment_payload)
    .execute(pool)
    .await?;
    Ok(())
}

async fn list_types() -> Result<()> {
    let pool = cardgame_pool().await?;
    let repo = CardRepository::new(Arc::new(pool));

    let pack_types = repo.list_pack_types().await?;

    if pack_types.is_empty() {
        println!("No pack types found. Run 'seed-defaults' first.");
        return Ok(());
    }

    for pt in &pack_types {
        println!(
            "{} - {} ({})",
            pt.pack_type_id,
            pt.name,
            if pt.active { "active" } else { "inactive" }
        );
        println!("  {}", pt.description);
        println!("  Slots: {}", pt.slots.len());
        for slot in &pt.slots {
            println!(
                "    Slot {}: Public={}, Private={}, Restricted={}, Classified={}, Singularity={}",
                slot.slot_index,
                slot.weight_public,
                slot.weight_private,
                slot.weight_restricted,
                slot.weight_classified,
                slot.weight_singularity,
            );
        }
        println!();
    }

    Ok(())
}

async fn grant_packs(user_id: &str, pack_type_id: &str, count: usize) -> Result<()> {
    let pool = cardgame_pool().await?;
    let repo = CardRepository::new(Arc::new(pool));

    // Verify pack type exists
    let pack_type = repo
        .get_pack_type(pack_type_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pack type '{}' not found", pack_type_id))?;

    println!(
        "Granting {} x '{}' to user {}...",
        count, pack_type.name, user_id
    );

    for i in 0..count {
        let pack_instance_id = Ulid::new().to_string();
        let pack = cardgame::pack::PackInstance {
            pack_instance_id: pack_instance_id.clone(),
            pack_type_id: pack_type_id.to_string(),
            owner_id: user_id.to_string(),
            source: "admin_cli".to_string(),
            source_ref: None,
            opened: false,
            opened_at: None,
            created_at: chrono::Utc::now(),
        };

        repo.create_pack_instance(&pack).await?;
        repo.create_pack_audit(
            &pack_instance_id,
            "granted",
            user_id,
            Some(serde_json::json!({
                "source": "admin_cli",
                "pack_type_id": pack_type_id,
            })),
        )
        .await?;

        println!("  [{}/{}] Created pack {}", i + 1, count, pack_instance_id);
    }

    println!("Done!");
    Ok(())
}
