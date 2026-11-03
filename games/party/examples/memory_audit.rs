//! Diagnose Memory Tiles bot losses through the same bout factory as rooms.
//! cargo run --release -p party --example memory_audit -- [games]
use party::bout::{Contestant, TICK_DT, create};

#[derive(Default)]
struct Tally {
    games: u32,
    credit: f64,
    tied_first: u32,
    star_wrong: u32,
    star_timeout: u32,
    field_wrong: u32,
    field_timeout: u32,
    star_survived: u32,
}

fn run(games: u64, top: f32, field: f32, offset: u64) -> Tally {
    let mut tally = Tally::default();
    for index in 0..games {
        let star = (index % 4) as u8;
        let seats: Vec<_> = (0..4)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == star { top } else { field },
                team: seat % 2,
                ally: None,
            })
            .collect();
        let mut game = create("memory", &seats, 10_000 + offset + index).unwrap();
        let mut reason = [None; 4];
        let mut events = Vec::new();
        for _ in 0..(150.0 / TICK_DT) as usize {
            let inputs: Vec<_> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            for event in events.drain(..) {
                let Some(seat) = event["seat"].as_u64().map(|seat| seat as usize) else {
                    continue;
                };
                if reason[seat].is_none() {
                    reason[seat] = match event["kind"].as_str() {
                        Some("wrong") => Some("wrong"),
                        Some("timeout") => Some("timeout"),
                        _ => None,
                    };
                }
            }
            if let Some(order) = game.placements() {
                tally.games += 1;
                if order[0].contains(&star) {
                    tally.credit += 1.0 / order[0].len() as f64;
                    tally.tied_first += u32::from(order[0].len() > 1);
                }
                tally.star_survived += u32::from(reason[star as usize].is_none());
                for (seat, outcome) in reason.into_iter().enumerate() {
                    match (seat as u8 == star, outcome) {
                        (true, Some("wrong")) => tally.star_wrong += 1,
                        (true, Some("timeout")) => tally.star_timeout += 1,
                        (false, Some("wrong")) => tally.field_wrong += 1,
                        (false, Some("timeout")) => tally.field_timeout += 1,
                        _ => {}
                    }
                }
                break;
            }
        }
    }
    tally
}

fn main() {
    let games = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(500);
    let offset = std::env::var("FORM_BALANCE_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    for (name, top, field) in [("top", 1.0, 0.7), ("mid", 0.7, 0.4)] {
        let tally = run(games, top, field, offset);
        println!(
            "{name}: {}/{} finished, {:.1}% favorite wins, {} first-place ties, star wrong/timeout/survived {}/{}/{}, field wrong/timeout {}/{}",
            tally.games,
            games,
            tally.credit * 100.0 / games as f64,
            tally.tied_first,
            tally.star_wrong,
            tally.star_timeout,
            tally.star_survived,
            tally.field_wrong,
            tally.field_timeout,
        );
    }
}
