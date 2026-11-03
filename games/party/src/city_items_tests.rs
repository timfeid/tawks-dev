use super::*;
const DT: f32 = 1. / 60.;
fn game() -> CityGame {
    let mut g = CityGame::new(
        &[(0, false), (1, false), (2, false)],
        CityConfig::default(),
        false,
    );
    g.briefing_left = 0.;
    g.time = 3.;
    g
}
fn put(g: &mut CityGame, seat: u8, at: (f32, f32)) {
    let a = g.actors.iter_mut().find(|a| a.seat == seat).unwrap();
    a.at = at;
    a.in_car = false;
    a.stun = 0.;
    a.immune = 0.;
}
fn give(g: &mut CityGame, seat: u8, item: ItemKind) {
    g.actors.iter_mut().find(|a| a.seat == seat).unwrap().item = Some(item);
}
fn tick(g: &mut CityGame, seconds: f32, input: &[(u8, Stick, f32)]) {
    for _ in 0..(seconds * 60.) as usize {
        g.tick(DT, input, &[0, 1, 2], |_, _| false);
    }
}
fn me(g: &CityGame, seat: u8) -> &Actor {
    g.actors.iter().find(|a| a.seat == seat).unwrap()
}

#[test]
fn pads_sit_away_from_every_heist_building() {
    let spots = pad_spots();
    assert!(spots.len() >= 12, "{}", spots.len());
    for p in spots {
        for l in [BANK, POWER, ESCAPE, JAIL] {
            assert!(distance(p, l) > 10.);
        }
        assert!(walkable(p));
    }
}

#[test]
fn a_robber_walks_over_a_pad_and_takes_its_item() {
    let mut g = game();
    tick(&mut g, 2., &[]);
    let pad = g
        .items
        .pads
        .iter()
        .find(|p| p.item.is_some())
        .map(|p| p.at)
        .expect("a stocked pad");
    put(&mut g, 1, pad);
    tick(&mut g, 0.1, &[]);
    assert!(me(&g, 1).item.is_some());
    put(&mut g, 0, pad);
    assert!(me(&g, 0).item.is_none(), "police get their tools elsewhere");
}

#[test]
fn caltrops_flatten_a_patrol_car_which_crawls_until_repaired() {
    let mut g = game();
    give(&mut g, 1, ItemKind::Caltrops);
    put(&mut g, 1, (17.05, 0.));
    assert!(g.use_item(1, ItemAct::Place, 0.));
    put(&mut g, 1, (60., 60.));
    g.place(0, (17.05, -14.), true, 0.);
    let go = [(0, Stick { x: 0., z: 1. }, 0.)];
    tick(&mut g, 2.5, &go);
    assert!(me(&g, 0).tire > 0., "the tyre is flat");
    assert!(g.items.hazards.is_empty(), "the strip is used up");
    tick(&mut g, 2., &go);
    assert!(
        me(&g, 0).drive_speed <= FLAT_TOP_SPEED + 0.01,
        "{}",
        me(&g, 0).drive_speed
    );
    assert_eq!(g.view_for(Some(0)).step, Some(Step::Repair));
    g.toggle_car(0);
    let c = g.view_for(Some(0)).context.unwrap();
    assert_eq!((c.verb, c.button), (Verb::Repair, "a"));
    g.control(0, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(g.qa_check_kind(0), Some(CheckKind::LugNuts));
    g.qa_pass_check(0);
    assert_eq!(me(&g, 0).tire, 0., "fixed by hand");
}

#[test]
fn a_flat_fixes_itself_in_time() {
    let mut g = game();
    g.place(0, (17.05, -14.), true, 0.);
    g.actors[0].tire = 1.;
    tick(&mut g, 1.2, &[]);
    assert_eq!(me(&g, 0).tire, 0.);
}

#[test]
fn a_flat_patrol_bot_stops_to_tighten_the_wheel_nuts() {
    let mut g = game();
    g.automate(0, true);
    g.place(0, (17.05, -14.), true, 0.);
    put(&mut g, 1, (60., 60.));
    put(&mut g, 2, (60., 58.));
    g.actors[0].tire = 60.;

    let mut started = false;
    for _ in 0..60 * 25 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        started |= g.qa_check_kind(0) == Some(CheckKind::LugNuts);
        if started && me(&g, 0).tire <= 0. {
            break;
        }
    }
    assert!(started, "the officer uses the wheel-nut check");
    assert_eq!(me(&g, 0).tire, 0., "the car is repaired");
}

