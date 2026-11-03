//! Inspect bot scoring in Stackbreak Yard through the production form path.
//! cargo run --release -p party --example stackbreak_audit -- [games] [seed offset]
use party::{
    bout::{Bout, Contestant, on_form_scaled},
    stackbreak_yard::StackbreakYard,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let offset: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(140_000);
    for (top, field) in [(1., 0.7), (0.7, 0.4)] {
        let mut wins = 0.;
        let mut star_points = 0.;
        let mut field_points = 0.;
        let mut star_blocks = 0.;
        let mut field_blocks = 0.;
        let mut star_shoves = 0.;
        let mut field_shoves = 0.;
        for n in 0..games {
            let seed = 10_000 + offset + n;
            let star = ((n / 4) % 4) as u8;
            let seats: Vec<_> = (0..4)
                .map(|seat| Contestant::bot(seat, if seat == star { top } else { field }))
                .collect();
            let mut game = StackbreakYard::new(&on_form_scaled(&seats, seed, 1.), seed);
            let mut events = Vec::new();
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, game.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut events);
                for event in &events {
                    if event["kind"] == "stack_shove" {
                        if event["seat"] == star {
                            star_shoves += 1.;
                        } else {
                            field_shoves += 1.;
                        }
                    }
                }
                events.clear();
            }
            let first = &game.placements().unwrap()[0];
            if first.contains(&star) {
                wins += 1. / first.len() as f64;
            }
            for player in game.snapshot()["players"].as_array().unwrap() {
                let seat = player["seat"].as_u64().unwrap() as u8;
                let points = player["points"].as_f64().unwrap();
                let blocks = player["blocks"].as_f64().unwrap();
                if seat == star {
                    star_points += points;
                    star_blocks += blocks;
                } else {
                    field_points += points;
                    field_blocks += blocks;
                }
            }
        }
        let n = games as f64;
        println!(
            "{top:.1} vs {field:.1}: win {:.0}% points {:.1}/{:.1} blocks {:.1}/{:.1} shoves {:.1}/{:.1}",
            wins * 100. / n,
            star_points / n,
            field_points / (n * 3.),
            star_blocks / n,
            field_blocks / (n * 3.),
            star_shoves / n,
            field_shoves / (n * 3.),
        );
    }
}
