use anyhow::Result;
use cardgame::types::StatType;
use sqlx::Row;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("\n=== Validating Card Data ===\n");

    // List all tables
    let tables = sqlx::query(
        r#"
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!("Available tables:");
    for row in &tables {
        let table_name: String = row.try_get("tablename")?;
        println!("  - {}", table_name);
    }
    println!();

    // Check card_effects table columns
    println!("\n=== Checking card_effects ===\n");

    let columns = sqlx::query(
        r#"
        SELECT column_name, data_type
        FROM information_schema.columns
        WHERE table_name = 'card_effects'
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!("card_effects columns:");
    for row in &columns {
        let col_name: String = row.try_get("column_name")?;
        let data_type: String = row.try_get("data_type")?;
        println!("  - {}: {}", col_name, data_type);
    }

    // Check for stat_type in card_effects using text casting
    let stat_effects = sqlx::query(
        r#"
        SELECT effect_id, effect_kind::text as effect_kind, stat_type::text as stat_type, stat_amount
        FROM card_effects
        WHERE stat_type IS NOT NULL
        LIMIT 20
        "#
    )
    .fetch_all(&pool)
    .await?;

    println!("\nEffects with stat_type:");
    let mut found_invalid = false;
    for row in stat_effects {
        let effect_id: String = row.try_get("effect_id")?;
        let effect_kind: String = row.try_get("effect_kind")?;
        let stat_type: Option<String> = row.try_get("stat_type")?;
        let stat_amount: Option<i32> = row.try_get("stat_amount")?;

        if let Some(st) = &stat_type {
            match serde_json::from_str::<StatType>(&format!("\"{}\"", st)) {
                Ok(_) => {
                    println!(
                        "  ✓ Effect {}: kind={}, stat_type={:?}, amount={:?}",
                        effect_id, effect_kind, st, stat_amount
                    );
                }
                Err(e) => {
                    println!(
                        "  ✗ Effect {}: kind={}, stat_type={:?} - Parse error: {}",
                        effect_id, effect_kind, st, e
                    );
                    found_invalid = true;
                }
            }
        }
    }

    if !found_invalid {
        println!("\n✓ All stat_type values are valid!");
    } else {
        println!("\n✗ Found invalid stat_type values!");
    }

    Ok(())
}
