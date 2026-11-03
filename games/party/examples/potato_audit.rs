//! Inspect Hot Potato wins and unavoidable late handoffs at production bot form.
//! cargo run --release -p party --example potato_audit -- [games] [seed offset] [form scale]
use party::{
    bout::{Bout, Contestant, Input, on_form_scaled},
    potato::{HotPotato, PASS_LOCK},
};

const DT: f32 = 1. / 60.;

fn sample(games: u64, offset: u64, scale: f32, top: f32, field: f32) {
    let mut credit = 0.;
    let mut started = 0;
    let mut started_wins = 0.;
    let mut late_received = 0;
    let mut late_received_losses = 0;
    let mut received = 0;
    let mut passed = 0;
    let mut first_booms = 0;
    let mut ties = 0;
    for n in 0..games {
        let seed = 10_000 + offset + n;
        // Pair every favorite seat with every seed-driven starting slot.
        let star = ((n / 4) % 4) as u8;
        let seats: Vec<_> = (0..4)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == star { top } else { field },
                team: seat % 2,
                ally: None,
            })
            .collect();
        let mut game = HotPotato::new(&on_form_scaled(&seats, seed, scale), seed);
        let first_holder = game.holder == Some(star);
        started += usize::from(first_holder);
        let mut events = Vec::new();
        let mut late = false;
        let mut first_boom = None;
        for _ in 0..60 * 100 {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, DT)))
                .collect();
            game.step(&inputs, DT, &mut events);
            for event in events.drain(..) {
                match event["kind"].as_str() {
                    Some("pass") => {
                        passed += usize::from(event["from"].as_u64() == Some(u64::from(star)));
                        if event["to"].as_u64() == Some(u64::from(star)) {
                            received += 1;
                            if event["fuse"].as_f64().unwrap_or(f64::INFINITY)
                                <= f64::from(PASS_LOCK)
                            {
                                late = true;
                            }
                        }
                    }
                    Some("boom") => {
                        let seat = event["seat"].as_u64().map(|s| s as u8);
                        if first_boom.is_none() {
                            first_boom = seat;
                        }
                    }
                    _ => {}
                }
            }
            if game.placements().is_some() {
                break;
            }
        }
        let first = &game.placements().expect("round ended")[0];
        let won = if first.contains(&star) {
            1. / first.len() as f64
        } else {
            0.
        };
        credit += won;
        started_wins += if first_holder { won } else { 0. };
        ties += usize::from(first.len() > 1);
        first_booms += usize::from(first_boom == Some(star));
        late_received += usize::from(late);
        late_received_losses += usize::from(late && won == 0.);
    }
    println!(
        "{top:.1} vs {field:.1}: win {:.0}% start {started}/{games} started win {:.0}% first boom {first_booms} passes {passed} received {received} late received {late_received} late losses {late_received_losses} ties {ties}",
        credit * 100. / games as f64,
        started_wins * 100. / started.max(1) as f64,
    );
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let offset = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(220_000);
    let scale = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1.);
    sample(games, offset, scale, 1., 0.7);
    sample(games, offset, scale, 0.7, 0.4);
}
