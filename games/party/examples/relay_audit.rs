//! Diagnose Bomb Relay bot passes and outcomes with production bout form.
//! cargo run --release -p party --example relay_audit -- [games]
use party::{
    bout::{Bout, Contestant, TICK_DT, on_form},
    relay::BombRelay,
};

#[derive(Default)]
struct Tally {
    finished: u32,
    favorite_credit: f64,
    first_carrier: u32,
    first_carrier_credit: f64,
    favorite_passes: u32,
    field_passes: u32,
    favorite_carrier_seconds: f32,
    field_carrier_seconds: f32,
    favorite_first_boom: u32,
}

fn run(games: u64, top: f32, field: f32, offset: u64) -> Tally {
    let mut tally = Tally::default();
    for index in 0..games {
        let favorite = (index % 4) as u8;
        let seats: Vec<_> = (0..4)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == favorite { top } else { field },
                team: seat % 2,
                ally: None,
            })
            .collect();
        let seed = 10_000 + offset + index;
        let mut game = BombRelay::new(&on_form(&seats, seed), seed);
        let began_favorite = game.holder == Some(favorite);
        tally.first_carrier += u32::from(began_favorite);
        let mut events = Vec::new();
        let mut first_boom = None;
        for _ in 0..(100.0 / TICK_DT) as usize {
            match game.holder {
                Some(seat) if seat == favorite => tally.favorite_carrier_seconds += TICK_DT,
                Some(_) => tally.field_carrier_seconds += TICK_DT,
                None => {}
            }
            let inputs: Vec<_> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            for event in events.drain(..) {
                if event["kind"] == "boom" && first_boom.is_none() {
                    first_boom = event["seat"].as_u64().map(|seat| seat as u8);
                }
            }
            if let Some(order) = game.placements() {
                tally.finished += 1;
                if order[0].contains(&favorite) {
                    let credit = 1.0 / order[0].len() as f64;
                    tally.favorite_credit += credit;
                    if began_favorite {
                        tally.first_carrier_credit += credit;
                    }
                }
                tally.favorite_first_boom += u32::from(first_boom == Some(favorite));
                for player in &game.players {
                    if player.seat == favorite {
                        tally.favorite_passes += player.passes;
                    } else {
                        tally.field_passes += player.passes;
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
            "{name}: {}/{} finished, {:.1}% favorite wins, first carrier {}/{}, {:.1}% wins when first and {:.1}% otherwise; passes favorite/field {}/{}, carrier seconds favorite/field {:.0}/{:.0}, favorite first boom {}",
            tally.finished,
            games,
            tally.favorite_credit * 100.0 / games as f64,
            tally.first_carrier,
            games,
            tally.first_carrier_credit * 100.0 / tally.first_carrier.max(1) as f64,
            (tally.favorite_credit - tally.first_carrier_credit) * 100.0
                / (games - u64::from(tally.first_carrier)).max(1) as f64,
            tally.favorite_passes,
            tally.field_passes,
            tally.favorite_carrier_seconds,
            tally.field_carrier_seconds,
            tally.favorite_first_boom,
        );
    }
}