#[test]
fn a_patrol_bot_gets_out_to_check_a_seen_loot_bag() {
    let mut g = game();
    g.automate(0, true);
    g.place(0, (17.05, -14.), true, 0.);
    put(&mut g, 1, (60., 60.));
    put(&mut g, 2, (60., 58.));
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (17.05, -12.),
        carrier: None,
    }];

    let mut started = false;
    for _ in 0..60 * 20 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        started |= g.qa_check_kind(0) == Some(CheckKind::Steady);
        if g.night_score[0] > 0 {
            break;
        }
    }
    assert!(started, "the officer uses the tag check");
    assert_eq!(g.night_score[0], 1, "police recover the bag");
}

#[test]
fn a_patrol_bot_tags_nearby_loot_before_chasing_a_robber() {
    let mut g = game();
    g.automate(0, true);
    g.place(0, (17.05, -14.), false, 0.);
    g.place(1, (18., -12.), false, 0.);
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (17.05, -12.),
        carrier: None,
    }];
    assert_eq!(g.ai_goal(0), ((17.05, -12.), true));
}

#[test]
fn a_banana_makes_the_carrier_drop_the_bag() {
    let mut g = game();
    give(&mut g, 2, ItemKind::Banana);
    put(&mut g, 2, (0., -2.));
    assert!(g.use_item(2, ItemAct::Place, 0.));
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (0., 0.),
        carrier: Some(1),
    }];
    put(&mut g, 1, (0., -6.));
    tick(&mut g, 1.5, &[(1, Stick { x: 0., z: 1. }, 0.)]);
    assert!(!g.carrying(1), "the bag went flying");
    assert!(!g.loose_bags().is_empty());
    assert!(me(&g, 1).stun > 0. || me(&g, 1).height > 0. || g.time > 0.);
}

#[test]
fn a_decoy_never_ends_a_job_and_pops_on_the_officer() {
    let mut g = game();
    give(&mut g, 1, ItemKind::Decoy);
    put(&mut g, 1, (0., -6.));
    assert!(g.use_item(1, ItemAct::Place, 0.));
    put(&mut g, 1, (60., 60.));
    put(&mut g, 0, (0., -7.));
    let police = g.view_for(Some(0));
    assert_eq!(police.bags.len(), 1);
    assert!(!police.bags[0].decoy, "police cannot tell");
    assert!(police.loot.is_none(), "and get no separate real-bag field");
    assert!(g.view_for(Some(2)).bags[0].decoy, "the crew can");
    g.control(0, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(
        g.qa_check_kind(0),
        Some(CheckKind::Steady),
        "the tag check, same as a real bag"
    );
    g.qa_pass_check(0);
    assert!(g.items.decoys.is_empty());
    assert!(me(&g, 0).stun > 0., "a dud in the face");
    assert_eq!(g.night_score, [0, 0], "no job ended");
}

#[test]
fn a_takedown_knocks_a_robber_down_and_never_arrests() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 2.5));
    assert!(!g.takedown(0, 0.), "a dive requires a callout and Clarity");
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
    assert!(g.takedown(0, 0.));
    g.clarity(0, false);
    tick(&mut g, 0.5, &[]);
    assert!(me(&g, 1).stun > 0., "down");
    assert!(g.arrests.is_empty(), "a takedown never arrests");
    assert!(me(&g, 0).stun <= 0., "no flop on a hit");
    assert!(!g.takedown(0, 0.), "on cooldown");
}

