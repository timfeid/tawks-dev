//! How often the best bot at a mixed table wins, through the same bout
//! factory the room uses, so bot form is part of the run.
//! cargo run --release -p party --example form_balance -- [games] [keys,comma,separated]
//!
//! step_top: one ruthless bot against sharp ones. step_mid: one sharp bot
//! against steady ones. ladder: one bot of each named skill at one table, and
//! each one's share of wins (relaxed / steady / sharp / ruthless). equal: four
//! sharp bots; the busiest seat's share of wins, which should sit near one in
//! four.
//! FORM_BALANCE_SEED shifts all seed ranges for independent validation.
use party::bout::{BOUTS, BoutSpec, Contestant, Format, create, on_form_scaled};
use std::thread;

const DT: f32 = 1. / 60.;

fn seed_offset() -> u64 {
    std::env::var("FORM_BALANCE_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn rotating_seat(game_index: u64, players: usize) -> usize {
    (game_index as usize / players) % players
}

fn winners(spec: &BoutSpec, skills: &[f32], seed: u64) -> Option<Vec<u8>> {
    let seats: Vec<_> = skills
        .iter()
        .enumerate()
        .map(|(s, &skill)| Contestant {
            seat: s as u8,
            is_bot: true,
            skill,
            team: (s % 2) as u8,
            ally: None,
            body_width: 1.0,
            pet: 4,
        })
        .collect();
    let scale = std::env::var("FORM_SCALE")
        .ok()
        .and_then(|value| value.parse::<f32>().ok());
    let mut game = if let Some(scale) = scale {
        (spec.create)(&on_form_scaled(&seats, seed, scale), seed)
    } else {
        create(spec.key, &seats, seed)?
    };
    let mut events = Vec::new();
    for _ in 0..60 * 600 {
        let inputs: Vec<_> = seats
            .iter()
            .map(|s| (s.seat, game.bot_input(s.seat, DT)))
            .collect();
        game.step(&inputs, DT, &mut events);
        events.clear();
        if let Some(order) = game.placements() {
            return order.first().cloned();
        }
    }
    None
}

fn share(spec: &BoutSpec, players: usize, top: f32, field: f32, games: u64) -> f64 {
    let mut credit = 0.;
    for seed in 0..games {
        // Vary the assigned seat independently of the seed's spawn rotation.
        let star = rotating_seat(seed, players);
        let skills: Vec<f32> = (0..players)
            .map(|s| if s == star { top } else { field })
            .collect();
        if let Some(first) = winners(spec, &skills, 10_000 + seed_offset() + seed) {
            if first.contains(&(star as u8)) {
                credit += 1. / first.len() as f64;
            }
        }
    }
    credit / games as f64
}

fn ladder(spec: &BoutSpec, games: u64) -> Vec<f64> {
    let levels: Vec<f32> = party::bout::SKILLS.iter().map(|(_, s)| *s).collect();
    let mut credit = vec![0.; levels.len()];
    for seed in 0..games {
        let shift = rotating_seat(seed, levels.len());
        let skills: Vec<f32> = (0..levels.len())
            .map(|s| levels[(s + shift) % levels.len()])
            .collect();
        if let Some(first) = winners(spec, &skills, 30_000 + seed_offset() + seed) {
            for seat in &first {
                credit[(*seat as usize + shift) % levels.len()] += 1. / first.len() as f64;
            }
        }
    }
    credit.iter().map(|c| c / games as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn favorite_visits_every_spawn_slot_from_every_seat() {
        for players in 2..=8 {
            let mut counts = vec![vec![0; players]; players];
            for game_index in 0..(players * players) as u64 {
                let star = rotating_seat(game_index, players);
                let spawn = (star + (10_000 + game_index) as usize % players) % players;
                counts[star][spawn] += 1;
            }
            assert!(counts.iter().flatten().all(|&count| count == 1));
        }
    }
}

fn busiest_seat(spec: &BoutSpec, players: usize, games: u64) -> f64 {
    let mut credit = vec![0.; players];
    for seed in 0..games {
        if let Some(first) = winners(spec, &vec![0.7; players], 20_000 + seed_offset() + seed) {
            for seat in &first {
                credit[*seat as usize] += 1. / first.len() as f64;
            }
        }
    }
    credit.iter().cloned().fold(0., f64::max) / games as f64
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let games = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100u64);
    let keys = args.get(2).cloned();
    let specs: Vec<&'static BoutSpec> = BOUTS
        .iter()
        .filter(|spec| {
            keys.as_ref()
                .is_none_or(|keys| keys.split(',').any(|key| key == spec.key))
        })
        .collect();
    let rows: Vec<_> = specs
        .into_iter()
        .map(|spec| {
            thread::spawn(move || {
                let players = 4.clamp(spec.min_players, spec.max_players);
                let top = share(spec, players, 1.0, 0.7, games);
                let mid = share(spec, players, 0.7, 0.4, games);
                let equal = busiest_seat(spec, players, games);
                let ladder = if spec.min_players <= 4 && spec.max_players >= 4 {
                    ladder(spec, games)
                } else {
                    Vec::new()
                };
                (
                    spec.key,
                    spec.format == Format::Teams,
                    players,
                    top,
                    mid,
                    ladder,
                    equal,
                )
            })
        })
        .collect();
    println!("bout | players | step_top | step_mid | ladder r/st/sh/ru | equal busiest seat");
    for row in rows {
        let (key, teams, players, top, mid, ladder, equal) = row.join().unwrap();
        let ladder: Vec<String> = ladder.iter().map(|c| format!("{:.0}", c * 100.)).collect();
        println!(
            "{key}{} | {players} | {:.0}% | {:.0}% | {} | {:.0}%",
            if teams { " (teams)" } else { "" },
            top * 100.,
            mid * 100.,
            ladder.join("/"),
            equal * 100.
        );
    }
}
