use super::*;
use crate::room::TICK_DT;

fn field(n: u8, bots: bool) -> Vec<Contestant> {
    (0..n)
        .map(|seat| Contestant {
            seat,
            is_bot: bots,
            skill: crate::bout::DEFAULT_SKILL,
            team: seat % 2,
            ally: None,
        })
        .collect()
}

/// Two players a step apart on the outer ring, the first facing the second.
fn face_off(gap: f32) -> LedgeShuffle {
    let mut game = LedgeShuffle::new(&field(2, false), 1);
    game.players[0].run = Runner::at(ARENA_RADIUS - 1.0 - gap - 0.4, 0.0);
    game.players[1].run = Runner::at(ARENA_RADIUS - 1.4, 0.0);
    game.players[0].facing = std::f32::consts::FRAC_PI_2;
    game
}

fn run(game: &mut LedgeShuffle, inputs: &[(u8, Input)], seconds: f32, events: &mut Vec<Value>) {
    for _ in 0..(seconds / TICK_DT).round() as usize {
        game.step(inputs, TICK_DT, events);
    }
}

fn press(seat: u8) -> (u8, Input) {
    (
        seat,
        Input {
            fire: true,
            ..Default::default()
        },
    )
}

fn walk(seat: u8, x: f32, z: f32) -> (u8, Input) {
    (seat, Input::from_stick(Stick { x, z }))
}

#[test]
fn a_shove_winds_up_then_sends_a_rival_over_the_rim() {
    let mut game = face_off(1.6);
    let mut events = Vec::new();
    game.step(&[press(0), walk(1, 0.0, 1.0)], TICK_DT, &mut events);
    assert!(game.players[0].winding(), "the shove winds up first");
    assert!(events.iter().any(|e| e["kind"] == "ledge_windup"));
    let before = game.players[1].run.x;
    run(
        &mut game,
        &[walk(1, 0.0, 1.0)],
        WINDUP_SECONDS - 2.0 * TICK_DT,
        &mut events,
    );
    assert!(
        !events.iter().any(|e| e["kind"] == "ledge_hit"),
        "nothing lands during the wind-up"
    );
    assert!(
        game.players[1].run.x - before < 0.2,
        "and the rival has not been moved"
    );
    run(&mut game, &[], 1.5, &mut events);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "ledge_hit" && e["seat"] == 1 && e["by"] == 0),
        "the shove landed"
    );
    assert!(
        !game.players[1].standing(),
        "and it carried them off the ledge"
    );
    assert!(game.players[0].standing(), "the shover kept their footing");
    assert_eq!(game.players[0].knockoffs, 1, "the knock-off is credited");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "over" && e["seat"] == 1 && e["by"] == 0)
    );
    assert_eq!(
        game.placements(),
        Some(vec![vec![0], vec![1]]),
        "the last one up wins"
    );
}

#[test]
fn a_shove_reaches_only_what_is_in_front() {
    let mut game = face_off(1.6);
    game.players[0].facing = -std::f32::consts::FRAC_PI_2;
    let mut events = Vec::new();
    game.step(&[press(0)], TICK_DT, &mut events);
    run(&mut game, &[], 1.0, &mut events);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "ledge_shove" && e["landed"] == false)
    );
    assert!(game.players[1].standing(), "a shove the other way whiffs");

    let mut game = face_off(SHOVE_REACH + 0.5);
    let mut events = Vec::new();
    game.step(&[press(0)], TICK_DT, &mut events);
    run(&mut game, &[], 1.0, &mut events);
    assert!(
        game.players[1].standing(),
        "and so does one thrown from too far away"
    );
}

#[test]
fn the_stick_aims_a_shove_at_the_moment_it_is_thrown() {
    let mut game = face_off(1.6);
    game.players[0].facing = 0.0;
    let mut events = Vec::new();
    let aimed = (
        0,
        Input {
            stick: Stick { x: 1.0, z: 0.0 },
            fire: true,
            ..Default::default()
        },
    );
    game.step(&[aimed], TICK_DT, &mut events);
    run(&mut game, &[], 1.5, &mut events);
    assert!(
        !game.players[1].standing(),
        "pressing toward them turned the shove onto them"
    );
}

