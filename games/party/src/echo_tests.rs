use super::*;
use crate::bout::TICK_DT;

fn people(count: u8) -> Vec<Contestant> {
    (0..count).map(Contestant::person).collect()
}

fn bots(count: u8, skill: f32) -> Vec<Contestant> {
    (0..count)
        .map(|seat| Contestant::bot(seat, skill))
        .collect()
}

/// The slot a perfect player would hit next, once per step, with a short
/// pause between rings like a person clicking.
fn perfect(game: &EchoChamber, seat: u8) -> Input {
    let Some(p) = game.players.iter().find(|p| p.seat == seat) else {
        return Input::default();
    };
    if game.phase != Phase::Replay || p.status != Status::Playing || !p.in_game() {
        return Input::default();
    }
    if ((game.clock / TICK_DT) as u32) % 12 != 6 {
        return Input::default();
    }
    pick_input(game.slot_of(game.sequence[p.progress]))
}

fn run_until(
    game: &mut EchoChamber,
    events: &mut Vec<Value>,
    mut drive: impl FnMut(&mut EchoChamber, u8) -> Input,
    stop: impl Fn(&EchoChamber) -> bool,
) -> f32 {
    let seats: Vec<u8> = game.players.iter().map(|p| p.seat).collect();
    let mut seconds = 0.0;
    for _ in 0..(200.0 / TICK_DT) as usize {
        if stop(game) || game.placements().is_some() {
            break;
        }
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|seat| (*seat, drive(game, *seat)))
            .collect();
        game.step(&inputs, TICK_DT, events);
        seconds += TICK_DT;
    }
    seconds
}

fn to_replay(game: &mut EchoChamber, events: &mut Vec<Value>) {
    run_until(
        game,
        events,
        |_, _| Input::default(),
        |g| g.phase == Phase::Replay,
    );
}

#[test]
fn a_spin_moves_every_chime_and_a_mirror_reverses_the_order() {
    let start = [0, 1, 2, 3, 4, 5];
    let spin = Twist {
        spin: 2,
        ..Default::default()
    };
    assert_eq!(
        spin.apply(&start),
        [4, 5, 0, 1, 2, 3],
        "each chime moves two slots clockwise"
    );
    let back = Twist {
        spin: -1,
        ..Default::default()
    };
    assert_eq!(back.apply(&start), [1, 2, 3, 4, 5, 0]);
    let mirror = Twist {
        spin: 0,
        mirror: true,
        swap: None,
    };
    assert_eq!(
        mirror.apply(&start),
        [0, 5, 4, 3, 2, 1],
        "slot 0 sits on the mirror line"
    );
    let all = Twist {
        spin: 1,
        mirror: true,
        swap: Some((0, 3)),
    };
    let mut expect = [5, 4, 3, 2, 1, 0];
    expect.swap(0, 3);
    assert_eq!(all.apply(&start), expect, "spin, then mirror, then swap");
    assert_eq!(
        Twist::from_json(&all.to_json()),
        all,
        "the client reads back what it is sent"
    );
}

#[test]
fn the_drawn_turn_starts_on_the_shown_layout_and_lands_on_the_new_one() {
    let before = [3, 0, 5, 1, 4, 2];
    for twist in [
        Twist {
            spin: 1,
            ..Default::default()
        },
        Twist {
            spin: -2,
            mirror: true,
            swap: None,
        },
        Twist {
            spin: 2,
            mirror: true,
            swap: Some((1, 4)),
        },
        Twist {
            spin: -1,
            mirror: false,
            swap: Some((0, 5)),
        },
    ] {
        let after = twist.apply(&before);
        let first = chime_positions(&before, &twist, 0.0, RING_RADIUS);
        let last = chime_positions(&before, &twist, twist.seconds(), RING_RADIUS);
        for slot in 0..CHIMES {
            let (x, z) = slot_position(slot as f32, RING_RADIUS);
            let at = first[before[slot] as usize];
            assert!(
                (at[0] - x).abs() < 1e-3 && (at[2] - z).abs() < 1e-3 && at[1].abs() < 1e-3,
                "{twist:?} starts in place"
            );
            let at = last[after[slot] as usize];
            assert!(
                (at[0] - x).abs() < 1e-3 && (at[2] - z).abs() < 1e-3 && at[1].abs() < 1e-3,
                "{twist:?} lands in slot {slot}: {at:?}"
            );
        }
    }
}

