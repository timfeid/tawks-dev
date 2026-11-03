use super::*;
use crate::bout::{Contestant, TICK_DT};

fn people(count: u8) -> Vec<Contestant> {
    (0..count).map(Contestant::person).collect()
}

fn bots(count: u8, skill: f32) -> Vec<Contestant> {
    (0..count)
        .map(|seat| Contestant::bot(seat, skill))
        .collect()
}

/// A gallery with seat 0 as the ghost.
fn gallery(count: u8) -> GhostGallery {
    let mut game = GhostGallery::new(&people(count), 11);
    game.rotate_lead(&(1..count).collect::<Vec<_>>());
    assert_eq!(game.ghost_seat(), Some(0));
    game
}

fn idle(game: &GhostGallery) -> Vec<(u8, Input)> {
    game.seats.iter().map(|s| (*s, Input::default())).collect()
}

fn run(
    game: &mut GhostGallery,
    seconds: f32,
    mut each: impl FnMut(&GhostGallery, &mut Vec<(u8, Input)>),
) -> Vec<Value> {
    let mut events = Vec::new();
    for _ in 0..(seconds / TICK_DT).round() as usize {
        let mut inputs = idle(game);
        each(game, &mut inputs);
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    events
}

fn put_ghost(game: &mut GhostGallery, at: (f32, f32)) {
    let ghost = game.ghost.as_mut().unwrap();
    ghost.run = Runner::at(at.0, at.1);
}

fn put_guard(game: &mut GhostGallery, index: usize, at: (f32, f32), yaw: f32) {
    game.guards[index].run = Runner::at(at.0, at.1);
    game.guards[index].yaw = yaw;
}

/// Every guard parked in a far corner looking at the wall.
fn park_guards(game: &mut GhostGallery) {
    let s = game.scale;
    for i in 0..game.guards.len() {
        put_guard(
            game,
            i,
            (-2.0 * s + i as f32 * 0.9 * s, -11.5 * s),
            std::f32::consts::PI,
        );
    }
}

fn hold(seat: u8, input: Input) -> impl FnMut(&GhostGallery, &mut Vec<(u8, Input)>) {
    move |_, inputs| {
        for (s, i) in inputs.iter_mut() {
            if *s == seat {
                *i = input;
            }
        }
    }
}

#[test]
fn it_is_registered_for_three_to_six_players() {
    let spec = crate::bout::find(KEY).expect("registered");
    assert_eq!(spec.name, "Ghost in the Gallery");
    assert!(!spec.fits(2) && spec.fits(3) && spec.fits(6) && !spec.fits(7));
    let game = crate::bout::create(KEY, &people(4), 3).unwrap();
    assert_eq!(game.snapshot()["game"], KEY);
    assert_eq!(game.setup()["paintings"].as_array().unwrap().len(), 8);
    let six = GhostGallery::new(&people(6), 3);
    assert_eq!(six.paintings.len(), 12, "more guards, more to guard");
    assert!(
        six.floor.half_x > game.setup()["half_x"].as_f64().unwrap() as f32,
        "and a bigger floor"
    );
}

#[test]
fn guards_are_never_sent_the_ghost_unless_a_beam_finds_it() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let s = game.scale;
    put_ghost(&mut game, (14.0 * s, 8.0 * s));
    run(&mut game, 0.2, |_, _| {});
    let guard = game.snapshot_for(Some(1)).unwrap();
    assert_eq!(guard["you"], "guard");
    assert_eq!(guard["ghost"]["visible"], false);
    assert!(
        guard["ghost"]["x"].is_null() && guard["ghost"]["z"].is_null(),
        "no position leaks: {}",
        guard["ghost"]
    );
    assert!(guard["ghost"]["stealing"].is_null() && guard["ghost"]["reach"].is_null());
    assert!(
        guard["players"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["seat"] != 0),
        "the ghost is not a player body"
    );
    let ghost = game.snapshot_for(Some(0)).unwrap();
    assert_eq!(ghost["you"], "ghost");
    assert!(ghost["ghost"]["x"].is_number());
    let watching = game.snapshot_for(None).unwrap();
    assert_eq!(watching["you"], "spectator");
    assert!(
        watching["ghost"]["x"].is_number(),
        "spectators see everything"
    );

    let at = (
        game.ghost.as_ref().unwrap().run.x,
        game.ghost.as_ref().unwrap().run.z,
    );
    let from = (at.0 - 5.0, at.1);
    put_guard(&mut game, 0, from, heading(from, at));
    run(
        &mut game,
        0.1,
        hold(
            1,
            Input {
                yaw: heading(from, at),
                ..Default::default()
            },
        ),
    );
    let guard = game.snapshot_for(Some(2)).unwrap();
    assert_eq!(guard["ghost"]["lit"], true);
    assert!(
        guard["ghost"]["x"].is_number(),
        "it glints in the light for every guard"
    );
}

