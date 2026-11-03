//! Production-path seeded Pigeon Post bot audit.
use party::bout::{Contestant, TICK_DT};
fn main() {
    let base = std::env::args()
        .nth(1)
        .and_then(|a| a.parse::<u64>().ok())
        .unwrap_or(60_000);
    for (favorite, field) in [(1.0, 0.7), (0.7, 0.4), (0.4, 0.4)] {
        let mut wins = [0u32; 4];
        for seed in 0..200 {
            let players = (0..4)
                .map(|s| Contestant::bot(s, if s == 0 { favorite } else { field }))
                .collect::<Vec<_>>();
            let mut game = party::bout::create(party::pigeon::KEY, &players, base + seed).unwrap();
            while game.placements().is_none() {
                let inputs = (0..4)
                    .map(|s| (s, game.bot_input(s, TICK_DT)))
                    .collect::<Vec<_>>();
                game.step(&inputs, TICK_DT, &mut vec![]);
            }
            for &seat in &game.placements().unwrap()[0] {
                wins[seat as usize] += 1;
            }
        }
        println!("favorite {favorite:.1} vs {field:.1}: first-place shares {wins:?}");
    }
}
