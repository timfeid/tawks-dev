use sqlx::PgPool;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = env::var("CARDGAME_DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    println!("Checking deck card counts...\n");

    let deck_ids = vec![
        ("roots_of_infinity", "Roots of Infinity"),
        ("flicker_strike", "Flicker Strike"),
        ("mind_rot", "Mind Rot"),
        ("blood_furnace", "Blood Furnace"),
    ];

    for (deck_id, deck_name) in &deck_ids {
        let result: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(quantity), 0) FROM deck_cards WHERE deck_id = $1")
                .bind(deck_id)
                .fetch_one(&pool)
                .await?;

        let total = result.0;
        let status = if total == 100 { "✓" } else { "✗" };
        println!("{} {}: {} cards", status, deck_name, total);

        if total != 100 {
            // Show breakdown
            let cards: Vec<(String, i32, Option<String>)> = sqlx::query_as(
                r#"
                SELECT c.name, dc.quantity, c.supertype::text
                FROM deck_cards dc
                JOIN cards c ON dc.card_id = c.card_id
                WHERE dc.deck_id = $1
                ORDER BY c.supertype DESC NULLS LAST
                "#,
            )
            .bind(deck_id)
            .fetch_all(&pool)
            .await?;

            let apex_count = cards
                .iter()
                .filter(|(_, _, supertype)| supertype.as_deref() == Some("apex"))
                .count();

            println!("  Apex cards: {}", apex_count);
            println!("  Total unique cards: {}", cards.len());

            // Show all apex cards
            println!("  Apex operators:");
            for (name, qty, _) in cards
                .iter()
                .filter(|(_, _, st)| st.as_deref() == Some("apex"))
            {
                println!("    - {} (x{})", name, qty);
            }
            println!();
        }
    }

    Ok(())
}
