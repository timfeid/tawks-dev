//! Paired board-AI experiment: cargo run -p party --example position_balance -- 12
//! Uses the real Room board/timer/bot loop and identical seeded arena awards
//! in both arms, so arena physics cannot hide the positioning change.
use party::{
    board::Map,
    room::{Phase, Room},
};
use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Default, Serialize)]
struct Report {
    seed: u64,
    map: String,
    positioning: bool,
    mature: bool,
    finished: bool,
    stalled: bool,
    ticks: u64,
    turns: usize,
    round_turns: Vec<usize>,
    turn_seconds: Vec<f32>,
    max_turn_seconds: f32,
    position_begins: u32,
    position_activates: u32,
    position_cancels: u32,
    position_moved: u32,
    position_timeouts: u32,
    position_wait_seconds: Vec<f32>,
    item_attempts: BTreeMap<String, u32>,
    item_hits: BTreeMap<String, u32>,
    item_results: BTreeMap<String, u32>,
    ambient_paid: i64,
    pedestrian_paid: i64,
    crawler_hits: u32,
    traffic_hits: u32,
    traffic_near_misses: u32,
    drive_starts: u32,
    drive_ends: u32,
    traffic_wait_seconds: f32,
    finale_seconds: f32,
    arena_paid: i64,
    combat_transferred: i64,
    prizes_bought: u32,
    knockouts: u32,
    winners: Vec<u8>,
    coin_leaders: Vec<u8>,
    final_coins: Vec<i32>,
    final_prizes: Vec<u32>,
    upgrades: BTreeMap<String, u32>,
    last_state: String,
}
fn count(map: &mut BTreeMap<String, u32>, key: &str) {
    *map.entry(key.into()).or_default() += 1;
}
fn observe(report: &mut Report, events: Vec<Value>) {
    for event in events {
        let kind = event["kind"].as_str().unwrap_or("");
        let coins = event["coins"].as_i64().unwrap_or(0);
        if kind == "walk_coin" || kind == "vacuumed" && event["what"] == "air_coin" {
            report.ambient_paid += coins;
        }
        if kind == "npc_robbed" {
            report.pedestrian_paid += coins;
        }
        if kind == "crawler_hit" {
            report.crawler_hits += 1;
        }
        if kind == "traffic_hit" {
            report.traffic_hits += 1;
        }
        if kind == "traffic_near_miss" {
            report.traffic_near_misses += 1;
        }
        if kind == "drive_start" {
            report.drive_starts += 1;
        }
        if kind == "drive_end" {
            report.drive_ends += 1;
        }
        if kind == "minigame" {
            report.arena_paid += coins;
        }
        if kind == "siphon" || kind == "item" && event["item"] == "grapple" {
            report.combat_transferred += coins;
        }
        if kind == "prize" {
            report.prizes_bought += 1;
        }
        if kind == "knockout" {
            report.knockouts += 1;
        }
        if kind == "position" {
            match event["phase"].as_str().unwrap_or("") {
                "begin" => report.position_begins += 1,
                "activate" => {
                    report.position_activates += 1;
                    report.position_moved +=
                        u32::from(event["distance"].as_f64().unwrap_or(0.0) > 0.05);
                    report.position_timeouts +=
                        u32::from(event["timed_out"].as_bool().unwrap_or(false));
                    report
                        .position_wait_seconds
                        .push(event["waited"].as_f64().unwrap_or(0.0) as f32);
                }
                "cancel" => report.position_cancels += 1,
                _ => {}
            }
        }
        if kind == "item" {
            let item = event["item"].as_str().unwrap_or("");
            let attempt = match item {
                "bat_ready" => Some("bat"),
                "throw_ready" => event["thrown"].as_str(),
                "check_ready" => event["checking"].as_str(),
                _ => None,
            };
            if let Some(item) = attempt {
                count(&mut report.item_attempts, item);
            }
            if let Some(quality) = event["quality"].as_str() {
                count(&mut report.item_results, &format!("{item}:{quality}"));
                if quality == "hit"
                    || matches!(quality, "direct" | "splash")
                    || item == "bat" && event["target"].is_number()
                {
                    count(&mut report.item_hits, item);
                }
            }
        }
    }
}
fn run(seed: u64, map: Map, positioning: bool) -> Report {
    let mut report = Report {
        seed,
        map: serde_json::to_value(map).unwrap().as_str().unwrap().into(),
        positioning,
        ..Report::default()
    };
    let mut room = Room::new(format!("position-balance-{seed}-{positioning}"), seed);
    room.connect("host", "Host", "").unwrap();
    for _ in 0..3 {
        room.add_bot("host").unwrap();
    }
    room.map = map;
    room.mature = true;
    report.mature = room.mature;
    room.start_party("host").unwrap();
    for member in &mut room.members {
        member.is_bot = true;
    }
    room.board.as_mut().unwrap().timed_items = positioning;
    room.board.as_mut().unwrap().take_events();
    let mut opening=true;
    let mut round = 1u32;
    let skills = [0.3, 0.5, 0.75, 0.95];
    let mut turn: Option<u8> = None;
    let mut turn_ticks = 0u64;
    let mut progress = 0u64;
    let mut round_turn_begin = 0usize;
    const MAX_TICKS: u64 = 60 * 60 * 30;
    for tick in 0..MAX_TICKS {
        if opening {
            room.step();report.ticks=tick+1;
            if let Some(board)=room.board.as_mut(){observe(&mut report,board.take_events());}
            opening=room.phase!=Phase::Board;
            continue;
        }
        if matches!(room.phase, Phase::GameOver { .. }) {
            report.finished = true;
            break;
        }
        if matches!(room.phase, Phase::MapFinale | Phase::Results { .. }) {
            report.finale_seconds += 1. / 60.;
            room.step();
            report.ticks = tick + 1;
            if let Some(board) = room.board.as_mut() {
                observe(&mut report, board.take_events());
            }
            continue;
        }
        if !matches!(room.phase, Phase::Board) {
            report.last_state = format!("unexpected {:?}", room.phase);
            report.stalled = true;
            break;
        }
        let seat = room.board.as_ref().unwrap().turn_seat();
        if turn != Some(seat) {
            if turn.is_some() {
                report.turn_seconds.push(turn_ticks as f32 / 60.0);
            }
            turn = Some(seat);
            turn_ticks = 0;
        }
        room.bot_skill = skills[(seat as usize + seed as usize) % skills.len()];
        turn_ticks += 1;
        room.step();
        let board = room.board.as_mut().unwrap();
        let events = board.take_events();
        if !events.is_empty() {
            progress = tick;
        }
        observe(&mut report, events);
        if board.view().walk.is_some_and(|w| w.waiting > 0.) {
            report.traffic_wait_seconds += 1. / 60.;
        }
        report.ticks = tick + 1;
        if tick - progress > 60 * 120 || turn_ticks > 60 * 180 {
            report.stalled = true;
            report.last_state = format!("round={round} seat={seat} step={:?}", board.step());
            break;
        }
        if matches!(room.phase, Phase::Vote { .. }) {
            report.turn_seconds.push(turn_ticks as f32 / 60.0);
            report
                .round_turns
                .push(report.turn_seconds.len() - round_turn_begin);
            round_turn_begin = report.turn_seconds.len();
            turn = None;
            turn_ticks = 0;
            let mut order = vec![0u8, 1, 2, 3];
            order.shuffle(&mut StdRng::seed_from_u64(
                seed ^ u64::from(round) * 0x9e3779b9,
            ));
            let placements: Vec<Vec<u8>> = order.into_iter().map(|seat| vec![seat]).collect();
            let more = board.finish_round(&placements);
            observe(&mut report, board.take_events());
            if !more {
                report.finished = true;
                break;
            }
            round += 1;
            room.phase = Phase::Board;
            progress = tick;
        }
    }
    let board = room.board.as_ref().unwrap();
    let view = board.view();
    report.turns = report.turn_seconds.len();
    report.max_turn_seconds = report.turn_seconds.iter().copied().fold(0.0, f32::max);
    report.final_coins = view.tokens.iter().map(|token| token.coins).collect();
    report.final_prizes = view.tokens.iter().map(|token| token.prizes).collect();
    report.winners = board.placements().first().cloned().unwrap_or_default();
    let max_coins = view
        .tokens
        .iter()
        .map(|token| token.coins)
        .max()
        .unwrap_or(0);
    report.coin_leaders = view
        .tokens
        .iter()
        .filter(|token| token.coins == max_coins)
        .map(|token| token.seat)
        .collect();
    for token in view.tokens {
        for upgrade in token.upgrades {
            count(
                &mut report.upgrades,
                serde_json::to_value(upgrade).unwrap().as_str().unwrap(),
            );
        }
    }
    if !report.finished && !report.stalled {
        report.stalled = true;
        report.last_state = format!("tick cap phase={:?} step={:?}", room.phase, board.step());
    }
    report
}
fn main() {
    let seeds = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse::<u64>().ok())
        .unwrap_or(12);
    let forced_map = std::env::args()
        .nth(2)
        .map(|name| serde_json::from_value::<Map>(Value::String(name)).expect("map key"));
    for seed in 0..seeds {
        for enabled in [false, true] {
            let report = run(
                1000 + seed,
                forced_map.unwrap_or(Map::ALL[seed as usize % Map::ALL.len()]),
                enabled,
            );
            println!("{}", serde_json::to_string(&report).unwrap());
        }
    }
    eprintln!(
        "{}",
        json!({"paired_seeds":seeds,"bots":4,"skills":[0.3,0.5,0.75,0.95],"arena":"identical seeded placements/normal award rules","tick_rate":60})
    );
}
