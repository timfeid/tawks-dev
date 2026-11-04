use super::*;
use crate::bout::{Stick, TICK_DT};
use crate::room::{Phase, Room};

fn bots(count: u8, skill: f32) -> Vec<Contestant> {
    (0..count)
        .map(|seat| Contestant::bot(seat, skill))
        .collect()
}

fn people(count: u8) -> Vec<Contestant> {
    (0..count).map(Contestant::person).collect()
}

fn play(
    game: &mut BridgeCrew,
    inputs: impl Fn(&mut BridgeCrew) -> Vec<(u8, Input)>,
    ticks: usize,
) -> Vec<Value> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        if game.placements().is_some() {
            break;
        }
        let given = inputs(game);
        game.step(&given, TICK_DT, &mut events);
    }
    events
}

fn all_bots(game: &mut BridgeCrew) -> Vec<(u8, Input)> {
    let seats: Vec<u8> = game
        .crews
        .iter()
        .flat_map(|c| c.hands.iter().filter_map(|h| h.seat))
        .collect();
    seats
        .into_iter()
        .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
        .collect()
}

fn ping_input(kind: Ping) -> Input {
    Input {
        fire: true,
        yaw: kind.code(),
        ..Default::default()
    }
}

fn drive_input(throttle: f32, steer: f32) -> Input {
    Input::from_stick(Stick {
        x: steer,
        z: throttle,
    })
}

/// The seats of the navigator and driver of the first crew this round.
fn crew_seats(game: &BridgeCrew, crew: usize) -> (u8, u8) {
    let nav = game.navigator_slot();
    let hands = &game.crews[crew].hands;
    (hands[nav].seat.unwrap(), hands[1 - nav].seat.unwrap())
}

#[test]
fn pings_round_trip_through_the_fire_yaw_and_junk_is_refused() {
    for ping in Ping::ALL {
        assert_eq!(Ping::from_code(ping.code()), Some(ping));
    }
    for junk in [-1.0, 6.0, 2.5, f32::NAN, f32::INFINITY, 0.3] {
        assert_eq!(Ping::from_code(junk), None, "{junk}");
    }
}

#[test]
fn four_players_make_one_crew_a_side_and_odd_sides_get_stand_ins() {
    let four = BridgeCrew::new(&people(4), 1);
    assert_eq!(four.crews.len(), 2);
    assert!(
        four.crews
            .iter()
            .all(|c| c.hands.iter().all(|h| h.seat.is_some()))
    );
    assert_ne!(four.crews[0].team, four.crews[1].team);

    let six = BridgeCrew::new(&people(6), 1);
    assert_eq!(six.crews.len(), 4, "three a side is two crews a side");
    let stand_ins: Vec<_> = six
        .crews
        .iter()
        .flat_map(|c| c.hands.iter())
        .filter(|h| h.seat.is_none())
        .collect();
    assert_eq!(stand_ins.len(), 2, "one stand-in fills each side");
    assert!(stand_ins.iter().all(|h| h.id >= STAND_IN));

    let lopsided: Vec<Contestant> = (0..4u8)
        .map(|s| Contestant::person(s).on_team(if s == 0 { 1 } else { 0 }))
        .collect();
    let game = BridgeCrew::new(&lopsided, 1);
    for team in 0..2 {
        assert_eq!(
            game.crews.iter().filter(|c| c.team == team).count(),
            2,
            "the sides are evened out"
        );
    }
    let lone = game.find(0).unwrap();
    assert_eq!(game.crews[lone.0].team, 1);

    let two = BridgeCrew::new(&people(2), 1);
    assert_eq!(
        two.crews.len(),
        2,
        "two players each get a stand-in partner"
    );
    let lanes: Vec<f32> = six.crews.iter().map(|c| c.lane).collect();
    for pair in lanes.windows(2) {
        assert!(
            pair[1] - pair[0] >= course::REACH * 2.0,
            "lanes never touch"
        );
    }
}

