//! Inspect why a skilled Bumper Balls bot wins or falls.
//! cargo run --release -p party --example bumper_audit -- [games] [seed offset] [players]
use bumper_balls::{BallState, BumperBalls};
use party::bout::{Bout, Contestant, Input, on_form_scaled};

const DT: f32 = 1. / 60.;

fn sample(games: u64, offset: u64, players: u8, top: f32, field: f32) {
    let mut wins = 0.;
    let mut first_falls = 0;
    let mut survivors = 0;
    let mut first_ties = 0;
    let mut no_falls = 0;
    let mut finish_seconds = 0.;
    let mut star_second = 0;
    let mut star_after_shrink = 0;
    let mut star_fall_seconds = 0.;
    let mut star_falls = 0;
    for n in 0..games {
        let seed = 10_000 + offset + n;
        let star = (n % u64::from(players)) as u8;
        let seats: Vec<_> = (0..players)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == star { top } else { field },
                team: seat % 2,
                ally: None,
                body_width: 1.0,
                pet: 4,
            })
            .collect();
        let mut game = BumperBalls::new(&on_form_scaled(&seats, seed, 1.), seed);
        let mut events = Vec::new();
        for _ in 0..60 * 65 {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, DT)))
                .collect();
            game.step(&inputs, DT, &mut events);
            events.clear();
            if game.placements().is_some() {
                break;
            }
        }
        let placement = game.placements().expect("round ended");
        let first = &placement[0];
        if first.contains(&star) {
            wins += 1. / first.len() as f64;
        }
        first_ties += usize::from(first.len() > 1);
        let star_ball = game.balls.iter().find(|b| b.seat == star).unwrap();
        survivors += usize::from(star_ball.state == BallState::Rolling);
        let first_fall = game
            .balls
            .iter()
            .filter_map(|b| b.fell_at)
            .min_by(f32::total_cmp);
        no_falls += usize::from(first_fall.is_none());
        first_falls += usize::from(star_ball.fell_at == first_fall && first_fall.is_some());
        if let Some(at) = star_ball.fell_at {
            star_falls += 1;
            star_fall_seconds += f64::from(at);
            star_after_shrink += usize::from(at >= 25.);
            star_second += usize::from(
                game.balls
                    .iter()
                    .filter(|b| b.seat != star && b.fell_at.is_none_or(|other| other > at))
                    .count()
                    == 1,
            );
        }
        finish_seconds += f64::from(game.time);
    }
    println!(
        "{players} players, {top:.1} vs {field:.1}: win {:.0}% first fall {:.0}% second {:.0}% survive {:.0}% fall after shrink {:.0}% mean fall {:.1}s tied first {:.0}% no falls {:.0}% finish {:.1}s",
        wins * 100. / games as f64,
        first_falls as f64 * 100. / games as f64,
        star_second as f64 * 100. / games as f64,
        survivors as f64 * 100. / games as f64,
        star_after_shrink as f64 * 100. / star_falls.max(1) as f64,
        star_fall_seconds / star_falls.max(1) as f64,
        first_ties as f64 * 100. / games as f64,
        no_falls as f64 * 100. / games as f64,
        finish_seconds / games as f64,
    );
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let offset = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(80_000);
    let players = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(4);
    assert!((2..=8).contains(&players));
    sample(games, offset, players, 1., 0.7);
    sample(games, offset, players, 0.7, 0.4);
}
