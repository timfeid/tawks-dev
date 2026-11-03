//! Quick seeded skill and seat audit for Mirth Chain.
//! cargo run -p party --example mirth_balance -- [games]
use party::bout::{Contestant, TICK_DT, create};

fn run(games: u64, favorite_skill: f32, field_skill: f32) {
    let mut wins = 0.0;
    let mut finished = 0;
    let mut favorite_score = 0.0;
    let mut field_score = 0.0;
    for index in 0..games {
        let favorite = (index % 4) as u8;
        let seats: Vec<_> = (0..4)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == favorite {
                    favorite_skill
                } else {
                    field_skill
                },
                team: seat % 2,
                ally: None,
            })
            .collect();
        let mut game = create("mirth", &seats, 41_000 + index).unwrap();
        let mut events = Vec::new();
        for _ in 0..(70.0 / TICK_DT) as usize {
            let inputs: Vec<_> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            events.clear();
            if let Some(order) = game.placements() {
                finished += 1;
                if order[0].contains(&favorite) {
                    wins += 1.0 / order[0].len() as f64;
                }
                let snapshot = game.snapshot();
                for player in snapshot["players"].as_array().unwrap() {
                    let score = player["score"].as_u64().unwrap() as f64;
                    if player["seat"].as_u64() == Some(favorite as u64) {
                        favorite_score += score;
                    } else {
                        field_score += score / 3.0;
                    }
                }
                break;
            }
        }
    }
    println!(
        "skills {favorite_skill:.1}/{field_skill:.1}: {finished}/{games} finished, {:.1}% favorite credit, scores {:.1}/{:.1}",
        wins * 100.0 / games as f64,
        favorite_score / games as f64,
        field_score / games as f64,
    );
}
fn main() {
    let games = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(160);
    for (favorite, field) in [(1.0, 0.7), (0.7, 0.4), (0.4, 0.4)] {
        run(games, favorite, field);
    }
}
