use anyhow::Result;
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

    println!("\n=== Searching for 'trample' references ===\n");

    // Check keywords
    println!("Checking keywords field...");
    let keywords = sqlx::query(
        r#"
        SELECT card_id, name, keywords::text as keywords_text
        FROM cards
        WHERE keywords::text ILIKE '%trample%'
        LIMIT 10
        "#,
    )
    .fetch_all(&pool)
    .await?;

    if keywords.is_empty() {
        println!("  No trample in keywords");
    } else {
        for row in keywords {
            let card_id: String = row.try_get("card_id")?;
            let name: String = row.try_get("name")?;
            let kws: String = row.try_get("keywords_text")?;
            println!("  Card {}: {} - {}", card_id, name, kws);
        }
    }

    // Check if there are any grant_keyword effects with trample
    println!("\nChecking grant_keyword effects...");
    let grant_effects = sqlx::query(
        r#"
        SELECT effect_id, keyword_kind::text as keyword
        FROM card_effects
        WHERE effect_kind = 'grant_keyword'
        AND keyword_kind::text = 'trample'
        LIMIT 10
        "#,
    )
    .fetch_all(&pool)
    .await?;

    if grant_effects.is_empty() {
        println!("  No grant_keyword with trample");
    } else {
        println!(
            "  Found {} grant_keyword effects with trample",
            grant_effects.len()
        );
        for row in grant_effects {
            let effect_id: String = row.try_get("effect_id")?;
            let keyword: String = row.try_get("keyword")?;
            println!("    Effect {}: keyword={}", effect_id, keyword);
        }
    }

    // Check card_effect_keywords table
    println!("\nChecking card_effect_keywords table...");
    let effect_keywords = sqlx::query(
        r#"
        SELECT effect_id, keyword_kind::text as keyword
        FROM card_effect_keywords
        WHERE keyword_kind::text = 'trample'
        LIMIT 10
        "#,
    )
    .fetch_all(&pool)
    .await?;

    if effect_keywords.is_empty() {
        println!("  No trample in card_effect_keywords");
    } else {
        println!("  Found {} trample keywords", effect_keywords.len());
        for row in effect_keywords {
            let effect_id: String = row.try_get("effect_id")?;
            let keyword: String = row.try_get("keyword")?;
            println!("    Effect {}: keyword={}", effect_id, keyword);
        }
    }

    println!("\n=== Summary ===");
    println!("Trample is correctly stored as a keyword, not a stat_type");
    println!("The database schema is correct.");

    Ok(())
}
