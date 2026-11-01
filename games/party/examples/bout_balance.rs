//! Actual bot controllers and physics; no synthetic winner injection.
//! cargo run -p party --example bout_balance -- 16 /tmp/bout-balance.json
use party::bout::{BOUTS, Contestant, Format};
use serde::Serialize;
use std::{collections::BTreeMap, time::Instant};
#[derive(Default, Serialize)]
struct Summary {
    games: u32,
    finished: u32,
    seconds: f64,
    min_seconds: f32,
    max_seconds: f32,
    ties: u32,
    seat_credit: Vec<f64>,
    sharp_credit: f64,
    events: BTreeMap<String, u64>,
    wall_ms: u128,
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let samples = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(16u64);
    let output = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("/tmp/bout-balance.json");
    let mut report = BTreeMap::new();
    for spec in BOUTS {
        if args
            .get(3)
            .is_some_and(|keys| !keys.split(',').any(|key| key == spec.key))
        {
            continue;
        }
        for players in [2, 4, 8usize] {
            if players < spec.min_players || players > spec.max_players {
                continue;
            }
            for mixed in [false, true] {
                let mut summary = Summary {
                    seat_credit: vec![0.; players],
                    min_seconds: f32::INFINITY,
                    ..Default::default()
                };
                let clock = Instant::now();
                for seed in 0..samples {
                    let sharp = seed as usize % players;
                    let seats: Vec<_> = (0..players)
                        .map(|s| Contestant {
                            seat: s as u8,
                            is_bot: true,
                            skill: if mixed && s == sharp { 0.95 } else { 0.4 },
                            team: (s % 2) as u8,
                            ally: None,
                        })
                        .collect();
                    let mut game = (spec.create)(&seats, seed);
                    let mut events = Vec::new();
                    let mut seconds = 0.;
                    for _ in 0..60 * 360 {
                        let inputs: Vec<_> = seats
                            .iter()
                            .map(|s| (s.seat, game.bot_input(s.seat, 1. / 60.)))
                            .collect();
                        game.step(&inputs, 1. / 60., &mut events);
                        seconds += 1. / 60.;
                        for event in events.drain(..) {
                            if let Some(kind) = event["kind"].as_str() {
                                *summary.events.entry(kind.to_owned()).or_default() += 1;
                            }
                        }
                        if let Some(order) = game.placements() {
                            summary.finished += 1;
                            if let Some(first) = order.first() {
                                let expected = if spec.format == Format::Teams {
                                    players / 2
                                } else {
                                    1
                                };
                                summary.ties += u32::from(first.len() > expected);
                                for &seat in first {
                                    summary.seat_credit[seat as usize] += 1. / first.len() as f64;
                                }
                                if first.contains(&(sharp as u8)) {
                                    summary.sharp_credit += 1. / first.len() as f64;
                                }
                            }
                            break;
                        }
                    }
                    summary.games += 1;
                    summary.seconds += seconds as f64;
                    summary.min_seconds = summary.min_seconds.min(seconds);
                    summary.max_seconds = summary.max_seconds.max(seconds);
                }
                summary.wall_ms = clock.elapsed().as_millis();
                let key = format!(
                    "{}-{}-{}",
                    spec.key,
                    players,
                    if mixed { "sharp" } else { "equal" }
                );
                println!(
                    "{key}: {}/{} finished, {:.1}s mean, {} ties",
                    summary.finished,
                    summary.games,
                    summary.seconds / summary.games as f64,
                    summary.ties
                );
                report.insert(key, summary);
            }
        }
    }
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("Saved {output}");
}