#[test]
fn slots_are_numbered_clockwise_from_the_far_side() {
    assert_eq!(slot_toward(0.0, -1.0), 0, "straight ahead is slot 0");
    assert_eq!(slot_toward(1.0, -0.6), 1, "then to the right");
    assert_eq!(slot_toward(0.0, 1.0), 3, "the near side is opposite");
    assert_eq!(slot_toward(-1.0, -0.6), 5, "and the left closes the ring");
    for slot in 0..CHIMES {
        let (x, z) = slot_position(slot as f32, 1.0);
        assert_eq!(slot_toward(x, z), slot);
    }
}

#[test]
fn only_a_marked_press_counts_as_a_pick() {
    assert_eq!(pick(&pick_input(4)), Some(4));
    assert_eq!(
        pick(&Input {
            fire: false,
            ..pick_input(2)
        }),
        None,
        "no press, no pick"
    );
    for yaw in [
        0.0,
        1.0,
        3.0,
        -2.5,
        6.2,
        PICK_BASE + 6.0,
        PICK_BASE - 1.0,
        PICK_BASE + 2.5,
        f32::NAN,
        f32::INFINITY,
    ] {
        assert_eq!(
            pick(&Input {
                fire: true,
                yaw,
                ..Default::default()
            }),
            None,
            "{yaw} is not a chime"
        );
    }
}

#[test]
fn the_rounds_twist_harder_as_they_go() {
    let mut rng = StdRng::seed_from_u64(4);
    assert!(
        twist_for(1, &mut rng).is_none(),
        "the first round only teaches"
    );
    for round in 2..=MAX_ROUNDS {
        let twist = twist_for(round, &mut rng);
        assert!(
            matches!(twist.spin.abs(), 1 | 2),
            "round {round} always turns"
        );
        assert_eq!(twist.mirror, round >= 6, "mirrors from round six");
        assert_eq!(
            twist.swap.is_some(),
            round >= 4 && round != 6,
            "swaps from round four"
        );
        if let Some((a, b)) = twist.swap {
            assert!(a < b && (b as usize) < CHIMES);
        }
    }
}

#[test]
fn playing_it_back_right_clears_the_round_and_the_pattern_grows() {
    let mut game = EchoChamber::new(&people(2), 3);
    let mut events = Vec::new();
    to_replay(&mut game, &mut events);
    let lit: Vec<u8> = events
        .iter()
        .filter(|e| e["kind"] == "chime")
        .map(|e| e["chime"].as_u64().unwrap() as u8)
        .collect();
    assert_eq!(
        lit, game.sequence,
        "every chime of the pattern is shown in order"
    );
    run_until(
        &mut game,
        &mut events,
        |g, seat| perfect(g, seat),
        |g| g.phase == Phase::Verdict,
    );
    assert!(
        game.players
            .iter()
            .all(|p| p.status == Status::Cleared && p.lives == LIVES)
    );
    run_until(
        &mut game,
        &mut events,
        |_, _| Input::default(),
        |g| g.round == 2,
    );
    assert_eq!(game.sequence.len(), FIRST_LENGTH + 1);
    assert!(
        game.players
            .iter()
            .all(|p| p.cleared == 1 && p.progress == 0)
    );
}

#[test]
fn hitting_where_a_chime_stood_before_the_turn_costs_a_life() {
    let mut game = EchoChamber::new(&people(2), 9);
    let mut events = Vec::new();
    run_until(
        &mut game,
        &mut events,
        |g, seat| perfect(g, seat),
        |g| g.round == 2,
    );
    to_replay(&mut game, &mut events);
    assert!(!game.twist.is_none());
    let first = game.sequence[0];
    let old = game.before.iter().position(|c| *c == first).unwrap();
    assert_ne!(old, game.slot_of(first), "the turn moved it");
    game.step(
        &[(0, pick_input(old)), (1, pick_input(game.slot_of(first)))],
        TICK_DT,
        &mut events,
    );
    assert_eq!(
        game.lives(0),
        Some(LIVES - 1),
        "the old spot is now another chime"
    );
    assert_eq!(game.players[0].status, Status::Missed);
    assert_eq!(game.lives(1), Some(LIVES), "following the colour is right");
    let progress = game.players[0].progress;
    game.step(
        &[(0, pick_input(game.slot_of(game.sequence[progress])))],
        TICK_DT,
        &mut events,
    );
    assert_eq!(
        game.players[0].progress, progress,
        "a miss sits you out for the rest of the round"
    );
}