#[test]
fn crew_body_load_trades_gust_stability_for_weak_plank_time() {
    let mut small = people(4);
    let mut broad = people(4);
    for rider in &mut small { rider.body_width = 0.7; }
    for rider in &mut broad { rider.body_width = 1.4; }
    let mut light = BridgeCrew::new(&small, 19);
    let mut heavy = BridgeCrew::new(&broad, 19);
    assert!(light.crews[0].load < 1.0);
    assert!(heavy.crews[0].load > 1.0);

    let gust = light.course().gusts.first().expect("course has a gust");
    let time = (0..200).map(|tick| tick as f32 * 0.05)
        .find(|&time| gust.state(time).0 == Blow::Blow).unwrap();
    let z = (gust.z0 + gust.z1) * 0.5;
    for game in [&mut light, &mut heavy] {
        game.time = time;
        game.crews[0].cart.x = 0.0;
        game.crews[0].cart.z = z;
        game.crews[0].cart.speed = MAX_SPEED;
        game.drive(0, Input::default(), TICK_DT, &mut Vec::new());
    }
    assert!(light.crews[0].cart.x.abs() > heavy.crews[0].cart.x.abs());

    let weak = light.course().spans.iter().enumerate()
        .find(|(_, span)| span.kind == Kind::Weak)
        .map(|(index, span)| (index, span.weak_delay(), span.at(0.5)))
        .expect("course has a weak plank");
    for game in [&mut light, &mut heavy] {
        game.time = weak.1;
        game.crews[0].cart.x = weak.2.0;
        game.crews[0].cart.z = weak.2.1;
        game.crews[0].cart.speed = 0.0;
        game.crews[0].cart.falling = None;
        game.crews[0].cart.weak_since[weak.0] = Some(0.0);
        game.drive(0, Input::default(), 0.0, &mut Vec::new());
    }
    assert!(!light.crews[0].cart.collapsed[weak.0]);
    assert!(heavy.crews[0].cart.collapsed[weak.0]);
}

#[test]
fn mixed_load_bridge_crews_can_both_win() {
    let mut wins = [0u32; 2];
    for seed in 0..24 {
        let broad_team = (seed % 2) as u8;
        let field = (0..4).map(|seat| {
            let team = seat % 2;
            let mut rider = Contestant::bot(seat, 0.7).on_team(team);
            rider.body_width = if team == broad_team { 1.4 } else { 0.7 };
            rider
        }).collect::<Vec<_>>();
        let mut game = BridgeCrew::new(&field, seed);
        play(&mut game, all_bots, 60 * 200);
        assert!(game.placements().is_some());
        for result in &game.results {
            if let Some(team) = result.winner {
                wins[usize::from(team == broad_team)] += 1;
            }
        }
    }
    println!("Bridge Crew mixed load rounds: small={}, broad={}", wins[0], wins[1]);
    let finished = wins[0] + wins[1];
    assert!(wins[0] * 3 >= finished && wins[1] * 3 >= finished,
        "both load profiles need a meaningful winning share: {wins:?}");
}

#[test]
fn partners_from_a_pairs_party_share_a_cart() {
    let mut field = people(4);
    field[0].ally = Some(0);
    field[2].ally = Some(0);
    field[1].ally = Some(1);
    field[3].ally = Some(1);
    for c in &mut field {
        c.team = c.ally.unwrap();
    }
    let game = BridgeCrew::new(&field, 3);
    let (crew, _) = game.find(0).unwrap();
    assert!(game.crews[crew].hands.iter().any(|h| h.seat == Some(2)));
}

#[test]
fn seats_swap_between_rounds() {
    let mut game = BridgeCrew::new(&bots(4, 0.6), 5);
    assert_eq!(game.role(0), Role::Navigator);
    let first = crew_seats(&game, 0);
    let mut events = Vec::new();
    while game.round == 0 {
        let given = all_bots(&mut game);
        game.step(&given, TICK_DT, &mut events);
    }
    assert!(matches!(game.stage, Stage::Swap { .. }));
    let second = crew_seats(&game, 0);
    assert_eq!(
        (first.0, first.1),
        (second.1, second.0),
        "navigator and driver trade"
    );
    assert!(events.iter().any(|e| e["kind"] == "swap"));
    assert_eq!(
        game.crews[0].cart.z,
        Cart::at_start(&game.courses[1]).z,
        "carts reset for the new course"
    );
}

