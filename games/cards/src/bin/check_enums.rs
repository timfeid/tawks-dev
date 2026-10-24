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

    println!("\n=== Checking Enum Types ===\n");

    // Check stat_type enum values
    let stat_types = sqlx::query(
        r#"
        SELECT e.enumlabel
        FROM pg_enum e
        JOIN pg_type t ON e.enumtypid = t.oid
        WHERE t.typname = 'stat_type'
        ORDER BY e.enumsortorder
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!("stat_type enum values:");
    for row in stat_types {
        let label: String = row.try_get("enumlabel")?;
        println!("  - {}", label);
    }

    Ok(())
}