#[test]
fn two_misses_knock_you_out_and_the_last_one_standing_wins() {
    let mut game = EchoChamber::new(&people(3), 11);
    let mut events = Vec::new();
    let wrong = |g: &EchoChamber| {
        let right = g.slot_of(g.sequence[0]);
        pick_input((right + 1) % CHIMES)
    };
    for _ in 0..2 {
        to_replay(&mut game, &mut events);
        let miss = wrong(&game);
        game.step(
            &[(0, miss), (1, miss), (2, Input::default())],
            TICK_DT,
            &mut events,
        );
        run_until(
            &mut game,
            &mut events,
            |g, seat| {
                if seat == 2 {
                    perfect(g, seat)
                } else {
                    Input::default()
                }
            },
            |g| g.phase != Phase::Replay,
        );
        run_until(
            &mut game,
            &mut events,
            |_, _| Input::default(),
            |g| g.phase != Phase::Verdict,
        );
    }
    let placed = game.placements().expect("two of three out ends it");
    assert_eq!(placed[0], vec![2], "the one still standing wins");
    assert_eq!(placed[1].len(), 2, "the two who went out together tie");
    assert_eq!(events.iter().filter(|e| e["kind"] == "out").count(), 2);
}

#[test]
fn running_out_of_time_costs_a_life() {
    let mut game = EchoChamber::new(&people(2), 5);
    let mut events = Vec::new();
    to_replay(&mut game, &mut events);
    let seconds = run_until(
        &mut game,
        &mut events,
        |g, seat| {
            if seat == 0 {
                perfect(g, seat)
            } else {
                Input::default()
            }
        },
        |g| g.phase == Phase::Verdict,
    );
    assert!(
        (seconds - replay_seconds(FIRST_LENGTH)).abs() < 0.05,
        "the clock runs out for the idle one"
    );
    assert_eq!(game.lives(1), Some(LIVES - 1));
    assert_eq!(game.lives(0), Some(LIVES));
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "timeout" && e["seat"] == 1)
    );
}

#[test]
fn the_pattern_stays_off_the_wire_until_the_verdict() {
    let mut game = EchoChamber::new(&bots(4, 0.5), 21);
    let mut events = Vec::new();
    let mut checked = 0;
    for _ in 0..(40.0 / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        let snap = game.snapshot();
        if game.phase == Phase::Replay {
            assert!(
                snap["sequence"].is_null(),
                "nobody can read the answer while playing back"
            );
            assert!(snap["lit"].is_null());
            checked += 1;
        }
        if game.phase == Phase::Verdict {
            assert_eq!(snap["sequence"], json!(game.sequence));
        }
    }
    assert!(checked > 0);
    assert!(
        events
            .iter()
            .filter(|e| e["kind"] == "ring")
            .all(|e| e.get("chime").is_none() && e.get("slot").is_none()),
        "a ring tells others how far you got, never which chime"
    );
}

#[test]
fn perfect_players_who_last_every_round_share_first() {
    let mut game = EchoChamber::new(&people(2), 8);
    let mut events = Vec::new();
    let seconds = run_until(
        &mut game,
        &mut events,
        |g, seat| perfect(g, seat),
        |_| false,
    );
    let placed = game.placements().expect("the last round ends it");
    assert_eq!(placed, vec![vec![0, 1]]);
    assert_eq!(game.round, MAX_ROUNDS);
    assert!(seconds < 100.0, "seven full rounds still fit: {seconds}");
}

#[test]
fn someone_who_leaves_is_out_and_ranked_last() {
    let mut game = EchoChamber::new(&people(3), 2);
    let mut events = Vec::new();
    to_replay(&mut game, &mut events);
    for _ in 0..40 {
        let inputs = [(0, perfect(&game, 0)), (1, perfect(&game, 1))];
        game.step(&inputs, TICK_DT, &mut events);
    }
    assert_eq!(game.players[2].out, Some(0));
    assert_eq!(game.snapshot()["players"][2]["status"], "left");
    assert!(events.iter().any(|e| e["kind"] == "left" && e["seat"] == 2));
    // Seat 1 misses every round; seat 0 plays on and wins once 1 is out.
    run_until(
        &mut game,
        &mut events,
        |g, seat| match seat {
            0 => perfect(g, 0),
            1 if g.phase == Phase::Replay && g.players[1].status == Status::Playing => {
                pick_input((g.slot_of(g.sequence[g.players[1].progress]) + 1) % CHIMES)
            }
            _ => Input::default(),
        },
        |_| false,
    );
    let placed = game.placements().expect("it ends once one is left");
    assert_eq!(placed.first(), Some(&vec![0]));
    assert_eq!(
        placed.last(),
        Some(&vec![2]),
        "leaving ranks below going out"
    );
}