#[test]
fn only_the_navigator_pings_and_only_the_driver_drives() {
    let mut game = BridgeCrew::new(&people(4), 9);
    let (nav, driver) = crew_seats(&game, 0);
    let start = game.crews[0].cart.z;
    let mut events = Vec::new();
    for _ in 0..60 {
        game.step(&[(nav, drive_input(1.0, 0.0))], TICK_DT, &mut events);
    }
    assert!(
        game.crews[0].cart.z - start < 0.05,
        "the navigator's stick does not drive"
    );
    for _ in 0..30 {
        game.step(&[(driver, ping_input(Ping::Left))], TICK_DT, &mut events);
    }
    assert!(
        game.crews[0].pings.is_empty(),
        "a driver's fire is a boost, not a ping"
    );
    assert!(game.crews[0].cart.boosting);
    game.step(&[(nav, ping_input(Ping::Stop))], TICK_DT, &mut events);
    assert_eq!(game.crews[0].pings.last().map(|p| p.kind), Some(Ping::Stop));
    game.step(&[(nav, ping_input(Ping::Go))], TICK_DT, &mut events);
    assert_eq!(game.crews[0].pings.len(), 1, "pings are rate limited");
    game.step(
        &[(
            nav,
            Input {
                fire: true,
                yaw: 9.0,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );
    assert_eq!(game.crews[0].pings.len(), 1, "an unknown ping is ignored");
}

#[test]
fn driving_off_the_side_falls_and_comes_back_at_the_checkpoint() {
    let mut game = BridgeCrew::new(&people(4), 2);
    let (_, driver) = crew_seats(&game, 0);
    let mut events = Vec::new();
    while !events.iter().any(|e: &Value| e["kind"] == "fall") {
        game.step(&[(driver, drive_input(1.0, 1.0))], TICK_DT, &mut events);
        assert!(game.time < 5.0);
    }
    let fall = events
        .iter()
        .find(|e| e["kind"] == "fall")
        .expect("the cart goes over");
    assert_eq!(fall["cause"], "edge");
    let events = play(&mut game, |_| vec![], (FALL_SECONDS * 60.0) as usize + 5);
    assert!(events.iter().any(|e| e["kind"] == "respawn"));
    let cart = &game.crews[0].cart;
    assert!(cart.falling.is_none());
    assert_eq!(cart.falls, 1);
    assert!(
        BridgeCrew::supports(game.course(), cart, game.time, cart.x, cart.z).is_some(),
        "back on planks"
    );
}

/// Puts the first crew's cart at the start of the first span of `kind`, lined
/// up with it.
fn park_before(game: &mut BridgeCrew, kind: Kind) -> usize {
    let course = game.course().clone();
    let index = course.spans.iter().position(|s| s.kind == kind).unwrap();
    let span = &course.spans[index];
    let cart = &mut game.crews[0].cart;
    let back = 0.6;
    let h = span.heading();
    cart.x = span.a.0 - h.sin() * back;
    cart.z = span.a.1 - h.cos() * back;
    cart.heading = h;
    cart.checkpoint = span.piece;
    index
}

#[test]
fn weak_planks_hold_a_cart_at_speed_but_drop_a_crawler() {
    let mut game = BridgeCrew::new(&people(4), 4);
    let (_, driver) = crew_seats(&game, 0);
    park_before(&mut game, Kind::Weak);
    game.crews[0].cart.speed = MAX_SPEED;
    let events = play(
        &mut game,
        |g| {
            let cart = &g.crews[0].cart;
            let steer = (-(wrap(0.0 - cart.heading)) * 2.0).clamp(-1.0, 1.0);
            vec![(driver, drive_input(1.0, steer))]
        },
        150,
    );
    assert!(events.iter().any(|e| e["kind"] == "creak"));
    assert!(
        !events.iter().any(|e| e["kind"] == "fall"),
        "full speed makes it across: {:?}",
        events
            .iter()
            .filter(|e| e["kind"] == "fall")
            .collect::<Vec<_>>()
    );

    let mut game = BridgeCrew::new(&people(4), 4);
    park_before(&mut game, Kind::Weak);
    let events = play(&mut game, |_| vec![(driver, drive_input(0.35, 0.0))], 300);
    let fall = events
        .iter()
        .find(|e| e["kind"] == "fall")
        .expect("a crawl is too slow");
    assert_eq!(fall["cause"], "weak");
    assert!(events.iter().any(|e| e["kind"] == "collapse"));
}

#[test]
fn an_open_swing_plank_drops_whoever_drives_on() {
    let mut game = BridgeCrew::new(&people(4), 6);
    let (_, driver) = crew_seats(&game, 0);
    park_before(&mut game, Kind::Lift);
    let lift = game.course().lifts[0].clone();
    while !lift.up(game.time + 0.25) {
        game.time += TICK_DT;
    }
    game.crews[0].cart.speed = MAX_SPEED;
    let events = play(&mut game, |_| vec![(driver, drive_input(1.0, 0.0))], 40);
    let fall = events
        .iter()
        .find(|e| e["kind"] == "fall")
        .expect("into the gap");
    assert_eq!(fall["cause"], "lift");
}

#[test]
fn gusts_shove_carts_sideways_while_they_blow() {
    let course = Course::generate(11);
    let gust = &course.gusts[0];
    let mut seen = [false; 3];
    let mut dirs = std::collections::HashSet::new();
    for tick in 0..2000 {
        let (blow, dir) = gust.state(tick as f32 * 0.05);
        seen[blow as usize] = true;
        dirs.insert(dir as i32);
    }
    assert_eq!(seen, [true; 3], "calm, warning and blowing all happen");
    assert_eq!(dirs.len(), 2, "from both sides");
}

#[test]
fn someone_who_walks_away_gets_a_bot_in_their_seat() {
    let mut game = BridgeCrew::new(&people(4), 8);
    let events = play(&mut game, |_| vec![], 60 * 12);
    let handed: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "autopilot" && e["on"] == true)
        .collect();
    assert_eq!(handed.len(), 4, "every idle seat is covered, drivers first");
    assert!(
        game.crews.iter().any(|c| c.cart.z > 15.0),
        "and the carts get going"
    );
    let (nav, _) = crew_seats(&game, 0);
    let mut events = Vec::new();
    game.step(&[(nav, ping_input(Ping::Go))], TICK_DT, &mut events);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "autopilot" && e["on"] == false),
        "touching a control takes it back"
    );
    assert!(
        !game.crews[0]
            .hands
            .iter()
            .find(|h| h.seat == Some(nav))
            .unwrap()
            .autopilot
    );
}

