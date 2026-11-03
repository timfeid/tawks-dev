use super::*;
const DT: f32 = 1. / 60.;

fn game_with(crew: usize) -> CityGame {
    let seats: Vec<(u8, bool)> = (0..=crew as u8).map(|s| (s, false)).collect();
    let mut g = CityGame::new(&seats, CityConfig::default(), false);
    g.briefing_left = 0.;
    g.time = 3.;
    g
}
fn game() -> CityGame {
    game_with(2)
}
fn put(g: &mut CityGame, seat: u8, at: (f32, f32)) {
    let a = g.actors.iter_mut().find(|a| a.seat == seat).unwrap();
    a.at = at;
    a.in_car = false;
    a.stun = 0.;
    a.immune = 0.;
}
fn me(g: &CityGame, seat: u8) -> &Actor {
    g.actors.iter().find(|a| a.seat == seat).unwrap()
}
fn tick(g: &mut CityGame, seconds: f32) {
    for _ in 0..(seconds * 60.).round() as usize {
        g.tick(DT, &[], &[0, 1, 2, 3, 4, 5, 6], |_, _| false);
    }
}
/// Press A at the current spot: one tick held, then released.
fn press_a(g: &mut CityGame, seat: u8) {
    g.control(seat, true, false);
    tick(g, DT);
    g.control(seat, false, false);
    tick(g, DT);
}
fn check(g: &CityGame, seat: u8) -> &checks::Check {
    me(g, seat).check.as_ref().expect("a running check")
}
fn view(g: &CityGame, seat: u8) -> CheckView {
    g.view_for(Some(seat)).check.expect("a check in the view")
}

#[test]
fn wires_pass_on_the_right_flicks_and_spark_on_a_wrong_one() {
    let mut g = game();
    put(&mut g, 1, POWER);
    press_a(&mut g, 1);
    assert_eq!(view(&g, 1).kind, Some(CheckKind::Wires));
    for _ in 0..4 {
        let v = view(&g, 1);
        let wire = v.wire.unwrap();
        let dir = v
            .terminals
            .unwrap()
            .iter()
            .position(|&t| t == wire)
            .unwrap() as u8;
        assert!(g.check_input(1, CheckInput::Flick { dir }));
        tick(&mut g, 0.2);
    }
    assert!(me(&g, 1).check.is_none());
    assert!(g.power > 0., "the power is cut");
    assert_eq!(g.job, Job::Vault);

    let mut g = game();
    put(&mut g, 1, POWER);
    press_a(&mut g, 1);
    let v = view(&g, 1);
    let right = v
        .terminals
        .unwrap()
        .iter()
        .position(|&t| t == v.wire.unwrap())
        .unwrap() as u8;
    g.check_input(
        1,
        CheckInput::Flick {
            dir: (right + 1) % 4,
        },
    );
    assert!(me(&g, 1).check.is_none());
    assert!(me(&g, 1).stun > 0., "a spark");
    assert!(!g.noises.is_empty(), "police hear it");
    assert_eq!(g.power, 0.);
}

#[test]
fn wires_run_out_of_time() {
    let mut g = game();
    put(&mut g, 1, POWER);
    press_a(&mut g, 1);
    tick(&mut g, 1.6);
    assert!(me(&g, 1).check.is_none(), "too slow on the first wire");
    assert_eq!(g.check_fails[CheckKind::Wires as usize], 1);
}

#[test]
fn the_dial_sets_numbers_by_feel_and_fumbles_make_noise() {
    let mut g = game();
    g.job = Job::Vault;
    g.power = 30.;
    put(&mut g, 1, BANK);
    press_a(&mut g, 1);
    assert!(g.alarm, "touching the vault sounds the alarm");
    let combo = check(&g, 1).seq.clone();
    // A wrong number resets and makes noise.
    g.check_input(
        1,
        CheckInput::Dial {
            at: f32::from(combo[0]) + 6.,
        },
    );
    g.check_input(1, CheckInput::Press);
    assert_eq!(check(&g, 1).step, 0);
    assert!(!g.noises.is_empty());
    for (i, n) in combo.iter().enumerate() {
        // Sweep toward the number and read the tremor, as a player would.
        let mut best = (0., 0.);
        for k in 0..80 {
            let at = f32::from(*n) - 4. + k as f32 * 0.1;
            g.check_input(1, CheckInput::Dial { at });
            let tremor = view(&g, 1).tremor.unwrap();
            if tremor > best.1 {
                best = (at, tremor);
            }
            if at > f32::from(*n) + 1. {
                break;
            }
        }
        g.check_input(1, CheckInput::Dial { at: best.0 });
        g.check_input(1, CheckInput::Press);
        if i + 1 < combo.len() {
            assert_eq!(check(&g, 1).step as usize, i + 1);
        }
    }
    assert!(me(&g, 1).check.is_none());
    assert_eq!(g.job, Job::Carry, "the vault opens");
    assert!(!g.bags.is_empty());
}