#[test]
fn a_shove_has_a_cooldown_so_it_cannot_be_spammed() {
    let mut game = face_off(1.6);
    game.players[1].run = Runner::at(0.0, 0.0);
    let mut events = Vec::new();
    run(&mut game, &[press(0)], SHOVE_COOLDOWN - 0.05, &mut events);
    let shoves = |events: &Vec<Value>| {
        events
            .iter()
            .filter(|e| e["kind"] == "ledge_windup")
            .count()
    };
    assert_eq!(
        shoves(&events),
        1,
        "holding the button does not chain shoves"
    );
    run(&mut game, &[press(0)], 0.1, &mut events);
    assert_eq!(
        shoves(&events),
        2,
        "and the next one comes once it is ready"
    );
}

#[test]
fn a_braced_player_barely_moves_and_the_shover_bounces_off() {
    let mut game = face_off(1.6);
    let mut events = Vec::new();
    run(&mut game, &[], BRACE_SETTLE + 0.05, &mut events);
    assert!(game.players[1].braced(), "standing still braces you");
    let (start, shover) = (game.players[1].run.x, game.players[0].run.x);
    game.step(&[press(0)], TICK_DT, &mut events);
    run(&mut game, &[], 1.0, &mut events);
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "ledge_block" && e["seat"] == 1 && e["by"] == 0)
    );
    assert!(game.players[1].standing(), "the braced player held the rim");
    assert!(
        (game.players[1].run.x - start).abs() < 1.0,
        "and gave less than a metre, {}",
        game.players[1].run.x - start
    );
    assert!(
        game.players[0].run.x < shover - 0.8,
        "the shover was thrown back, {} from {shover}",
        game.players[0].run.x
    );
}

#[test]
fn walking_into_someone_braced_does_not_push_them() {
    let mut game = face_off(1.2);
    let mut events = Vec::new();
    run(&mut game, &[], BRACE_SETTLE + 0.05, &mut events);
    let start = game.players[1].run.x;
    run(&mut game, &[walk(0, 1.0, 0.0)], 1.5, &mut events);
    assert!(
        (game.players[1].run.x - start).abs() < 0.05,
        "they did not give ground"
    );
    assert!(game.players[1].standing());
}

#[test]
fn a_shove_lands_on_someone_moving_even_without_a_wind_up_to_brace() {
    let mut game = face_off(1.6);
    let mut events = Vec::new();
    run(&mut game, &[], BRACE_SETTLE + 0.05, &mut events);
    game.step(&[press(0), walk(1, 0.0, 1.0)], TICK_DT, &mut events);
    run(&mut game, &[walk(1, 0.0, 0.4)], 1.5, &mut events);
    assert!(!game.players[1].standing(), "moving gives up the brace");
}

#[test]
fn partners_cannot_shove_each_other() {
    let mut contestants = field(2, false);
    contestants[0].ally = Some(7);
    contestants[1].ally = Some(7);
    let mut game = LedgeShuffle::new(&contestants, 1);
    game.players[0].run = Runner::at(ARENA_RADIUS - 3.0, 0.0);
    game.players[1].run = Runner::at(ARENA_RADIUS - 1.4, 0.0);
    game.players[0].facing = std::f32::consts::FRAC_PI_2;
    let mut events = Vec::new();
    game.step(&[press(0)], TICK_DT, &mut events);
    run(&mut game, &[], 1.0, &mut events);
    assert!(game.players[1].standing());
}

#[test]
fn walking_off_the_rim_drops_you() {
    let mut game = face_off(1.6);
    let mut events = Vec::new();
    run(&mut game, &[walk(1, 1.0, 0.0)], 1.0, &mut events);
    assert!(!game.players[1].standing());
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "over" && e["seat"] == 1 && e["by"].is_null()),
        "nobody is credited for a walk off"
    );
}