fn landed_pin() -> CityGame {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 2.5));
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
    assert!(g.takedown(0, 0.));
    g.clarity(0, false);
    tick(&mut g, 0.5, &[]);
    assert!(me(&g, 1).stun > 0.);
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.arrests.first().map(|a| a.stage), Some(Stage::Ground), "cop {:?} interact {} grab {:?}, crew {:?} height {} stun {}", me(&g, 0).at, me(&g, 0).interact, me(&g, 0).grab, me(&g, 1).at, me(&g, 1).height, me(&g, 1).stun);
    g
}

#[test]
fn crew_gets_one_twenty_percent_ground_escape_check_per_round() {
    let mut g = landed_pin();
    assert!(me(&g, 1).ground_escape_used);
    g.arrests[0].clock = arrest::GROUND_ESCAPE_SECONDS * 0.5;
    assert!(g.struggle(1), "a press inside the narrow band escapes");
    assert!(g.arrests.is_empty());
    assert_eq!(me(&g, 1).stun, 0., "escaped crew can run immediately");
    g.start_arrest(0, 1);
    assert_eq!(g.arrests[0].stage, Stage::Cuffing, "no second ground check this round");

    let mut g = landed_pin();
    g.arrests[0].clock = 0.1;
    assert!(!g.struggle(1), "a press outside the band misses");
    assert_eq!(g.arrests[0].stage, Stage::Cuffing);

    let mut g = landed_pin();
    tick(&mut g, arrest::GROUND_ESCAPE_SECONDS + 0.1, &[]);
    assert_eq!(g.arrests[0].stage, Stage::Cuffing, "waiting out the check starts cuffing");
}

#[test]
fn pinned_crew_can_spend_smoke_to_break_free() {
    let mut g = landed_pin();
    g.actors[1].tools[0] = 1;
    assert!(g.use_tool(1, Tool::Smoke));
    assert!(g.arrests.is_empty());
    assert_eq!(g.actors[1].tools[0], 0);
    assert_eq!(g.smokes.len(), 1);
    assert_eq!(me(&g, 1).stun, 0.);
}

#[test]
fn a_missed_takedown_flops_and_a_dodge_makes_it_miss() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 2.5));
    assert!(g.dodge(1, std::f32::consts::FRAC_PI_2));
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
    assert!(g.takedown(0, 0.));
    g.clarity(0, false);
    tick(&mut g, 0.5, &[]);
    assert!(me(&g, 1).stun <= 0., "rolled clear");
    assert!(me(&g, 0).stun > 0., "belly flop");
    assert!(!g.dodge(1, 0.), "the roll is on cooldown");
}

#[test]
fn clarity_requires_a_callout_and_release_starts_recharge() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 3.));
    assert!(!g.clarity(0, true));
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
    tick(&mut g, 0.2, &[]);
    assert!(g.time_scale() < 1.);
    assert!(g.clarity(0, false));
    assert_eq!(g.time_scale(), 1.);
    assert!(!g.clarity(0, true), "Clarity cannot restart during recharge");
    tick(&mut g, 5.1, &[]);
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
}

#[test]
fn hands_up_beam_follows_until_the_suspect_leaves_range() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 3.));
    assert!(g.callout(0, 0.));
    tick(&mut g, 5.1, &[]);
    assert_eq!(g.view_for(Some(0)).callout, Some([0, 1]));
    put(&mut g, 1, (0., 17.));
    tick(&mut g, 0.1, &[]);
    assert_eq!(g.view_for(Some(0)).callout, None);
}

#[test]
fn both_teams_strafe_faster_during_clarity() {
    let run = |slow: bool| {
        let mut g = game();
        put(&mut g, 0, (0., 0.));
        put(&mut g, 1, (0., 3.));
        if slow {
            assert!(g.callout(0, 0.));
            assert!(g.clarity(0, true));
        }
        tick(&mut g, 0.25, &[
            (0, Stick { x: 1., z: 0. }, 0.),
            (1, Stick { x: 1., z: 0. }, 0.),
        ]);
        (me(&g, 0).at.0, me(&g, 1).at.0, me(&g, 1).yaw, g.time_scale())
    };
    let normal = run(false);
    let clarity = run(true);
    assert!(clarity.0 > normal.0, "cop strafe: normal {normal:?}, clarity {clarity:?}");
    assert!(clarity.1 > normal.1, "crew strafe: normal {normal:?}, clarity {clarity:?}");
}