#[test]
fn the_dial_stops_when_the_power_comes_back() {
    let mut g = game();
    g.job = Job::Vault;
    g.power = 0.1;
    put(&mut g, 1, BANK);
    press_a(&mut g, 1);
    assert!(me(&g, 1).check.is_some());
    tick(&mut g, 0.3);
    assert!(me(&g, 1).check.is_none());
}

#[test]
fn stuffing_a_bag_repeats_the_shown_buttons() {
    let mut g = game();
    g.job = Job::Carry;
    g.spill_bags();
    let bag = g.bags[0].at;
    put(&mut g, 1, bag);
    press_a(&mut g, 1);
    let seq = check(&g, 1).seq.clone();
    assert!(view(&g, 1).showing.unwrap());
    assert!(view(&g, 1).shown.is_some(), "the first button is on show");
    // Pressing while it is still being shown fails.
    g.check_input(1, CheckInput::Button { b: seq[0] });
    assert!(me(&g, 1).check.is_none());
    tick(&mut g, 1.1);
    press_a(&mut g, 1);
    let seq = check(&g, 1).seq.clone();
    while view(&g, 1).showing == Some(true) {
        tick(&mut g, DT);
    }
    for b in &seq {
        g.check_input(1, CheckInput::Button { b: *b });
    }
    assert!(g.carrying(1), "the bag is stuffed and carried");
}

#[test]
fn the_van_takes_steady_trigger_pressure_and_spills_on_too_much() {
    let mut g = game();
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: ESCAPE,
        carrier: Some(1),
    }];
    put(&mut g, 1, ESCAPE);
    press_a(&mut g, 1);
    assert_eq!(view(&g, 1).kind, Some(CheckKind::Pressure));
    // Slamming the trigger spills.
    for _ in 0..40 {
        g.check_input(1, CheckInput::Trigger { value: 1. });
        tick(&mut g, DT);
    }
    assert!(check(&g, 1).fails >= 1);
    assert!(!g.noises.is_empty());
    // Following the band delivers.
    for _ in 0..60 * 12 {
        let target = view(&g, 1).target.unwrap();
        g.check_input(1, CheckInput::Trigger { value: target });
        tick(&mut g, DT);
        if me(&g, 1).check.is_none() {
            break;
        }
    }
    assert_eq!(g.view_for(Some(1)).night_score[1], 1, "bag delivered");
}

#[test]
fn the_lockpick_turns_only_at_its_sweet_spot_and_breaks_when_forced() {
    let mut g = game();
    g.actors[2].jailed = true;
    g.actors[2].jail_left = 60.;
    put(&mut g, 1, JAIL);
    press_a(&mut g, 1);
    let sweet = check(&g, 1).target;
    for _ in 0..60 * 3 {
        g.check_input(
            1,
            CheckInput::Pick {
                angle: sweet + 0.6,
                turn: 1.,
            },
        );
        tick(&mut g, DT);
    }
    assert!(
        me(&g, 1).check.as_ref().is_none_or(|c| c.fails >= 1),
        "forcing breaks picks"
    );
    tick(&mut g, 2.);
    press_a(&mut g, 1);
    let sweet = check(&g, 1).target;
    for _ in 0..60 {
        g.check_input(
            1,
            CheckInput::Pick {
                angle: sweet,
                turn: 1.,
            },
        );
        tick(&mut g, DT);
    }
    assert!(!g.actors[2].jailed, "the cell opens");
}

