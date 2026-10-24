use anyhow::Result;
use sqlx::{PgPool, Row};

#[tokio::main]
async fn main() -> Result<()> {
    let db_url = std::env::var("CARDGAME_DATABASE_URL")?;
    let pool = PgPool::connect(&db_url).await?;

    let rows = sqlx::query(
        "SELECT conname, pg_get_constraintdef(c.oid) FROM pg_constraint c JOIN pg_namespace n ON n.oid = c.connamespace WHERE n.nspname = 'public' AND conrelid = 'cards'::regclass"
    ).fetch_all(&pool).await?;

    for row in rows {
        let name: String = row.get(0);
        let def: String = row.get(1);
        println!("{}: {}", name, def);
    }
    Ok(())
}
