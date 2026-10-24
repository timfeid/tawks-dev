use anyhow::Result;
use std::env;
use tawks_data::database::create_connection;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL is required");
    let pool = create_connection(&database_url).await;

    println!("Clearing all card image URLs...");
    sqlx::query("UPDATE cards SET image_url = NULL;")
        .execute(&*pool)
        .await?;

    println!("All card images cleared.");
    Ok(())
}