#[test]
fn the_security_desk_is_a_second_post_for_big_crews() {
    let mut g = game_with(4);
    assert!(g.tuning.second_post);
    put(&mut g, 1, POWER);
    press_a(&mut g, 1);
    for _ in 0..4 {
        let v = view(&g, 1);
        let dir = v
            .terminals
            .unwrap()
            .iter()
            .position(|&t| t == v.wire.unwrap())
            .unwrap() as u8;
        g.check_input(1, CheckInput::Flick { dir });
    }
    assert_eq!(g.job, Job::Power, "power alone is not enough");
    put(&mut g, 2, CAMERAS);
    press_a(&mut g, 2);
    assert_eq!(view(&g, 2).kind, Some(CheckKind::Cameras));
    let mut hits = 0;
    for _ in 0..60 * 14 {
        let v = view(&g, 2);
        let period = v.period.unwrap();
        let cursor = ((v.elapsed / period).rem_euclid(1.) * 2. - 1.).abs();
        if (cursor - v.target.unwrap()).abs() < v.width.unwrap() * 0.3 {
            g.check_input(2, CheckInput::Press);
            hits += 1;
        }
        tick(&mut g, DT);
        if me(&g, 2).check.is_none() {
            break;
        }
    }
    assert!(hits >= 3);
    assert_eq!(
        g.job,
        Job::Vault,
        "power and the desk together open the vault job"
    );
}

#[test]
fn police_recover_a_bag_by_holding_aim_on_the_tag() {
    let mut g = game();
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (0., 0.),
        carrier: None,
    }];
    put(&mut g, 0, (0., 1.));
    press_a(&mut g, 0);
    assert_eq!(view(&g, 0).kind, Some(CheckKind::Steady));
    for _ in 0..60 * 8 {
        let aim = view(&g, 0).aim.unwrap();
        g.check_input(
            0,
            CheckInput::Aim {
                x: aim[0],
                y: aim[1],
            },
        );
        tick(&mut g, DT);
        if me(&g, 0).check.is_none() {
            break;
        }
    }
    assert_eq!(g.view_for(Some(0)).night_score[0], 1);
}

#[test]
fn wheel_nuts_tighten_clockwise_only() {
    let mut g = game();
    g.actors[0].tire = 10.;
    let car = g.actors[0].car_at;
    put(&mut g, 0, (car.0 + 1.5, car.1));
    press_a(&mut g, 0);
    assert_eq!(view(&g, 0).kind, Some(CheckKind::LugNuts));
    let mut angle = 0.;
    for _ in 0..60 * 8 {
        angle -= 0.35;
        g.check_input(0, CheckInput::Spin { angle });
        tick(&mut g, DT);
        if me(&g, 0).check.is_none() {
            break;
        }
    }
    assert_eq!(me(&g, 0).tire, 0., "fixed");
}

#[test]
fn bots_pass_some_checks_and_fail_others() {
    let mut passes = 0;
    let mut fails = 0;
    for seed in 0..40 {
        let mut g = CityGame::seeded(&[(0, true), (1, true)], CityConfig::default(), false, seed);
        // A lone robber's post is the power switch.
        g.briefing_left = 0.;
        g.time = 3.;
        // Out of sight, so nobody loses their nerve.
        put(&mut g, 0, CREW_SPAWN);
        put(&mut g, 1, POWER);
        g.actors[1].interact = true;
        let i = g.index_of(1).unwrap();
        g.start_check(i, Station::Power, POWER);
        tick(&mut g, 8.);
        passes += g.check_passes[CheckKind::Wires as usize];
        fails += g.check_fails[CheckKind::Wires as usize];
    }
    assert!(passes > 0 && fails > 0, "passes {passes} fails {fails}");
}

#[test]
fn a_client_cannot_claim_a_result() {
    let mut g = game();
    put(&mut g, 1, POWER);
    press_a(&mut g, 1);
    // Inputs for another check, or garbage, do nothing.
    g.check_input(1, CheckInput::Trigger { value: 1. });
    g.check_input(1, CheckInput::Dial { at: f32::NAN });
    assert!(me(&g, 1).check.is_some());
    assert_eq!(g.power, 0.);
    // No check running: inputs are refused.
    assert!(!g.check_input(2, CheckInput::Press));
}