#[test]
fn nobody_left_still_ends_the_bout() {
    let mut game = EchoChamber::new(&people(2), 6);
    let mut events = Vec::new();
    for _ in 0..60 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(game.placements().is_none(), "waits for the verdict");
    for _ in 0..(30.0 / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(game.placements().is_some());
}

#[test]
fn bots_fall_for_the_twist_more_when_they_are_clumsy() {
    let twist = Twist {
        spin: 1,
        mirror: true,
        swap: Some((0, 2)),
    };
    assert!(fooled(0.15, &twist) > fooled(1.0, &twist) * 2.0);
    assert_eq!(
        fooled(0.5, &Twist::default()),
        0.0,
        "no twist, nothing to fall for"
    );
    assert!(
        forget(0.15, 6) > forget(1.0, 6) * 4.0,
        "a long pattern is past a clumsy bot"
    );
    assert!(
        forget(1.0, 2) < 0.02,
        "but even a sharp bot slips now and then"
    );
}

struct Sim {
    seconds: f32,
    rounds: u32,
    winners: Vec<u8>,
}

fn simulate(field: &[Contestant], seed: u64) -> Sim {
    let mut game = EchoChamber::new(field, seed);
    let mut events = Vec::new();
    let mut ticks = 0;
    while game.placements().is_none() {
        let inputs: Vec<(u8, Input)> = field
            .iter()
            .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        ticks += 1;
        assert!(ticks < 6_000, "seed {seed} never finished");
    }
    Sim {
        seconds: ticks as f32 * TICK_DT,
        rounds: game.round,
        winners: game.placements().unwrap()[0].clone(),
    }
}

#[test]
fn seeded_bot_bouts_finish_in_time_and_reward_skill() {
    let mut report = String::new();
    for (label, skill) in crate::bout::SKILLS {
        for count in [2u8, 4, 8] {
            let runs: Vec<Sim> = (0..150)
                .map(|seed| simulate(&bots(count, *skill), seed))
                .collect();
            let mean = runs.iter().map(|r| r.seconds).sum::<f32>() / runs.len() as f32;
            let longest = runs.iter().map(|r| r.seconds).fold(0.0, f32::max);
            let rounds = runs.iter().map(|r| r.rounds as f32).sum::<f32>() / runs.len() as f32;
            let solo =
                runs.iter().filter(|r| r.winners.len() == 1).count() as f32 / runs.len() as f32;
            report += &format!(
                "{label:>8} x{count}: mean {mean:5.1}s  longest {longest:5.1}s  rounds {rounds:.1}  sole winner {:.0}%\n",
                solo * 100.0
            );
            assert!(longest < 100.0, "{label} x{count} ran {longest}s");
            if count == 4 {
                assert!((35.0..=90.0).contains(&mean), "{label} x4 averages {mean}s");
            }
        }
    }
    println!("{report}");
    let mut sharp = 0;
    let runs = 300;
    for seed in 0..runs {
        let mut field = bots(4, 0.15);
        field[3].skill = 1.0;
        if simulate(&field, 1000 + seed).winners.contains(&3) {
            sharp += 1;
        }
    }
    println!("a ruthless bot among relaxed ones wins {sharp} of {runs}");
    assert!(sharp > runs / 2, "skill tells: {sharp} of {runs}");
    assert!(
        sharp < runs * 19 / 20,
        "but nobody is a lock: {sharp} of {runs}"
    );
}

#[test]
fn a_full_room_spreads_out_along_the_gallery() {
    let game = EchoChamber::new(&bots(8, 0.5), 1);
    let snap = game.snapshot();
    let spots: Vec<(f32, f32)> = snap["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["x"].as_f64().unwrap() as f32,
                p["z"].as_f64().unwrap() as f32,
            )
        })
        .collect();
    for (i, a) in spots.iter().enumerate() {
        assert!(
            a.1 < 0.0,
            "everyone stands behind the ring, facing the camera side"
        );
        for b in &spots[i + 1..] {
            assert!(
                (a.0 - b.0).hypot(a.1 - b.1) > 1.6,
                "nobody stands on anyone"
            );
        }
    }
}
