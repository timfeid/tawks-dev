//! Production-form Crumble Run outcomes by fall cause.
//! cargo run --release -p party --example crumble_audit -- [games] [seed offset] [form scale]
use party::{
    bout::{Bout, Contestant, Input, on_form_scaled},
    crumble::{CrumbleRun, TileState},
};

const DT: f32 = 1. / 60.;

fn sample(games: u64, offset: u64, scale: f32, top: f32, field: f32) {
    let mut credit = 0.;
    let mut first_fall = 0;
    let mut survived = 0;
    let mut hole = 0;
    let mut outside = 0;
    let mut other = 0;
    let mut from_cracking = 0;
    let mut from_solid = 0;
    let mut from_hole = 0;
    let mut near_solid = 0;
    let mut reachable_solid = 0;
    let mut any_solid = 0;
    let mut fall_seconds = 0.;
    let mut falls = 0;
    let mut round_seconds = 0.;
    let mut ties = 0;
    for n in 0..games {
        let seed = 10_000 + offset + n;
        let star = (n % 4) as u8;
        let seats: Vec<_> = (0..4)
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
        let mut game = CrumbleRun::new(&on_form_scaled(&seats, seed, scale), seed);
        let mut events = Vec::new();
        let mut star_fell = false;
        for _ in 0..60 * 80 {
            let me = game.players.iter().find(|p| p.seat == star).unwrap();
            let before = game.tile_state(me.run.x, me.run.z);
            let before_near_solid = game.tiles.iter().any(|tile| {
                tile.state == TileState::Solid
                    && (tile.x - me.run.x).hypot(tile.z - me.run.z) <= 2.6 * 1.8
            });
            let before_reachable_solid = game.tiles.iter().any(|tile| {
                tile.state == TileState::Solid
                    && (tile.x - me.run.x).hypot(tile.z - me.run.z) <= 2.6 * 1.8
                    && (1..=12).all(|step| {
                        let t = step as f32 / 12.;
                        matches!(
                            game.tile_state(
                                me.run.x + (tile.x - me.run.x) * t,
                                me.run.z + (tile.z - me.run.z) * t,
                            ),
                            Some(TileState::Solid | TileState::Cracking)
                        )
                    })
            });
            let before_any_solid = game.tiles.iter().any(|tile| tile.state == TileState::Solid);
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, DT)))
                .collect();
            game.step(&inputs, DT, &mut events);
            events.clear();
            if !star_fell {
                let me = game.players.iter().find(|p| p.seat == star).unwrap();
                if let Some(at) = me.fell_at {
                    star_fell = true;
                    falls += 1;
                    fall_seconds += f64::from(at);
                    near_solid += usize::from(before_near_solid);
                    reachable_solid += usize::from(before_reachable_solid);
                    any_solid += usize::from(before_any_solid);
                    match before {
                        Some(TileState::Cracking) => from_cracking += 1,
                        Some(TileState::Solid) => from_solid += 1,
                        _ => from_hole += 1,
                    }
                    match game.tile_state(me.run.x, me.run.z) {
                        Some(TileState::Falling | TileState::Gone) => hole += 1,
                        None => outside += 1,
                        _ => other += 1,
                    }
                }
            }
            if game.placements().is_some() {
                break;
            }
        }
        let first = &game.placements().expect("round ended")[0];
        credit += if first.contains(&star) {
            1. / first.len() as f64
        } else {
            0.
        };
        ties += usize::from(first.len() > 1);
        survived += usize::from(!star_fell);
        let star_time = game
            .players
            .iter()
            .find(|p| p.seat == star)
            .unwrap()
            .fell_at;
        let earliest = game
            .players
            .iter()
            .filter_map(|p| p.fell_at)
            .min_by(f32::total_cmp);
        first_fall += usize::from(star_time.is_some() && star_time == earliest);
        round_seconds += f64::from(game.time);
    }
    println!(
        "{top:.1} vs {field:.1}: win {:.0}% first fall {:.0}% survive {:.0}% hole/outside/other {hole}/{outside}/{other} prior cracking/solid/hole {from_cracking}/{from_solid}/{from_hole} solid near/reachable/any {near_solid}/{reachable_solid}/{any_solid} mean fall {:.1}s mean round {:.1}s ties {:.0}%",
        credit * 100. / games as f64,
        first_fall as f64 * 100. / games as f64,
        survived as f64 * 100. / games as f64,
        fall_seconds / falls.max(1) as f64,
        round_seconds / games as f64,
        ties as f64 * 100. / games as f64,
    );
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let offset = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(120_000);
    let scale = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.4);
    sample(games, offset, scale, 1., 0.7);
    sample(games, offset, scale, 0.7, 0.4);
}