#[test]
fn lifting_a_painting_takes_holding_still_and_gives_itself_away() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let frame = (0..game.paintings.len())
        .max_by(|a, b| {
            let corner = (0.0, -11.5 * game.scale);
            let d = |i: usize| distance(game.paintings[i].spot(), corner);
            d(*a).partial_cmp(&d(*b)).unwrap()
        })
        .unwrap();
    let spot = game.paintings[frame].spot();
    put_ghost(&mut game, spot);
    let steal = Input {
        crouch: true,
        stick: Stick { x: 1.0, z: 0.0 },
        ..Default::default()
    };
    let events = run(&mut game, STEAL_SECONDS * 0.5, hold(0, steal));
    let ghost = game.ghost.as_ref().unwrap();
    assert_eq!(ghost.stealing, Some(frame));
    assert!(
        distance((ghost.run.x, ghost.run.z), spot) < 1e-3,
        "rooted while lifting"
    );
    assert!(
        events
            .iter()
            .filter(|e| e["kind"] == "creak" && e["frame"] == frame)
            .count()
            >= 1
    );
    let guard = game.snapshot_for(Some(1)).unwrap();
    assert!(
        guard["paintings"][frame]["sway"].as_f64().unwrap() > 0.5,
        "the frame sways for everyone"
    );
    assert!(
        guard["paintings"][frame]["progress"].is_null(),
        "but only the ghost sees how far along it is"
    );

    let events = run(&mut game, STEAL_SECONDS * 0.6, hold(0, steal));
    assert!(game.paintings[frame].stolen);
    let stolen = events
        .iter()
        .find(|e| e["kind"] == "stolen")
        .expect("a stolen event");
    assert_eq!(stolen["count"], 1);
    assert_eq!(stolen["title"], game.paintings[frame].title);
    assert_eq!(game.ghost.as_ref().unwrap().stolen, 1);
}

#[test]
fn holding_the_steal_key_away_from_a_frame_or_behind_it_does_nothing() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let s = game.scale;
    put_ghost(&mut game, (-10.0 * s, 4.0 * s));
    let steal = Input {
        crouch: true,
        ..Default::default()
    };
    run(&mut game, 1.0, hold(0, steal));
    assert!(game.paintings.iter().all(|p| p.progress == 0.0));
    let frame = game.paintings[0].clone();
    let behind = (
        frame.hook.x - frame.hook.nx * 0.9,
        frame.hook.z - frame.hook.nz * 0.9,
    );
    assert_eq!(game.reachable(behind), None, "not through the wall");
}

#[test]
fn half_lifted_paintings_settle_back_slowly() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let spot = game.paintings[0].spot();
    put_ghost(&mut game, spot);
    run(
        &mut game,
        1.5,
        hold(
            0,
            Input {
                crouch: true,
                ..Default::default()
            },
        ),
    );
    let lifted = game.paintings[0].progress;
    assert!(lifted > 0.45);
    run(&mut game, 1.0, |_, _| {});
    let after = game.paintings[0].progress;
    assert!(
        after < lifted && after > lifted - STEAL_DECAY * 1.1,
        "{lifted} -> {after}"
    );
}

