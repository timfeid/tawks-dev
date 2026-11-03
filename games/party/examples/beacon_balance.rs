//! Seeded server-only Beacon Bloom bot audit.
use party::bout::{Contestant, TICK_DT};

fn main() {
    let seed_base = std::env::args()
        .nth(1)
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60_000);
    for (favorite, field) in [(1.0, 0.7), (0.7, 0.4), (0.4, 0.4)] {
        let mut wins = [0u32; 4];
        let mut total = 0u64;
        for seed in 0..200u64 {
            let contestants = (0..4)
                .map(|seat| Contestant::bot(seat, if seat == 0 { favorite } else { field }))
                .collect::<Vec<_>>();
            let mut game =
                party::bout::create(party::beacon::KEY, &contestants, seed + seed_base).unwrap();
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
                    .collect::<Vec<_>>();
                game.step(&inputs, TICK_DT, &mut vec![]);
            }
            for &seat in &game.placements().unwrap()[0] {
                wins[seat as usize] += 1;
            }
            total += game.snapshot()["players"][0]["score"].as_u64().unwrap_or(0);
        }
        println!(
            "favorite {favorite:.1} vs {field:.1}: wins {wins:?}, favorite mean score {:.1}",
            total as f32 / 200.0
        );
    }
}
