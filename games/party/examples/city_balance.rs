//! Reproducible city AI sweep with real traffic, seeded starts and seat counts.
//! cargo run -p party --example city_balance -- 64 /tmp/city-balance.json
use party::{
    board::{Board, Map},
    city_heist::{CityConfig, CityGame, Side},
};
use serde::Serialize;
use std::{collections::BTreeMap, time::Instant};
#[derive(Serialize)]
struct Run {
    players: u8,
    police: u8,
    seed: u64,
    winner: Option<Side>,
    draw: bool,
    night_score: [u16; 2],
    reason: Option<String>,
    seconds: f32,
    vault_opened: bool,
    loot_moved: bool,
    arrests: u32,
    rescues: u32,
    ticks: u64,
    wall_ms: u128,
}
fn run(players: u8, police: u8, seed: u64) -> Run {
    let seats: Vec<_> = (0..players).map(|s| (s, true)).collect();
    let ids: Vec<_> = (0..players).collect();
    let config = CityConfig {
        police,
        jobs: 1,
        host_police: false,
    };
    let mut game = CityGame::seeded(&seats, config, true, seed);
    let mut board = Board::for_room(Map::Frozen, &ids, 10, seed);
    board.use_manual_events();
    let clock = Instant::now();
    let mut old = BTreeMap::new();
    let mut record = Run {
        players,
        police,
        seed,
        winner: None,
        draw: false,
        night_score: [0, 0],
        reason: None,
        seconds: 0.,
        vault_opened: false,
        loot_moved: false,
        arrests: 0,
        rescues: 0,
        ticks: 0,
        wall_ms: 0,
    };
    for tick in 0..60 * 365 {
        board.traffic_tick_with_obstacles(1. / 60.,&game.patrol_traffic_obstacles());
        game.tick_with_traffic(1. / 60., &[], &ids, |from, to, yaw| {
            match yaw {
                Some(yaw)=>board.traffic_blocks_patrol_vehicle(from,to,yaw),
                None=>board.traffic_collision(from,to).is_some(),
            }
        },|from,to|board.traffic_wait(from,to)>0.);
        record.ticks += 1;
        if tick % 15 == 0 || game.resolved.is_some() {
            let view = game.view_for(Some(police));
            record.vault_opened |= view.loot_carrier.is_some() || view.loot.is_some();
            record.loot_moved |= view.actors.iter().any(|a| {
                a.carrying
                    && a.x
                        .zip(a.z)
                        .is_some_and(|(x, z)| (x + 4.).hypot(z - 28.) > 5.)
            });
            for actor in view.actors {
                let last = old.insert(actor.seat, actor.jailed).unwrap_or(false);
                if !last && actor.jailed {
                    record.arrests += 1;
                }
                if last && !actor.jailed {
                    record.rescues += 1;
                }
            }
        }
        if game.resolved.is_some() {
            break;
        }
    }
    let view = game.view_for(Some(police));
    record.winner = game.resolved;
    record.draw = view.draw;
    record.night_score = view.night_score;
    record.reason = view.reason.map(str::to_owned);
    record.seconds = game.time;
    record.wall_ms = clock.elapsed().as_millis();
    record
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let samples: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(32);
    let output = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("/tmp/city-balance.json");
    let variants = [
        (2, 1),
        (3, 1),
        (4, 1),
        (4, 2),
        (6, 1),
        (6, 2),
        (8, 1),
        (8, 2),
        (8, 3),
    ];
    let mut runs = Vec::new();
    std::thread::scope(|scope| {
        let jobs: Vec<_> = variants
            .into_iter()
            .map(|(players, police)| {
                scope.spawn(move || {
                    (0..samples)
                        .map(|seed| run(players, police, seed))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for job in jobs {
            runs.extend(job.join().unwrap());
        }
    });
    let mut groups = BTreeMap::new();
    for run in &runs {
        let summary=groups.entry(format!("{}v{}",run.police,run.players-run.police)).or_insert(serde_json::json!({"games":0,"police":0,"crew":0,"timeouts":0,"seconds":0.,"vault_opened":0,"arrests":0,"rescues":0,"wall_ms":0}));
        for (key, value) in [
            ("games", 1),
            (
                "police",
                u64::from(run.winner == Some(Side::Police) && !run.draw),
            ),
            (
                "crew",
                u64::from(run.winner == Some(Side::Crew) && !run.draw),
            ),
            (
                "timeouts",
                u64::from(run.reason.as_deref() == Some("The crew ran out of time")),
            ),
            ("draws", u64::from(run.draw)),
            ("vault_opened", u64::from(run.vault_opened)),
            ("arrests", run.arrests as u64),
            ("rescues", run.rescues as u64),
            ("wall_ms", run.wall_ms as u64),
        ] {
            summary[key] = serde_json::json!(summary[key].as_u64().unwrap_or(0) + value);
        }
        summary["seconds"] =
            serde_json::json!(summary["seconds"].as_f64().unwrap() + run.seconds as f64);
    }
    std::fs::write(
        output,
        serde_json::to_vec_pretty(
            &serde_json::json!({"samples":samples,"fixed_hz":60,"summary":groups,"runs":runs}),
        )
        .unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&groups).unwrap());
    println!("Saved {output}");
}
