use anyhow::Result;
use sqlx::Row;
use std::env;
use tawks_data::database::create_connection;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL is required");
    let pool = create_connection(&database_url).await;

    let rows = sqlx::query("SELECT name, image_url FROM cards WHERE image_url IS NOT NULL;")
        .fetch_all(&*pool)
        .await?;

    if rows.is_empty() {
        println!("No cards have image URLs.");
    } else {
        println!("Found {} cards with image URLs:", rows.len());
        for row in rows {
            let name: String = row.get("name");
            let url: String = row.get("image_url");
            println!("- {}: {}", name, url);
        }
    }
    Ok(())
}