#[test]
fn a_beam_held_for_a_second_and_a_half_catches_the_ghost() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let s = game.scale;
    let at = (-12.0 * s, 6.0 * s);
    put_ghost(&mut game, at);
    let from = (at.0 + 6.0, at.1);
    put_guard(&mut game, 0, from, heading(from, at));
    let aim = Input {
        yaw: heading(from, at),
        ..Default::default()
    };
    run(&mut game, CATCH_SECONDS - 0.2, hold(1, aim));
    assert!(game.placements().is_none(), "not yet");
    let events = run(&mut game, 0.4, hold(1, aim));
    let caught = events
        .iter()
        .find(|e| e["kind"] == "caught")
        .expect("caught");
    assert_eq!(caught["by"], json!([1]));
    assert_eq!(game.outcome.as_ref().unwrap().reason, "caught");
    let places = game.placements().unwrap();
    assert_eq!(places[0], vec![1, 2, 3], "the guards win together");
    assert_eq!(places[1], vec![0]);
    assert_eq!(game.snapshot()["result"]["winner"], "guards");
}

#[test]
fn walls_stop_the_light_and_the_meter_drains_in_the_dark() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let s = game.scale;
    let at = (-15.0 * s, 2.0 * s);
    put_ghost(&mut game, at);
    let from = (-15.0 * s, -2.0 * s);
    put_guard(&mut game, 0, from, heading(from, at));
    let aim = Input {
        yaw: heading(from, at),
        ..Default::default()
    };
    run(&mut game, 3.0, hold(1, aim));
    assert_eq!(
        game.ghost.as_ref().unwrap().meter,
        0.0,
        "the half wall is between them"
    );
    let open = (-10.0 * s, 2.0 * s);
    put_guard(&mut game, 0, open, heading(open, at));
    let aim = Input {
        yaw: heading(open, at),
        ..Default::default()
    };
    run(&mut game, 1.0, hold(1, aim));
    let meter = game.ghost.as_ref().unwrap().meter;
    assert!(meter > 0.6 && meter < 1.0);
    put_guard(&mut game, 0, open, heading(open, at) + 1.5);
    run(
        &mut game,
        0.3,
        hold(
            1,
            Input {
                yaw: heading(open, at) + 1.5,
                ..Default::default()
            },
        ),
    );
    let drained = game.ghost.as_ref().unwrap().meter;
    assert!(
        (meter - drained - CATCH_DECAY * 0.3).abs() < 0.05,
        "{meter} -> {drained}"
    );
}

#[test]
fn two_beams_catch_faster_than_one() {
    let time_to_catch = |beams: usize| {
        let mut game = gallery(4);
        park_guards(&mut game);
        let s = game.scale;
        let at = (0.0, 6.0 * s);
        put_ghost(&mut game, at);
        let spots = [(-4.0, 6.0 * s), (4.0, 6.0 * s)];
        for (i, from) in spots.iter().enumerate().take(beams) {
            put_guard(&mut game, i, *from, heading(*from, at));
        }
        let yaws: Vec<f32> = spots.iter().map(|f| heading(*f, at)).collect();
        let mut ticks = 0;
        run(&mut game, 5.0, |_, inputs| {
            ticks += 1;
            for (seat, input) in inputs.iter_mut() {
                if (1..=beams as u8).contains(seat) {
                    input.yaw = yaws[*seat as usize - 1];
                }
            }
        });
        assert_eq!(game.outcome.as_ref().unwrap().reason, "caught");
        ticks
    };
    assert!(time_to_catch(2) < time_to_catch(1));
}

#[test]
fn three_paintings_win_it_for_the_ghost_even_on_the_tick_it_is_caught() {
    let mut game = gallery(4);
    park_guards(&mut game);
    for p in game.paintings.iter_mut().take(2) {
        p.stolen = true;
    }
    game.ghost.as_mut().unwrap().stolen = 2;
    let frame = 2;
    let spot = game.paintings[frame].spot();
    put_ghost(&mut game, spot);
    game.paintings[frame].progress = 1.0 - TICK_DT / STEAL_SECONDS * 0.5;
    game.ghost.as_mut().unwrap().meter = 0.999;
    let from = (
        spot.0 + game.paintings[frame].hook.nx * 4.0,
        spot.1 + game.paintings[frame].hook.nz * 4.0,
    );
    put_guard(&mut game, 0, from, heading(from, spot));
    let events = run(
        &mut game,
        TICK_DT,
        hold(
            0,
            Input {
                crouch: true,
                ..Default::default()
            },
        ),
    );
    assert!(events.iter().any(|e| e["kind"] == "escaped"));
    assert_eq!(game.outcome.as_ref().unwrap().winner, Side::Ghost);
    assert_eq!(game.placements().unwrap(), vec![vec![0], vec![1, 2, 3]]);
}

