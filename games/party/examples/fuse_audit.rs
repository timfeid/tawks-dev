//! Inspect Overcooked Fuse bot outcomes through the room's form path.
//! cargo run --release -p party --example fuse_audit -- [games] [seed offset]
use party::{
    bout::{Bout, Contestant, on_form_scaled},
    fuse::OvercookedFuse,
};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let offset: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(160_000);
    for (top, field) in [(1., 0.7), (0.7, 0.4)] {
        let mut wins = 0.;
        let mut star_score = 0.;
        let mut field_score = 0.;
        let mut star_booms = 0.;
        let mut field_booms = 0.;
        let mut star_swats = 0.;
        let mut field_swats = 0.;
        for n in 0..games {
            let seed = 10_000 + offset + n;
            let star = ((n / 4) % 4) as u8;
            let seats: Vec<_> = (0..4)
                .map(|seat| Contestant::bot(seat, if seat == star { top } else { field }))
                .collect();
            let mut game = OvercookedFuse::new(&on_form_scaled(&seats, seed, 1.), seed);
            let mut events = Vec::new();
            while game.placements().is_none() {
                let inputs = seats
                    .iter()
                    .map(|s| (s.seat, game.bot_input(s.seat, 1. / 60.)))
                    .collect::<Vec<_>>();
                game.step(&inputs, 1. / 60., &mut events);
                events.clear();
            }
            let first = &game.placements().unwrap()[0];
            if first.contains(&star) {
                wins += 1. / first.len() as f64;
            }
            for seat in &game.seats {
                if seat.seat == star {
                    star_score += f64::from(seat.score);
                    star_booms += f64::from(seat.booms);
                    star_swats += f64::from(seat.swats);
                } else {
                    field_score += f64::from(seat.score);
                    field_booms += f64::from(seat.booms);
                    field_swats += f64::from(seat.swats);
                }
            }
        }
        let n = games as f64;
        println!(
            "{top:.1} vs {field:.1}: win {:.0}% score {:.1}/{:.1} booms {:.2}/{:.2} swats {:.2}/{:.2}",
            wins * 100. / n,
            star_score / n,
            field_score / (n * 3.),
            star_booms / n,
            field_booms / (n * 3.),
            star_swats / n,
            field_swats / (n * 3.),
        );
    }
}