#[test]
fn a_nightstick_hit_keeps_a_suspect_down_longer_and_spends_the_item() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (0., 2.5));
    give(&mut g, 0, ItemKind::Nightstick);
    assert!(g.callout(0, 0.));
    assert!(g.clarity(0, true));
    assert!(g.takedown(0, 0.));
    g.clarity(0, false);
    tick(&mut g, 0.5, &[]);
    assert!(me(&g, 1).stun > 1.8, "the nightstick gives time to cuff");
    assert_eq!(me(&g, 0).item, None, "a hit spends the nightstick");
}

#[test]
fn a_patrol_car_brakes_then_reverses_without_turning_around() {
    let mut g = game();
    g.place(0, (16., -10.), true, 0.);
    tick(&mut g, 0.4, &[(0, Stick { x: 0., z: 1. }, 0.)]);
    assert!(me(&g, 0).drive_speed > 0.);
    tick(&mut g, 0.8, &[(0, Stick { x: 0., z: -1. }, 0.)]);
    assert!(me(&g, 0).drive_speed < 0.);
    assert!(me(&g, 0).car_yaw.abs() < 0.1);
}

#[test]
fn steering_right_moves_toward_screen_right_in_forward_and_reverse() {
    let mut g = game();
    g.place(0, (16., -10.), true, 0.);
    tick(&mut g, 0.3, &[(0, Stick { x: 0., z: 1. }, 0.)]);
    tick(&mut g, 0.3, &[(0, Stick { x: 1., z: 1. }, 0.)]);
    assert!(me(&g, 0).car_yaw < 0., "right input turns right while driving forward");
    assert!(me(&g, 0).at.0 < 16., "the car moves to the right of its starting lane");
    g.place(0, (16., -10.), true, 0.);
    tick(&mut g, 0.5, &[(0, Stick { x: 0., z: -1. }, 0.)]);
    tick(&mut g, 0.3, &[(0, Stick { x: 1., z: -1. }, 0.)]);
    assert!(me(&g, 0).car_yaw > 0., "right input moves right while reversing");
    assert!(me(&g, 0).at.0 < 16., "reversing with right input also moves right");
}

#[test]
fn stuns_that_stack_are_halved() {
    let mut g = game();
    g.stun_for(1, 2.);
    tick(&mut g, 2.1, &[]);
    g.stun_for(1, 2.);
    assert!((me(&g, 1).stun - 1.).abs() < 0.01, "{}", me(&g, 1).stun);
    tick(&mut g, 6., &[]);
    g.stun_for(1, 2.);
    assert!((me(&g, 1).stun - 2.).abs() < 0.01, "a fresh stun is whole");
}

#[test]
fn tools_are_locked_to_their_side() {
    let mut g = game();
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (5., 0.));
    assert!(!g.use_tool(0, Tool::Smoke), "police cannot smoke");
    assert!(!g.use_tool(1, Tool::Scanner), "the crew cannot ping");
    assert!(g.use_tool(1, Tool::Smoke));
    assert!(g.use_tool(0, Tool::Scanner));
}

#[test]
fn enemy_hazards_hide_until_close() {
    let mut g = game();
    give(&mut g, 1, ItemKind::Banana);
    put(&mut g, 1, (0., -10.));
    assert!(g.use_item(1, ItemAct::Place, 0.));
    put(&mut g, 0, (0., 10.));
    assert!(g.view_for(Some(0)).hazards.is_empty(), "too far to see");
    assert_eq!(
        g.view_for(Some(2)).hazards.len(),
        1,
        "the crew always sees its own"
    );
    put(&mut g, 0, (0., -4.));
    assert_eq!(g.view_for(Some(0)).hazards.len(), 1, "close up it shows");
}