#[test]
fn a_dead_heat_over_both_rounds_is_a_shared_place() {
    let mut game = BridgeCrew::new(&bots(4, 0.5), 1);
    game.results = vec![
        RoundResult {
            winner: Some(0),
            times: [20.0, 22.0],
        },
        RoundResult {
            winner: Some(1),
            times: [22.0, 20.0],
        },
    ];
    let mut events = Vec::new();
    game.settle(&mut events);
    assert_eq!(game.placements().unwrap().len(), 1);
    assert_eq!(game.placements().unwrap()[0].len(), 4);

    game.results[1].times = [21.0, 20.0];
    game.settle(&mut events);
    let placed = game.placements().unwrap();
    assert_eq!(placed.len(), 2, "split rounds go to the faster total");
    assert!(
        placed[0]
            .iter()
            .all(|s| game.crews[game.find(*s).unwrap().0].team == 0)
    );
}

#[test]
fn stand_ins_never_show_up_in_the_placements() {
    let mut game = BridgeCrew::new(&bots(3, 0.6), 21);
    play(&mut game, all_bots, 60 * 200);
    let placed: Vec<u8> = game.placements().expect("finished").concat();
    let mut sorted = placed.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2]);
}

#[test]
fn a_room_plays_bridge_crew_to_results_with_bots_filling_the_crews() {
    let mut room = Room::new("lobby-bridge", 4);
    room.enable_native_mode();
    room.connect("a", "A", "").unwrap();
    room.add_bot("a").unwrap();
    room.add_bot("a").unwrap();
    room.choose_game("a", KEY).unwrap();
    room.start("a").unwrap();
    let mut saw_playing = false;
    for _ in 0..(60 * 240) {
        if room.phase == Phase::Playing {
            saw_playing = true;
            let snap = room.snapshot().unwrap();
            assert_eq!(snap["crews"].as_array().unwrap().len(), 2);
        }
        if matches!(room.phase, Phase::Results { .. }) {
            break;
        }
        let _ = room.ready("a");
        room.step();
    }
    assert!(saw_playing);
    assert!(
        matches!(room.phase, Phase::Results { .. }),
        "the bout ends: {:?}",
        room.phase
    );
}