#[test]
fn the_ledge_crumbles_from_the_rim_inward_with_warning() {
    let mut game = LedgeShuffle::new(&field(4, false), 9);
    for (i, player) in game.players.iter_mut().enumerate().skip(1) {
        player.run = Runner::at(-1.2 + i as f32 * 0.8, 0.6);
    }
    let mut events = Vec::new();
    run(&mut game, &[], RING_SCHEDULE[2].0 - 0.5, &mut events);
    assert!(
        game.sections.iter().all(|s| s.state == Footing::Solid),
        "a quiet start"
    );
    let rim = |game: &LedgeShuffle, ring: usize| {
        game.sections
            .iter()
            .filter(|s| s.ring == ring)
            .map(|s| s.state)
            .collect::<Vec<_>>()
    };
    run(&mut game, &[], 1.0, &mut events);
    let first = game
        .sections
        .iter()
        .position(|s| s.state == Footing::Cracking)
        .expect("a rim section cracked");
    assert_eq!(game.sections[first].ring, 2, "the rim goes first");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "ledge_crack" && e["section"] == first)
    );
    assert!(rim(&game, 0).iter().all(|s| *s == Footing::Solid));

    // Someone standing on it is still held until the crack runs out.
    let ring = game.rings[2];
    let slot = first - ring.first;
    let angle = ring.turn + TAU * (slot as f32 + 0.5) / ring.count as f32;
    let out = (ring.inner + ring.outer) / 2.0;
    game.players[0].run = Runner::at(angle.cos() * out, angle.sin() * out);
    run(&mut game, &[], CRACK_SECONDS - 1.0, &mut events);
    assert!(game.players[0].standing(), "a cracking section still holds");
    run(&mut game, &[], 1.0, &mut events);
    assert!(
        !game.players[0].standing(),
        "and drops whoever stayed on it"
    );
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "ledge_fall" && e["section"] == first)
    );

    let left = RING_SCHEDULE[2].1 - game.time + CRACK_SECONDS + FALL_SECONDS + 0.1;
    run(&mut game, &[], left, &mut events);
    assert!(
        rim(&game, 2).iter().all(|s| *s == Footing::Gone),
        "the whole rim is gone"
    );
    assert!(
        rim(&game, 1).iter().any(|s| *s == Footing::Solid),
        "the middle ring is not yet"
    );
    assert!(!game.holds(ARENA_RADIUS - 1.0, 0.0));

    let left = RING_SCHEDULE[0].1 - game.time + CRACK_SECONDS + FALL_SECONDS + 0.1;
    run(&mut game, &[], left, &mut events);
    assert!(
        game.sections.iter().all(|s| s.state == Footing::Gone),
        "only the core is left"
    );
    assert!(game.holds(0.0, 0.0) && game.holds(CORE_RADIUS - 0.1, 0.0));
    assert!(!game.holds(CORE_RADIUS + 0.2, 0.0));
}

#[test]
fn every_section_has_its_own_time_and_the_core_only_wears_at_the_end() {
    let game = LedgeShuffle::new(&field(4, false), 3);
    assert_eq!(
        game.sections.len(),
        RINGS.iter().map(|(_, n)| n).sum::<usize>()
    );
    for (k, (from, until)) in RING_SCHEDULE.iter().enumerate() {
        let mut times: Vec<f32> = game
            .sections
            .iter()
            .filter(|s| s.ring == k)
            .map(|s| s.crack_at)
            .collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((times[0] - from).abs() < 1e-4 && (times[times.len() - 1] - until).abs() < 1e-4);
        assert!(
            times.windows(2).all(|w| w[1] - w[0] > 1.0),
            "one at a time, ring {k}"
        );
    }
    assert!(
        RING_SCHEDULE[0].1 + CRACK_SECONDS + FALL_SECONDS < CORE_SHRINK.0,
        "there is a showdown on the whole core first"
    );
    assert!(
        CORE_SHRINK.1 < ROUND_SECONDS,
        "and it has worn to room for one before the bell"
    );
    assert!((game.core_at(CORE_SHRINK.0) - CORE_RADIUS).abs() < 1e-4);
    assert!((game.core_at(ROUND_SECONDS) - CORE_MIN_RADIUS).abs() < 1e-4);
    // Every point on the ledge belongs to exactly the section the lookup says.
    for ring in &game.rings {
        for slot in 0..ring.count {
            let angle = ring.turn + TAU * (slot as f32 + 0.5) / ring.count as f32;
            let out = (ring.inner + ring.outer) / 2.0;
            assert_eq!(
                game.section_at(angle.cos() * out, angle.sin() * out),
                Some(Some(ring.first + slot))
            );
        }
    }
    assert_eq!(game.section_at(0.5, 0.5), Some(None));
    assert_eq!(game.section_at(ARENA_RADIUS + 0.1, 0.0), None);
}