#[test]
fn dawn_wins_it_for_the_guards() {
    let mut game = gallery(4);
    park_guards(&mut game);
    run(&mut game, ROUND_SECONDS + 1.0, |_, _| {});
    assert_eq!(game.outcome.as_ref().unwrap().reason, "dawn");
    assert_eq!(game.placements().unwrap()[0], vec![1, 2, 3]);
}

#[test]
fn a_rattle_shakes_a_far_frame_like_a_theft_and_then_has_to_recharge() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let ghost = game.ghost.as_ref().unwrap();
    let at = (ghost.run.x, ghost.run.z);
    let (frame, hook) = game
        .paintings
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.hook))
        .filter(|(_, h)| distance(at, (h.x, h.z)) < RATTLE_RANGE - 1.0)
        .max_by(|a, b| {
            distance(at, (a.1.x, a.1.z))
                .partial_cmp(&distance(at, (b.1.x, b.1.z)))
                .unwrap()
        })
        .unwrap();
    let rattle = Input {
        fire: true,
        yaw: heading(at, (hook.x, hook.z)),
        ..Default::default()
    };
    let events = run(&mut game, 0.2, hold(0, rattle));
    assert!(events.is_empty(), "the opening cooldown holds it back");
    game.ghost.as_mut().unwrap().rattle = 0.0;
    let events = run(&mut game, 1.0, hold(0, rattle));
    let creaks: Vec<_> = events.iter().filter(|e| e["kind"] == "creak").collect();
    assert_eq!(creaks.len(), 1, "one rattle per cooldown");
    assert_eq!(creaks[0]["frame"], frame);
    assert_eq!(
        creaks[0].as_object().unwrap().len(),
        2,
        "a creak looks the same as a real theft"
    );
    assert_eq!(game.paintings[frame].progress, 0.0);
    assert!(game.paintings[frame].sway > 0.0);
}

#[test]
fn the_ghost_leaving_hands_it_to_the_guards_and_leavers_place_last() {
    let mut game = gallery(4);
    park_guards(&mut game);
    let mut events = Vec::new();
    for _ in 0..30 {
        let inputs: Vec<(u8, Input)> = [0, 1, 2].iter().map(|s| (*s, Input::default())).collect();
        game.step(&inputs, TICK_DT, &mut events);
    }
    assert!(
        game.placements().is_none(),
        "a guard leaving does not end it"
    );
    for _ in 0..40 {
        let inputs: Vec<(u8, Input)> = [1, 2].iter().map(|s| (*s, Input::default())).collect();
        game.step(&inputs, TICK_DT, &mut events);
    }
    assert_eq!(game.outcome.as_ref().unwrap().reason, "fled");
    assert_eq!(
        game.placements().unwrap(),
        vec![vec![1, 2], vec![0], vec![3]]
    );
}

#[test]
fn the_ghost_wins_if_every_guard_leaves() {
    let mut game = gallery(3);
    let mut events = Vec::new();
    for _ in 0..40 {
        game.step(&[(0, Input::default())], TICK_DT, &mut events);
    }
    assert_eq!(game.outcome.as_ref().unwrap().reason, "unguarded");
    assert_eq!(game.placements().unwrap()[0], vec![0]);
}

#[test]
fn the_ghost_goes_round_the_party() {
    assert_eq!(next_ghost(&[0, 1, 2, 3], &[]), vec![0, 1, 2, 3]);
    assert_eq!(next_ghost(&[0, 1, 2, 3], &[2]), vec![0, 1, 3]);
    assert_eq!(
        next_ghost(&[0, 1, 2, 3], &[2, 0, 1, 3]),
        vec![2],
        "everyone had a turn, the earliest goes again"
    );
    assert_eq!(
        next_ghost(&[0, 1, 5], &[3, 3, 1]),
        vec![0, 5],
        "seats that left do not count"
    );
    let mut earlier = Vec::new();
    for round in 0..8u64 {
        let mut game = GhostGallery::new(&people(4), round * 31);
        game.rotate_lead(&earlier);
        earlier.extend(game.lead());
    }
    for seat in 0..4u8 {
        assert_eq!(
            earlier.iter().filter(|s| **s == seat).count(),
            2,
            "{earlier:?}"
        );
    }
}

