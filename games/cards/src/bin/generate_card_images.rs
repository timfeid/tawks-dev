use anyhow::Result;
use cardgame::art::ArtStudio;
use cardgame::card::Card;
use cardgame::repository::{CardArgs, CardCursor, CardRepository};
use std::sync::Arc;
use tawks_data::database::create_connection;
use tawks_data::pagination::PaginationArgs;
use tawks_data::repository::Repository;

fn main_color_for_card(card: &Card) -> &'static str {
    if card.cost.neon > 0 {
        "Neon Cyan"
    } else if card.cost.glitch > 0 {
        "Glitch Purple"
    } else if card.cost.chrome > 0 {
        "Chrome Silver"
    } else if card.cost.slag > 0 {
        "Slag Orange"
    } else if card.cost.bio > 0 {
        "Bio Green"
    } else {
        "Monochrome"
    }
}

fn prompts_for_card(card: &Card) -> Vec<String> {
    let main_color = main_color_for_card(card);
    vec![
        format!(
            "A borderless, full-frame digital concept art illustration of '{} {:?}'. Theme: Cyberpunk, {} color palette, high-tech futuristic aesthetic, neon lighting, highly detailed, cinematic. no text.",
            card.name, card.kind, main_color
        ),
        format!(
            "A borderless, full-frame digital concept art illustration. Theme: Cyberpunk, {} color palette, high-tech futuristic aesthetic, neon lighting, highly detailed, cinematic. no text.",
            main_color
        ),
    ]
}

async fn generate_one(card: &Card, studio: &ArtStudio, repo: &CardRepository) {
    let prompts = prompts_for_card(card);

    let mut generated_url: Option<String> = None;
    for (i, prompt) in prompts.iter().enumerate() {
        if i > 0 {
            println!("  [{}] Retrying with fallback prompt...", card.name);
        }
        println!("[{}] Prompt: {}", card.name, prompt);
        match studio.generate(prompt).await {
            Ok(url) => {
                generated_url = Some(url);
                break;
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("safety") {
                    eprintln!("  [{}] Safety rejection, trying next prompt...", card.name);
                } else {
                    eprintln!("[{}] Failed to generate: {}", card.name, msg);
                    break;
                }
            }
        }
    }

    if let Some(temp_url) = generated_url {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let filename = format!("cards/{}_{}.png", card.id, timestamp);
        println!("[{}] Uploading {}", card.name, filename);

        match studio.store(&temp_url, &filename).await {
            Ok(final_url) => {
                let mut updated_card = card.clone();
                updated_card.image_url = Some(final_url.clone());
                if let Err(e) = repo.update_card(&updated_card).await {
                    eprintln!("[{}] DB update failed: {}", card.name, e);
                } else {
                    println!("done [{}] -> {}", card.name, final_url);
                }
            }
            Err(e) => eprintln!("[{}] Failed to store: {}", card.name, e),
        }
    } else {
        eprintln!("[{}] All prompts rejected, skipping.", card.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    //
    let cardgame_database_url =
        std::env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL is required");

    let pool = create_connection(&cardgame_database_url).await;
    let repo = Arc::new(CardRepository::new(pool.clone()));
    let studio = Arc::new(ArtStudio::from_env()?);

    println!("Fetching cards from database...");

    let pagination = PaginationArgs {
        before: None,
        after: None,
        first: Some(1000),
        last: None,
    };

    let cards: Vec<Card> = repo
        .find(
            None::<(tawks_data::repository::CursorDirection, CardCursor)>,
            1000,
            &CardArgs {
                pagination,
                owner_id: None,
            },
        )
        .await?;

    // Filter to cards that need images
    let pending: Vec<&Card> = cards
        .iter()
        .filter(|c| !studio.is_stored(c.image_url.as_deref()))
        .collect();

    println!(
        "Found {} cards, {} need images.",
        cards.len(),
        pending.len()
    );

    // Process in batches of 3 concurrently
    for chunk in pending.chunks(3) {
        let mut handles = Vec::new();
        for card in chunk {
            let card = (*card).clone();
            let studio = Arc::clone(&studio);
            let repo = Arc::clone(&repo);
            handles.push(tokio::spawn(async move {
                generate_one(&card, &studio, &repo).await;
            }));
        }
        for handle in handles {
            if let Err(e) = handle.await {
                eprintln!("Task panicked: {}", e);
            }
        }
    }

    println!("--------------------------------------------------");
    println!("Done!");

    Ok(())
}