#[test]
fn a_bigger_field_gets_a_wider_ledge_with_more_sections() {
    let four = LedgeShuffle::new(&field(4, false), 1);
    let eight = LedgeShuffle::new(&field(8, false), 1);
    assert_eq!(four.radius(), ARENA_RADIUS);
    assert!((eight.radius() / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4);
    assert!(eight.sections.len() > four.sections.len());
    assert_eq!(
        eight.setup()["radius"],
        json!(eight.radius()),
        "and the client is told"
    );
    assert_eq!(eight.setup()["rings"].as_array().map(Vec::len), Some(3));
    for player in &eight.players {
        assert!(
            eight.holds(player.run.x, player.run.z),
            "everyone starts on the ledge"
        );
    }
}

#[test]
fn the_worn_core_leaves_room_for_one() {
    let mut game = LedgeShuffle::new(&field(3, false), 4);
    for (i, player) in game.players.iter_mut().enumerate() {
        player.run = Runner::at(-1.2 + i as f32 * 1.2, 0.0);
    }
    let mut events = Vec::new();
    run(&mut game, &[], CORE_SHRINK.0, &mut events);
    assert!(
        game.players.iter().all(Shover::standing),
        "all three held out on the core"
    );
    run(
        &mut game,
        &[],
        ROUND_SECONDS - CORE_SHRINK.0 + 0.5,
        &mut events,
    );
    let placements = game.placements().expect("the round ended");
    assert_eq!(
        placements[0],
        vec![1],
        "the one in the middle outlasted the wear"
    );
    assert_eq!(placements.iter().flatten().count(), 3);
}

#[test]
fn survivors_at_the_bell_share_first() {
    let mut game = LedgeShuffle::new(&field(2, false), 4);
    game.players[0].run = Runner::at(-0.2, 0.0);
    game.players[1].run = Runner::at(0.2, 0.0);
    let mut events = Vec::new();
    run(&mut game, &[], ROUND_SECONDS + 0.5, &mut events);
    let placements = game.placements().expect("the clock ended it");
    assert_eq!(
        placements,
        vec![vec![0, 1]],
        "both squeezed onto the last stone, so both win"
    );
}

fn bot_round(count: u8, seed: u64) -> (LedgeShuffle, Vec<Value>, f32) {
    let mut game = LedgeShuffle::new(&field(count, true), seed);
    let mut events = Vec::new();
    let mut steps = 0usize;
    while game.placements().is_none() && steps < ((ROUND_SECONDS + 1.0) / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..count)
            .map(|s| (s, game.bot_input(s, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    (game, events, steps as f32 * TICK_DT)
}

#[test]
fn bots_fight_it_out_and_leave_one_standing() {
    let (mut shoves, mut hits, mut knockoffs, mut decided) = (0, 0, 0, 0);
    for seed in 0..6 {
        let (game, events, ended) = bot_round(4, 40 + seed);
        let placements = game.placements().expect("the round finished");
        assert_eq!(placements.iter().flatten().count(), 4, "everyone is ranked");
        assert!(
            ended > 15.0,
            "seed {seed}: it was not over before it began, {ended}"
        );
        decided += usize::from(placements[0].len() == 1);
        shoves += events.iter().filter(|e| e["kind"] == "ledge_shove").count();
        hits += events.iter().filter(|e| e["kind"] == "ledge_hit").count();
        knockoffs += events
            .iter()
            .filter(|e| e["kind"] == "over" && !e["by"].is_null())
            .count();
        // Bots do not walk off a ledge that is not yet crumbling.
        let early = events
            .iter()
            .position(|e| e["kind"] == "ledge_crack")
            .unwrap_or(events.len());
        assert!(
            !events[..early]
                .iter()
                .any(|e| e["kind"] == "over" && e["by"].is_null()),
            "seed {seed}: a bot walked off on its own"
        );
    }
    assert!(
        decided >= 5,
        "nearly every round ends with one winner, {decided} of 6"
    );
    assert!(shoves > 40, "they throw shoves, {shoves}");
    assert!(hits > 15, "and land them, {hits}");
    assert!(
        knockoffs >= 6,
        "and shoves are what knock people off, {knockoffs}"
    );
}

#[test]
fn bots_get_off_cracking_ground() {
    let (mut on_cracks, mut cracked_under) = (0, 0);
    for seed in 0..6 {
        let (_, events, _) = bot_round(4, 70 + seed);
        for e in events.iter().filter(|e| e["kind"] == "over") {
            on_cracks += 1;
            cracked_under += usize::from(e["by"].is_null());
        }
    }
    assert!(
        cracked_under * 3 <= on_cracks,
        "most falls are shoves, not bots standing on cracks: {cracked_under} of {on_cracks}"
    );
}

#[test]
fn eight_bots_play_a_full_round() {
    let (game, events, ended) = bot_round(8, 5);
    let placements = game.placements().expect("the round finished");
    assert_eq!(placements.iter().flatten().count(), 8);
    assert!(ended <= ROUND_SECONDS + 0.1);
    assert!(events.iter().filter(|e| e["kind"] == "ledge_hit").count() > 5);
}