#[test]
fn a_room_rotates_the_ghost_from_bout_to_bout() {
    use crate::room::{Phase, Room};
    let mut room = Room::new("ghost-room", 9);
    room.connect("host", "Host", "").unwrap();
    for _ in 0..3 {
        room.add_bot("host").unwrap();
    }
    room.choose_game("host", KEY).unwrap();
    let mut ghosts = Vec::new();
    for _ in 0..4 {
        room.start("host").unwrap();
        room.ready("host").unwrap();
        let mut seen = None;
        for _ in 0..(60 * 120) {
            room.step();
            if matches!(room.phase, Phase::Playing) && seen.is_none() {
                seen = room.snapshot().and_then(|s| s["ghost"]["seat"].as_u64());
            }
            if matches!(room.phase, Phase::Lobby) {
                break;
            }
        }
        ghosts.push(seen.expect("the bout was played"));
        assert!(
            matches!(room.phase, Phase::Lobby),
            "back to the lobby: {:?}",
            room.phase
        );
    }
    let mut sorted = ghosts.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec![0, 1, 2, 3],
        "four bouts, four different ghosts: {ghosts:?}"
    );
    let seat = room.seat_of("host");
    let guard_view = room.snapshot_for(seat).unwrap();
    assert!(guard_view["you"] == "guard" || guard_view["you"] == "ghost");
}

#[test]
fn guard_bots_do_not_know_where_an_unlit_ghost_is() {
    let inputs_with_ghost_at = |at: (f32, f32)| {
        let mut game = GhostGallery::new(&bots(4, 0.7), 5);
        game.rotate_lead(&[1, 2, 3]);
        park_guards(&mut game);
        put_ghost(&mut game, at);
        let mut seen = Vec::new();
        let mut events = Vec::new();
        for _ in 0..240 {
            let inputs: Vec<(u8, Input)> = (1..4)
                .map(|s| (s, game.bot_input(s, TICK_DT)))
                .chain([(0, Input::default())])
                .collect();
            seen.push(inputs.clone());
            game.step(&inputs, TICK_DT, &mut events);
        }
        assert!(!game.ghost.as_ref().unwrap().lit);
        seen
    };
    let s = crowd_scale(4);
    assert_eq!(
        inputs_with_ghost_at((15.0 * s, 9.0 * s)),
        inputs_with_ghost_at((-15.0 * s, 9.0 * s))
    );
}

struct Tally {
    games: u32,
    ghost_wins: u32,
    reasons: HashMap<&'static str, u32>,
    seconds: f32,
    stolen: u32,
    rattles: u32,
    spotted: u32,
}