#[test]
fn bot_routes_go_around_a_hazard_they_can_see() {
    let from = (17.05, -30.);
    let to = (17.05, 30.);
    let strip = ((17.05, 0.), CALTROPS_HALF + 1.6);
    let straight = route(from, to, true);
    assert!(
        straight.iter().any(|p| distance(*p, strip.0) < strip.1),
        "the plain route crosses it"
    );
    let around = route_avoiding(from, to, true, &[strip]);
    assert!(!around.is_empty(), "there is another way");
    let mut here = from;
    for p in &around {
        assert!(
            segment_distance(strip.0, here, *p) >= strip.1 - 1.01,
            "{here:?} to {p:?}"
        );
        here = *p;
    }
}

#[test]
fn bots_only_avoid_hazards_they_see_and_that_hurt_them() {
    let mut g = CityGame::new(
        &[(0, true), (1, true), (2, false)],
        CityConfig::default(),
        false,
    );
    g.briefing_left = 0.;
    g.time = 3.;
    give(&mut g, 2, ItemKind::Caltrops);
    put(&mut g, 2, (17.05, 0.));
    assert!(g.use_item(2, ItemAct::Place, 0.));
    give(&mut g, 2, ItemKind::Banana);
    put(&mut g, 2, (0., -3.));
    assert!(g.use_item(2, ItemAct::Place, 0.));
    put(&mut g, 0, (17.05, -30.));
    assert!(g.hazards_to_avoid(me(&g, 0)).is_empty(), "too far to see");
    put(&mut g, 0, (17.05, -5.));
    assert_eq!(
        g.hazards_to_avoid(me(&g, 0)).len(),
        1,
        "the strip, now in sight"
    );
    put(&mut g, 1, (1., -5.));
    let crew = g.hazards_to_avoid(me(&g, 1));
    assert_eq!(
        crew.len(),
        1,
        "a teammate's peel, never the caltrops: {crew:?}"
    );
}

#[test]
fn a_robber_carrying_a_decoy_passes_for_the_carrier_until_cuffed() {
    let mut g = game();
    give(&mut g, 1, ItemKind::Decoy);
    put(&mut g, 1, (16., 6.));
    assert!(g.use_item(1, ItemAct::Place, 0.));
    for _ in 0..10 {
        g.control(1, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert!(me(&g, 1).fake_bag > 0., "picked the decoy back up");
    assert!(g.items.decoys.is_empty());
    put(&mut g, 0, (16., 2.));
    let police = g.view_for(Some(0));
    let robber = police.actors.iter().find(|a| a.seat == 1).unwrap();
    assert!(robber.carrying, "police see a carrier");
    assert_eq!(police.step, Some(Step::ChaseBag));
    let crew = g.view_for(Some(2));
    assert!(
        !crew.actors.iter().find(|a| a.seat == 1).unwrap().carrying,
        "the crew know better"
    );
    put(&mut g, 0, (16., 4.));
    put(&mut g, 1, (16.8, 4.));
    g.actors[1].stun = 2.;
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    for _ in 0..(2.7 * 60.) as usize {
        g.control(0, true, false);
        if let Some(t) = g.arrests.first().map(|x| x.t) {
            let p = arrest::figure_eight(t + std::f32::consts::TAU / 2.5 / 60.);
            g.trace(0, p.0, p.1);
        }
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(me(&g, 1).fake_bag, 0., "the cuffs pop the decoy");
    assert_eq!(g.night_score, [0, 0], "no bag was seized");
}

#[test]
fn a_flat_car_parked_in_the_bay_is_fixed() {
    let mut g = game();
    assert!(
        drivable(BAY) && !on_road(BAY),
        "the bay sits off the street, reachable on snow"
    );
    g.place(0, BAY, true, 0.);
    g.actors[0].tire = FLAT_SECONDS;
    let v = g.view_for(Some(0));
    assert_eq!(v.context.unwrap().verb, Verb::Bay);
    for _ in 0..(BAY_SECONDS * 60.) as usize + 5 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(me(&g, 0).tire, 0.);
}