#[derive(Default, Debug)]
struct Tally {
    bouts: u32,
    rounds: u32,
    timeouts: u32,
    team_wins: [u32; 2],
    ties: u32,
    round_time: f32,
    bout_time: f32,
    falls: [u32; 4],
    finished: u32,
    pings: u32,
    longest: f32,
}

fn simulate(players: u8, skill: f32, seeds: std::ops::Range<u64>) -> Tally {
    let mut tally = Tally::default();
    for seed in seeds {
        let field: Vec<Contestant> = (0..players)
            .map(|seat| {
                Contestant::bot(
                    seat,
                    (skill + (seat as f32 * 0.37 + seed as f32 * 0.11).sin() * 0.15)
                        .clamp(0.0, 1.0),
                )
            })
            .collect();
        let mut game = BridgeCrew::new(&field, seed);
        let mut events = Vec::new();
        let mut ticks = 0u32;
        while game.placements().is_none() {
            let given = all_bots(&mut game);
            game.step(&given, TICK_DT, &mut events);
            ticks += 1;
            assert!(ticks < 60 * 200, "seed {seed} never ends");
        }
        tally.bouts += 1;
        tally.bout_time += ticks as f32 * TICK_DT;
        for event in &events {
            match event["kind"].as_str().unwrap_or("") {
                "across" => {
                    tally.rounds += 1;
                    if event["timeout"] == true {
                        tally.timeouts += 1;
                    }
                }
                "fall" => {
                    let index = match event["cause"].as_str().unwrap() {
                        "edge" => 0,
                        "weak" => 1,
                        "lift" => 2,
                        _ => 3,
                    };
                    tally.falls[index] += 1;
                }
                "finish" => {
                    tally.finished += 1;
                    let at = event["time"].as_f64().unwrap() as f32;
                    tally.longest = tally.longest.max(at);
                }
                "ping" => tally.pings += 1,
                "winner" => match event["team"].as_u64() {
                    Some(team) => tally.team_wins[team as usize] += 1,
                    None => tally.ties += 1,
                },
                _ => {}
            }
        }
        for result in &game.results {
            tally.round_time += result.times.iter().cloned().fold(f32::MAX, f32::min);
        }
    }
    tally
}

#[test]
fn seeded_bot_bouts_finish_in_time_and_either_side_can_win() {
    for (players, skill) in [(4u8, 0.4f32), (6, 0.4), (8, 0.7), (4, 0.15), (4, 1.0)] {
        let tally = simulate(players, skill, 0..40);
        let per_round = tally.round_time / tally.rounds as f32;
        let bout = tally.bout_time / tally.bouts as f32;
        println!(
            "{players} bots at {skill}: bout {bout:.1}s, winning round {per_round:.1}s, timeouts {}/{}, wins {:?} ties {}, falls edge/weak/lift/dead {:?} per bout {:.1}, pings per bout {:.0}, slowest finish {:.1}s",
            tally.timeouts,
            tally.rounds,
            tally.team_wins,
            tally.ties,
            tally.falls,
            tally.falls.iter().sum::<u32>() as f32 / tally.bouts as f32,
            tally.pings as f32 / tally.bouts as f32,
            tally.longest,
        );
        assert_eq!(tally.rounds, tally.bouts * ROUNDS as u32);
        assert!(
            (40.0..=100.0).contains(&bout),
            "{players}@{skill}: a bout lasts {bout:.1}s"
        );
        assert!(
            tally.timeouts * 4 <= tally.rounds,
            "{players}@{skill}: most rounds end at the line, {} timeouts",
            tally.timeouts
        );
        assert!(
            tally.team_wins[0] > 0 && tally.team_wins[1] > 0,
            "{players}@{skill}: {:?}",
            tally.team_wins
        );
    }
}