fn simulate(players: u8, skill: f32, games: u64) -> Tally {
    let mut tally = Tally {
        games: 0,
        ghost_wins: 0,
        reasons: HashMap::new(),
        seconds: 0.0,
        stolen: 0,
        rattles: 0,
        spotted: 0,
    };
    for seed in 0..games {
        let seats = bots(players, skill);
        let mut game = GhostGallery::new(&seats, seed * 7919 + 1);
        let mut events = Vec::new();
        for _ in 0..(60 * 100) {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        let outcome = game.outcome.clone().expect("every bout finishes");
        tally.games += 1;
        tally.ghost_wins += u32::from(outcome.winner == Side::Ghost);
        *tally.reasons.entry(outcome.reason).or_default() += 1;
        tally.seconds += game.time;
        tally.stolen += game.ghost.as_ref().unwrap().stolen;
        let creaks = events.iter().filter(|e| e["kind"] == "creak").count();
        let thefts: usize = game.paintings.iter().filter(|p| p.stolen).count();
        tally.rattles += creaks.saturating_sub(thefts * 3) as u32;
        tally.spotted += events.iter().filter(|e| e["kind"] == "spotted").count() as u32;
    }
    tally
}

#[test]
fn bots_play_both_sides_to_a_close_contest() {
    let tallies: Vec<(u8, Tally)> = (3..=6u8)
        .map(|players| (players, simulate(players, 0.4, 60)))
        .collect();
    for (players, tally) in &tallies {
        eprintln!(
            "{players} players: ghost wins {:.0}%, {:.1}s mean, {:.1} paintings a bout, {:.1} sightings, {:?}",
            tally.ghost_wins as f32 / tally.games as f32 * 100.0,
            tally.seconds / tally.games as f32,
            tally.stolen as f32 / tally.games as f32,
            tally.spotted as f32 / tally.games as f32,
            tally.reasons
        );
    }
    for (players, tally) in &tallies {
        let rate = tally.ghost_wins as f32 / tally.games as f32;
        assert!(
            (0.3..=0.7).contains(&rate),
            "{players} players: ghost wins {rate}"
        );
        assert!(
            tally.reasons.get("caught").copied().unwrap_or(0) > tally.games / 10,
            "guards make catches"
        );
        assert!(tally.stolen > tally.games, "the ghost steals");
        assert!(
            tally.seconds / tally.games as f32 > 20.0,
            "bouts are not over at once"
        );
    }
}

#[test]
fn a_sharper_ghost_wins_more() {
    let wins = |ghost_skill: f32| {
        let mut wins = 0;
        for seed in 0..40u64 {
            let mut seats = bots(4, 0.4);
            let mut game = GhostGallery::new(&seats, seed);
            let ghost = game.ghost_seat().unwrap();
            seats[ghost as usize].skill = ghost_skill;
            let mut game_sharp = GhostGallery::new(&seats, seed);
            std::mem::swap(&mut game, &mut game_sharp);
            let mut events = Vec::new();
            for _ in 0..(60 * 100) {
                let inputs: Vec<(u8, Input)> = seats
                    .iter()
                    .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
                    .collect();
                game.step(&inputs, TICK_DT, &mut events);
                if game.placements().is_some() {
                    break;
                }
            }
            wins += u32::from(game.outcome.as_ref().unwrap().winner == Side::Ghost);
        }
        wins
    };
    let (sharp, clumsy) = (wins(1.0), wins(0.0));
    assert!(sharp > clumsy, "sharp {sharp} against clumsy {clumsy}");
}

#[test]
#[ignore]
fn trace() {
    let players: u8 = std::env::var("GHOST_PLAYERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let seed: u64 = std::env::var("GHOST_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let seats = bots(players, 0.4);
    let mut game = GhostGallery::new(&seats, seed);
    let mut events = Vec::new();
    for tick in 0..(60 * 100) {
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|c| (c.seat, game.bot_input(c.seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if tick
            % std::env::var("GHOST_EVERY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30)
            == 0
        {
            let g = game.ghost.as_ref().unwrap();
            let guards: Vec<String> = game
                .guards
                .iter()
                .map(|g| format!("({:.1},{:.1} y{:.1})", g.run.x, g.run.z, g.yaw))
                .collect();
            eprintln!(
                "t{:.1} ghost ({:.1},{:.1}) lit {} meter {:.2} steal {:?} stolen {} | {}",
                game.time,
                g.run.x,
                g.run.z,
                g.lit,
                g.meter,
                g.stealing,
                g.stolen,
                guards.join(" ")
            );
        }
        for e in events.drain(..) {
            eprintln!("  {e}");
        }
        if game.placements().is_some() {
            break;
        }
    }
    eprintln!("{:?}", game.outcome);
}

/// `cargo test -p party --lib gallery::tests::balance_report -- --ignored --nocapture`
#[test]
#[ignore]
fn balance_report() {
    for skill in [0.15, 0.4, 0.7, 1.0] {
        for players in 3..=6u8 {
            let tally = simulate(players, skill, 200);
            eprintln!(
                "skill {skill} {players}p: ghost {:.0}%, {:.1}s, {:.2} paintings, {:.1} sightings, {:?}",
                tally.ghost_wins as f32 / tally.games as f32 * 100.0,
                tally.seconds / tally.games as f32,
                tally.stolen as f32 / tally.games as f32,
                tally.spotted as f32 / tally.games as f32,
                tally.reasons
            );
        }
    }
}
