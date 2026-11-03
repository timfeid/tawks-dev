use super::*;
use crate::bout::{Contestant, TICK_DT};

fn bots(n: u8, skill: f32) -> Vec<Contestant> {
    (0..n).map(|seat| Contestant::bot(seat, skill)).collect()
}

fn people(n: u8) -> Vec<Contestant> {
    (0..n).map(Contestant::person).collect()
}

fn run_bots(game: &mut TowerTeeter, seats: &[u8]) -> Vec<Value> {
    let mut events = Vec::new();
    for _ in 0..((ROUND_SECONDS + 2.0) / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|&s| (s, game.bot_input(s, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    events
}

fn aim(x: f32, angle: f32, fire: bool) -> Input {
    Input {
        pitch: x / HOVER_RANGE,
        yaw: angle,
        fire,
        ..Default::default()
    }
}

fn tick(game: &mut TowerTeeter, inputs: &[(u8, Input)], seconds: f32, events: &mut Vec<Value>) {
    for i in 0..(seconds / TICK_DT) as usize {
        let now: Vec<(u8, Input)> = inputs
            .iter()
            .map(|(s, input)| {
                (
                    *s,
                    Input {
                        fire: input.fire && i == 0,
                        ..*input
                    },
                )
            })
            .collect();
        game.step(&now, TICK_DT, events);
    }
}

#[test]
fn bots_finish_and_rank_everyone() {
    for n in [2u8, 4, 8] {
        let mut game = TowerTeeter::new(&bots(n, 0.5), 11 + n as u64);
        let seats: Vec<u8> = (0..n).collect();
        let events = run_bots(&mut game, &seats);
        let placements = game.placements().expect("the horn ends it");
        assert_eq!(placements.iter().flatten().count(), n as usize);
        assert!(events.iter().any(|e| e["kind"] == "horn"));
        assert!(events.iter().any(|e| e["kind"] == "final"));
        assert!(
            game.rafts.iter().all(|r| r.placed >= 4),
            "bots keep building: {:?}",
            game.rafts.iter().map(|r| r.placed).collect::<Vec<_>>()
        );
        assert!(
            game.rafts.iter().any(|r| r.height > 2.0),
            "somebody builds a tower"
        );
    }
}

#[test]
fn every_raft_gets_the_same_queue_and_sea() {
    let game = TowerTeeter::new(&people(4), 5);
    let first: Vec<usize> = (0..4).map(|i| game.held(&game.rafts[i])).collect();
    assert!(first.iter().all(|k| *k == first[0]));
    assert_eq!(
        KINDS[game.queue[0] as usize].name, "crate",
        "a steady opener"
    );
}

#[test]
fn a_careful_human_builds_and_a_dropped_piece_costs_height() {
    let mut game = TowerTeeter::new(&people(2), 3);
    let mut events = Vec::new();
    // Seat 0 stacks in the middle, waiting out each drop.
    for _ in 0..4 {
        tick(
            &mut game,
            &[(0, aim(0.0, 0.0, true)), (1, aim(0.0, 0.0, false))],
            2.0,
            &mut events,
        );
    }
    let built = game.raft(0).unwrap().height;
    assert!(built > 1.5, "four pieces stand, {built}");
    // Seat 1 throws its piece off the end of the raft.
    tick(
        &mut game,
        &[(0, Input::default()), (1, aim(HOVER_RANGE, 0.0, true))],
        3.0,
        &mut events,
    );
    let r1 = game.raft(1).unwrap();
    assert_eq!(r1.lost, 1, "it went in the sea");
    assert!(r1.score() < 0.0);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "splash" && e["seat"] == 1)
    );
}

#[test]
fn drops_respect_the_cooldown_and_the_horn() {
    let mut game = TowerTeeter::new(&people(2), 9);
    let mut events = Vec::new();
    for _ in 0..10 {
        game.step(&[(0, aim(0.0, 0.0, true))], TICK_DT, &mut events);
    }
    assert_eq!(game.raft(0).unwrap().placed, 1, "one drop per cooldown");
    while game.time < PLAY_SECONDS + 0.1 {
        game.step(&[], TICK_DT, &mut events);
    }
    let placed = game.raft(0).unwrap().placed;
    game.step(&[(0, aim(0.0, 0.0, true))], TICK_DT, &mut events);
    assert_eq!(
        game.raft(0).unwrap().placed,
        placed,
        "nothing drops after the horn"
    );
}

#[test]
fn hostile_aim_is_clamped() {
    let mut game = TowerTeeter::new(&people(2), 1);
    let mut events = Vec::new();
    game.step(
        &[(
            0,
            Input {
                pitch: f32::NAN,
                yaw: f32::INFINITY,
                fire: true,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );
    let r = game.raft(0).unwrap();
    assert!(r.cursor.is_finite() && r.angle.is_finite());
    game.step(
        &[(
            1,
            Input {
                pitch: 1e9,
                yaw: 1e9,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );
    let r = game.raft(1).unwrap();
    assert!(r.cursor <= HOVER_RANGE + 1e-4);
    assert!(r.angle.abs() <= std::f32::consts::PI + 1e-4);
    assert!(
        game.snapshot()["players"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["hold"][1].as_f64().unwrap().is_finite())
    );
}

#[test]
fn a_leaver_still_places_and_nobody_waits_for_them() {
    let mut field = bots(3, 0.6);
    field.push(Contestant::person(3));
    let mut game = TowerTeeter::new(&field, 21);
    let mut events = Vec::new();
    // The person leaves at once; the room sends them nothing at all.
    for _ in 0..((ROUND_SECONDS + 1.0) / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..3).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
    }
    let placements = game.placements().unwrap();
    assert_eq!(
        placements.iter().flatten().count(),
        4,
        "the leaver is still ranked"
    );
    let empty = game.raft(3).unwrap();
    assert_eq!((empty.placed, empty.score()), (0, 0.0));
    let below = placements.iter().position(|g| g.contains(&3)).unwrap();
    assert!(
        placements[below + 1..]
            .iter()
            .flatten()
            .all(|s| game.raft(*s).unwrap().score() < 0.0),
        "only rafts that lost more than they built rank below an empty one"
    );
}

#[test]
fn level_ties_share_a_place() {
    let mut game = TowerTeeter::new(&people(3), 2);
    let mut events = Vec::new();
    while game.placements().is_none() {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.placements().unwrap(), vec![vec![0, 1, 2]]);
}

#[test]
fn the_same_seed_and_inputs_replay_the_same_towers() {
    let run = || {
        let mut game = TowerTeeter::new(&bots(4, 0.5), 77);
        run_bots(&mut game, &[0, 1, 2, 3]);
        game.snapshot().to_string()
    };
    assert_eq!(run(), run());
}

#[test]
fn the_sea_roughens_and_swells_are_announced() {
    let game = TowerTeeter::new(&people(2), 4);
    let calm = (0..300)
        .map(|i| game.sea.pose(i as f32 * 0.02).angle.abs())
        .fold(0.0f32, f32::max);
    let rough = (0..300)
        .map(|i| {
            game.sea
                .pose(PLAY_SECONDS - 6.0 + i as f32 * 0.02)
                .angle
                .abs()
        })
        .fold(0.0f32, f32::max);
    assert!(rough > calm * 2.0, "{calm} then {rough}");
    assert!(game.sea.swells.len() >= 3);
    let mut game = TowerTeeter::new(&people(2), 4);
    let mut events = Vec::new();
    while game.placements().is_none() {
        game.step(&[], TICK_DT, &mut events);
    }
    let swells = events.iter().filter(|e| e["kind"] == "swell").count();
    assert_eq!(swells, game.sea.swells.len(), "one warning each");
}

#[test]
fn sharper_bots_score_better_and_lose_less_to_the_sea() {
    let (mut relaxed, mut ruthless) = ((0.0, 0), (0.0, 0));
    for seed in 0..10 {
        let field: Vec<Contestant> = (0..4u8)
            .map(|s| Contestant::bot(s, if s < 2 { 0.15 } else { 1.0 }))
            .collect();
        let mut game = TowerTeeter::new(&field, 300 + seed);
        run_bots(&mut game, &[0, 1, 2, 3]);
        for r in &game.rafts {
            let side = if r.seat < 2 {
                &mut relaxed
            } else {
                &mut ruthless
            };
            side.0 += r.score();
            side.1 += r.lost;
        }
    }
    assert!(
        ruthless.0 > relaxed.0,
        "ruthless {ruthless:?} against relaxed {relaxed:?}"
    );
    assert!(
        ruthless.1 < relaxed.1,
        "ruthless {ruthless:?} against relaxed {relaxed:?}"
    );
}

/// `cargo test -p party teeter::tests::balance -- --ignored --nocapture`
#[test]
#[ignore]
fn balance() {
    use std::collections::BTreeMap;
    let skills = [0.15f32, 0.4, 0.7, 1.0];
    let mut wins: BTreeMap<usize, u32> = BTreeMap::new();
    let mut heights = [0.0f32; 4];
    let mut lost = [0u32; 4];
    let mut placed = [0u32; 4];
    let mut topples = 0;
    let (mut quick, mut later) = (0, 0);
    let mut swells = 0;
    let mut finished = 0;
    let runs = 60u64;
    let mut best = 0.0f32;
    let mut scores = Vec::new();
    for seed in 0..runs {
        let field: Vec<Contestant> = (0..4u8)
            .map(|s| Contestant::bot(s, skills[s as usize]))
            .collect();
        let mut game = TowerTeeter::new(&field, 1000 + seed);
        let events = run_bots(&mut game, &[0, 1, 2, 3]);
        if let Some(p) = game.placements() {
            finished += 1;
            for s in &p[0] {
                *wins.entry(*s as usize).or_default() += 1;
            }
        }
        topples += events.iter().filter(|e| e["kind"] == "topple").count();
        for e in events.iter().filter(|e| e["kind"] == "splash") {
            let dropped = events
                .iter()
                .position(|d| d["kind"] == "drop" && d["seat"] == e["seat"] && d["id"] == e["id"])
                .unwrap();
            let splashed = events.iter().position(|x| std::ptr::eq(x, e)).unwrap();
            let between = events[dropped..splashed]
                .iter()
                .filter(|d| d["kind"] == "drop" && d["seat"] == e["seat"])
                .count();
            if between <= 1 { quick += 1 } else { later += 1 }
        }
        swells += events.iter().filter(|e| e["kind"] == "swell").count();
        for r in &game.rafts {
            heights[r.seat as usize] += r.height;
            lost[r.seat as usize] += r.lost;
            placed[r.seat as usize] += r.placed;
            best = best.max(r.height);
            scores.push(r.score());
        }
    }
    println!(
        "finished {finished}/{runs}  lost straight off {quick}  lost in a later collapse {later}"
    );
    for (i, s) in skills.iter().enumerate() {
        println!(
            "skill {s:.2}: wins {:>3}  mean height {:.2}  lost/bout {:.2}  placed/bout {:.1}",
            wins.get(&i).copied().unwrap_or(0),
            heights[i] / runs as f32,
            lost[i] as f32 / runs as f32,
            placed[i] as f32 / runs as f32
        );
    }
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!(
        "topples/bout {:.2}  swells/bout {:.2}  tallest {:.2}  score p10 {:.2} p50 {:.2} p90 {:.2}",
        topples as f32 / runs as f32,
        swells as f32 / runs as f32,
        best,
        scores[scores.len() / 10],
        scores[scores.len() / 2],
        scores[scores.len() * 9 / 10]
    );
    assert_eq!(finished, runs as u32);
    let mut game = TowerTeeter::new(&bots(8, 1.0), 9);
    let mut events = Vec::new();
    let mut worst = std::time::Duration::ZERO;
    let mut total = std::time::Duration::ZERO;
    let mut ticks = 0;
    while game.placements().is_none() {
        let start = std::time::Instant::now();
        let inputs: Vec<(u8, Input)> = (0..8).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        let took = start.elapsed();
        worst = worst.max(took);
        total += took;
        ticks += 1;
    }
    println!(
        "eight sharp bots: mean tick {:?}, worst tick {:?}",
        total / ticks,
        worst
    );
}

#[test]
fn a_neat_column_rides_out_the_sea_but_a_skyscraper_does_not() {
    let survives = |crates: usize| {
        let mut game = TowerTeeter::new(&people(1), 5);
        game.queue = vec![5; 200];
        let mut events = Vec::new();
        for _ in 0..crates {
            game.step(&[(0, aim(0.0, 0.0, true))], TICK_DT, &mut events);
            for _ in 0..90 {
                game.step(&[(0, aim(0.0, 0.0, false))], TICK_DT, &mut events);
            }
        }
        while game.placements().is_none() {
            game.step(&[], TICK_DT, &mut events);
        }
        game.raft(0).unwrap().lost == 0
    };
    assert!(
        survives(6),
        "six crates square on the deck hold through every swell"
    );
    assert!(!survives(10), "ten in a column go over");
}
