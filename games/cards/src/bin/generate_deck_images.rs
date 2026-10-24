use anyhow::Result;
use cardgame::art::ArtStudio;
use cardgame::repository::CardRepository;
use std::sync::Arc;
use tawks_data::database::create_connection;

fn prompt_for_deck(name: &str, colors: &[String]) -> Vec<String> {
    let color_str = colors.join(", ");
    vec![
        format!(
            "A borderless, full-frame digital concept art illustration for '{}'. \
              Color palette: {}. \
             Style: Cyberpunk, high-tech futuristic aesthetic, neon lighting, cinematic composition, \
             highly detailed, no text, no card borders, suitable as a deck cover image.",
            name, color_str,
        ),
        format!(
            "A borderless, full-frame digital concept art illustration. \
             Theme: Cyberpunk trading card game deck. Color palette: {}. \
             High-tech futuristic aesthetic, neon lighting, cinematic, highly detailed, no text.",
            color_str,
        ),
    ]
}

fn colors_for_deck(repo_cards: &[(String, cardgame::types::ManaCost)]) -> Vec<String> {
    let mut colors = Vec::new();
    for (_, cost) in repo_cards {
        if cost.neon > 0 && !colors.contains(&"Neon Cyan".to_string()) {
            colors.push("Neon Cyan".to_string());
        }
        if cost.bio > 0 && !colors.contains(&"Bio Green".to_string()) {
            colors.push("Bio Green".to_string());
        }
        if cost.slag > 0 && !colors.contains(&"Slag Orange".to_string()) {
            colors.push("Slag Orange".to_string());
        }
        if cost.glitch > 0 && !colors.contains(&"Glitch Purple".to_string()) {
            colors.push("Glitch Purple".to_string());
        }
        if cost.chrome > 0 && !colors.contains(&"Chrome Silver".to_string()) {
            colors.push("Chrome Silver".to_string());
        }
    }
    if colors.is_empty() {
        colors.push("Monochrome".to_string());
    }
    colors
}

async fn generate_one(
    deck: &cardgame::repository::DeckRecord,
    colors: &[String],
    studio: &ArtStudio,
    repo: &CardRepository,
) {
    let prompts = prompt_for_deck(&deck.name, colors);

    let mut generated_url: Option<String> = None;
    for (i, prompt) in prompts.iter().enumerate() {
        if i > 0 {
            println!("  [{}] Retrying with fallback prompt...", deck.name);
        }
        println!("[{}] Prompt: {}", deck.name, prompt);
        match studio.generate(prompt).await {
            Ok(url) => {
                generated_url = Some(url);
                break;
            }
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("safety") {
                    eprintln!("  [{}] Safety rejection, trying next prompt...", deck.name);
                } else {
                    eprintln!("[{}] Failed to generate: {}", deck.name, msg);
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
        let filename = format!("decks/{}_{}.png", deck.id, timestamp);
        println!("[{}] Uploading {}", deck.name, filename);

        match studio.store(&temp_url, &filename).await {
            Ok(final_url) => {
                if let Err(e) = repo.update_deck_image_url(&deck.id, &final_url).await {
                    eprintln!("[{}] DB update failed: {}", deck.name, e);
                } else {
                    println!("done [{}] -> {}", deck.name, final_url);
                }
            }
            Err(e) => eprintln!("[{}] Failed to store: {}", deck.name, e),
        }
    } else {
        eprintln!("[{}] All prompts rejected, skipping.", deck.name);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cardgame_database_url =
        std::env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL is required");

    let pool = create_connection(&cardgame_database_url).await;
    let repo = Arc::new(CardRepository::new(pool.clone()));
    let studio = Arc::new(ArtStudio::from_env()?);

    println!("Fetching decks from database...");

    let decks = repo.list_base_decks().await?;

    // Filter to decks that need images
    let pending: Vec<&cardgame::repository::DeckRecord> = decks
        .iter()
        .filter(|d| !studio.is_stored(d.image_url.as_deref()))
        .collect();

    println!(
        "Found {} decks, {} need images.",
        decks.len(),
        pending.len()
    );

    // Process decks one at a time (only 5 decks, no need for batching)
    for deck in pending {
        // Get deck card costs for color computation
        let templates = repo.load_deck_templates(&deck.id).await.unwrap_or_default();
        let card_costs: Vec<(String, cardgame::types::ManaCost)> = templates
            .iter()
            .map(|t| (t.card.id.clone(), t.card.cost.clone()))
            .collect();
        let colors = colors_for_deck(&card_costs);

        generate_one(deck, &colors, &studio, &repo).await;
    }

    println!("--------------------------------------------------");
    println!("Done!");

    Ok(())
}