#[test]
fn better_crews_beat_worse_ones() {
    let mut sharp_wins = 0;
    for seed in 0..60 {
        let field: Vec<Contestant> = (0..4u8)
            .map(|seat| {
                Contestant::bot(seat, if seat % 2 == 0 { 0.75 } else { 0.35 }).on_team(seat % 2)
            })
            .collect();
        let mut game = BridgeCrew::new(&field, seed);
        play(&mut game, all_bots, 60 * 200);
        let first = &game.placements().unwrap()[0];
        if first.contains(&0) && first.len() == 2 {
            sharp_wins += 1;
        }
    }
    assert!(
        sharp_wins >= 38,
        "the sharp crew wins most: {sharp_wins}/60"
    );
    assert!(sharp_wins < 60, "but not every time: {sharp_wins}/60");
}

#[test]
fn bots_listen_to_a_person_navigating() {
    let mut crossed = 0;
    let mut wrong_turns = 0;
    for seed in 0..20 {
        let field = vec![
            Contestant::person(0),
            Contestant::bot(1, 0.5),
            Contestant::bot(2, 0.5),
            Contestant::bot(3, 0.5),
        ];
        let mut game = BridgeCrew::new(&field, seed);
        let (crew, _) = game.find(0).unwrap();
        // A helpful person: calls each fork as it comes, and nothing else.
        let mut events = Vec::new();
        let mut called = std::collections::HashSet::new();
        while game.round == 0 && game.placements().is_none() {
            let mut given = all_bots(&mut game);
            given.retain(|(s, _)| *s != 0);
            let cart = game.crews[crew].cart.clone();
            for (index, fork) in game.course().forks.clone().iter().enumerate() {
                if fork.split.1 - cart.z < 10.0 && cart.z < fork.split.1 && called.insert(index) {
                    given.push((
                        0,
                        ping_input(if fork.good_left {
                            Ping::Left
                        } else {
                            Ping::Right
                        }),
                    ));
                }
            }
            if !given.iter().any(|(s, _)| *s == 0) {
                given.push((
                    0,
                    Input {
                        stick: Stick { x: 0.3, z: 0.0 },
                        ..Default::default()
                    },
                ));
            }
            game.step(&given, TICK_DT, &mut events);
        }
        if game.crews[crew].cart.finished_at.is_some()
            || game.results[0].winner == Some(game.crews[crew].team)
        {
            crossed += 1;
        }
        wrong_turns += events
            .iter()
            .filter(|e| e["kind"] == "fall" && e["crew"] == crew && e["cause"] == "dead")
            .count();
    }
    assert!(
        wrong_turns <= 4,
        "called forks are taken: {wrong_turns} wrong turns"
    );
    assert!(crossed >= 5, "{crossed}");
}

#[test]
#[ignore]
fn where_bots_fall() {
    let mut by_piece = std::collections::BTreeMap::new();
    for seed in 0..20 {
        let mut game = BridgeCrew::new(&bots(4, 0.4), seed);
        let mut events = Vec::new();
        while game.round == 0 {
            let before: Vec<usize> = game
                .crews
                .iter()
                .map(|c| c.cart.span.unwrap_or(0))
                .collect();
            let given = all_bots(&mut game);
            let start = events.len();
            game.step(&given, TICK_DT, &mut events);
            for e in &events[start..] {
                if e["kind"] == "fall" {
                    let crew = e["crew"].as_u64().unwrap() as usize;
                    let span = &game.courses[0].spans[before[crew]];
                    let piece = game.courses[0].pieces[span.piece];
                    let cart = &game.crews[crew].cart;
                    let (t, off) = span.project(cart.x, cart.z);
                    *by_piece
                        .entry(format!("{piece:?}/{:?}", span.kind))
                        .or_insert(0) += 1;
                    if seed < 2 {
                        println!(
                            "seed {seed} crew {crew} {piece:?} span {} t {t:.2} off {off:.2} heading {:.2} speed {:.1} gust {}",
                            before[crew], cart.heading, cart.speed, cart.gusted
                        );
                    }
                }
            }
        }
    }
    println!("{by_piece:?}");
}
