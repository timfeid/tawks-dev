use super::*;
use crate::city_slip::SlipInput;
const DT: f32 = 1. / 60.;
fn game() -> CityGame {
    let mut g = CityGame::new(
        &[(0, false), (1, false), (2, false)],
        CityConfig::default(),
        false,
    );
    g.briefing_left = 0.;
    g
}
#[test]
fn vault_bot_finishes_active_dial_before_covering_a_missing_post() {
    let seats: Vec<_> = (0..5).map(|seat| (seat, true)).collect();
    let mut g = CityGame::seeded(
        &seats,
        CityConfig {
            police: 1,
            ..Default::default()
        },
        true,
        42,
    );
    g.briefing_left = 0.;
    g.time = 30.;
    g.job = Job::Vault;
    g.power = 1.5;
    g.cameras = 20.;
    at(&mut g, 1, BANK);
    g.actors[2].jailed = true;
    at(&mut g, 4, ESCAPE);
    assert!(g.post_uncovered(hud::Post::Power, POWER));
    assert!(g.start_check(1, Station::Vault, BANK));
    assert_eq!(g.ai_goal(1), (BANK, true));
    g.actors[1].check = None;
    assert_eq!(g.ai_goal(1), (POWER, true));
}
#[test]
fn solo_officer_checks_the_bank_late_in_a_two_robber_night() {
    let mut g = CityGame::new(
        &[(0, true), (1, true), (2, true)],
        CityConfig::default(),
        true,
    );
    assert!(g.inspecting(&g.actors[0]).is_none());
    g.time = 239.;
    assert!(g.inspecting(&g.actors[0]).is_none());
    g.time = 241.;
    assert_eq!(g.inspecting(&g.actors[0]), Some(BANK));
}
#[test]
fn two_officers_four_robbers_keep_the_skill_ladder() {
    let seats: Vec<_> = (0..6).map(|seat| (seat, true)).collect();
    let mut crew = Vec::new();
    for selected in [0.15, 0.4, 0.7, 1.0] {
        let g = CityGame::seeded(
            &seats,
            CityConfig {
                police: 2,
                skill: Some(selected),
                ..Default::default()
            },
            true,
            42,
        );
        assert_eq!(g.actors[0].skill, selected);
        assert_eq!(g.actors[1].skill, selected);
        assert!(g.actors[2..].iter().all(|a| a.skill == g.actors[2].skill));
        crew.push(g.actors[2].skill);
    }
    assert!(crew.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(crew[0] > 0.15);
    assert_eq!(crew[1], 0.4);
    assert!(crew[2] < 0.7);
    assert!(crew[3] < 1.0);

    let other = CityGame::seeded(
        &seats[..5],
        CityConfig {
            police: 2,
            skill: Some(0.7),
            ..Default::default()
        },
        true,
        42,
    );
    assert!(other.actors.iter().all(|a| a.skill == 0.7));
}
fn at(game: &mut CityGame, seat: u8, point: (f32, f32)) {
    let a = game.actors.iter_mut().find(|a| a.seat == seat).unwrap();
    a.at = point;
    a.stun = 0.;
    a.immune = 0.;
    a.in_car = false;
    game.briefing_left = 0.;
}
/// Press A at a station and pass whatever check it starts, then let time run.
fn hold(game: &mut CityGame, seat: u8, seconds: f32) {
    game.control(seat, true, false);
    game.tick(DT, &[], &[0, 1, 2], |_, _| false);
    game.qa_pass_check(seat);
    game.control(seat, false, false);
    for _ in 0..(seconds * 60.) as usize {
        game.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
}
/// Hold E to grab, then trace the figure eight at a steady pace.
fn cuff(game: &mut CityGame, seat: u8, seconds: f32) {
    let at = game.actors.iter().find(|a| a.seat == seat).unwrap().at;
    if let Some(target) = game.actors.iter_mut()
        .filter(|a| a.side == Side::Crew && !a.jailed && distance(a.at, at) < 2.5)
        .min_by(|a, b| distance(a.at, at).total_cmp(&distance(b.at, at))) {
        target.stun = target.stun.max(2.6);
    }
    for _ in 0..(seconds * 60.) as usize {
        game.control(seat, true, false);
        if let Some(t) = game.arrests.iter().find(|x| x.officer == seat).map(|x| x.t) {
            let p = arrest::figure_eight(t + std::f32::consts::TAU / 2.5 / 60.);
            game.trace(seat, p.0, p.1);
        }
        game.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
}
#[test]
fn bank_job_requires_power_loot_and_escape_then_resolves_once() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, BANK);
    hold(&mut g, 1, 2.);
    assert_eq!(g.vault, 0.);
    at(&mut g, 2, POWER);
    hold(&mut g, 2, 3.3);
    assert!(g.power > 0.);
    hold(&mut g, 1, 0.1);
    assert!(g.alarm);
    assert_eq!(g.job, Job::Carry);
    let bag = g.bags[0].at;
    at(&mut g, 1, bag);
    hold(&mut g, 1, 0.1);
    assert!(g.carrying(1));
    assert!(g.resolved.is_none());
    at(&mut g, 1, ESCAPE);
    hold(&mut g, 1, 4.1);
    assert!(
        g.resolved.is_none(),
        "an extraction must not end the six-minute night"
    );
    assert_eq!(g.night_score, [0, 1]);
    hold(&mut g, 1, 3.);
    assert_eq!(g.night_score, [0, 1]);
    g.time = LIMIT + 2.99;
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(g.resolved, Some(Side::Crew));
    assert_eq!(g.scores, [0, 1]);
}
#[test]
fn police_snapshot_cannot_reveal_crew_objectives_or_hidden_positions() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    let crew = g.view_for(Some(1));
    let police = g.view_for(Some(0));
    let spectator = g.view_for(None);
    assert_eq!(crew.escape, Some([ESCAPE.0, ESCAPE.1]));
    assert!(!crew.targets.is_empty());
    assert!(
        police.escape.is_none()
            && police.targets.iter().all(|t| t.label.starts_with("PATROL"))
            && police.vault_progress.is_none()
    );
    assert!(spectator.targets.is_empty());
    assert!(
        police
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .all(|a| a.x.is_none() && a.z.is_none())
    );
    at(&mut g, 1, (JAIL.0 - 1., JAIL.1));
    assert!(
        g.view_for(Some(0))
            .actors
            .iter()
            .find(|a| a.seat == 1)
            .unwrap()
            .x
            .is_some()
    );
}
#[test]
fn arrest_is_proximity_based_and_a_teammate_can_rescue() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, JAIL);
    cuff(&mut g, 0, 3.2);
    assert!(g.actors[1].jailed);
    assert!(g.resolved.is_none());
    at(&mut g, 0, (45., 45.));
    at(&mut g, 2, (JAIL.0 - 1., JAIL.1));
    hold(&mut g, 2, 0.1);
    assert!(!g.actors[1].jailed);
    assert!(g.actors[1].immune > 0.);
}
#[test]
fn all_ai_produces_a_bounded_event_result_with_one_or_two_police() {
    for seats in [4u8, 6, 8] {
        for police in [1, 2] {
            if !formats::allowed(police, usize::from(seats)) {
                continue;
            }
            let players: Vec<_> = (0..seats).map(|s| (s, true)).collect();
            let mut g = CityGame::seeded(
                &players,
                CityConfig {
                    police,
                    jobs: 1,
                    host_police: false,
                    ..Default::default()
                },
                true,
                1,
            );
            let eligible: Vec<_> = (0..seats).collect();
            let mut board = crate::board::Board::for_room(
                crate::board::Map::Frozen,
                &eligible,
                10,
                1,
            );
            board.use_manual_events();
            // Slow motion stretches the night a little in real ticks.
            for _ in 0..60 * 420 {
                let world = DT * g.time_scale();
                board.traffic_tick_city(world, &g.patrol_traffic_obstacles(), &g.patrol_sirens());
                g.tick_streets(DT, &[], &eligible, &mut board);
                if g.resolved.is_some() {
                    break;
                }
            }
            assert!(
                g.resolved.is_some(),
                "bots finish {police}v{}",
                seats - police
            );
            assert!(
                g.time >= LIMIT + 3. && g.time < LIMIT + 3.1,
                "every night runs for six minutes: {}",
                g.time
            );
            assert!(
                g.night_score.iter().sum::<u16>() > 0,
                "bots must complete real jobs in {police}v{}: {:?}",
                seats - police,
                g.view_for(Some(police))
            );
        }
    }
}
#[test]
fn controls_expire_and_nonfinite_movement_cannot_escape_the_city() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, BANK);
    g.control(1, true, true);
    let old = g.actors[1].at;
    for _ in 0..60 {
        g.tick(
            DT,
            &[(
                1,
                Stick {
                    x: f32::NAN,
                    z: f32::INFINITY,
                },
                f32::NAN,
            )],
            &[0, 1, 2],
            |_, _| false,
        );
    }
    assert_eq!(g.actors[1].at, old);
    assert!(!g.actors[1].interact && !g.actors[1].suction);
}
#[test]
fn smoke_breaks_line_of_sight_and_scanner_does_not_reveal_identity() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, (JAIL.0 - 2., JAIL.1));
    assert!(g.visible(g.actors[0].at, g.actors[1].at, 25.));
    assert!(g.use_tool(1, Tool::Smoke));
    assert!(!g.visible(g.actors[0].at, g.actors[1].at, 25.));
    assert!(!g.use_tool(1, Tool::Smoke));
    assert!(!g.use_tool(1, Tool::Scanner), "the radio is police kit");
    assert!(g.use_tool(0, Tool::Scanner));
    assert_eq!(
        g.view_for(Some(0)).scanner,
        Some("Strong signal · very close")
    );
}
#[test]
fn robber_bots_can_do_the_whole_bank_job_and_escape() {
    let mut g = CityGame::new(
        &[(0, false), (1, true), (2, true)],
        CityConfig {
            police: 1,
            jobs: 1,
            host_police: true,
            ..Default::default()
        },
        false,
    );
    for _ in 0..60 * 120 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        if g.resolved.is_some() {
            break;
        }
    }
    assert!(g.night_score[1] > 0, "robber bots must deliver bags");
    assert!(
        g.resolved.is_none(),
        "the night keeps running after delivery"
    );
}
#[test]
fn a_police_bot_can_arrest_without_a_human_driving_its_controls() {
    let mut g = CityGame::new(
        &[(0, true), (1, false)],
        CityConfig {
            police: 1,
            jobs: 1,
            host_police: false,
            ..Default::default()
        },
        false,
    );
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, (JAIL.0 - 1., JAIL.1));
    for _ in 0..60 * 5 {
        g.tick(DT, &[], &[0, 1], |_, _| false);
        if g.resolved.is_some() {
            break;
        }
    }
    assert!(
        g.night_score[0] > 0,
        "a police bot must stop a crew without ending the night"
    );
    assert!(g.resolved.is_none());
}
#[test]
fn one_officer_cannot_mass_arrest_a_group_in_a_single_hold() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, (JAIL.0 - 1., JAIL.1));
    at(&mut g, 2, (JAIL.0 + 1., JAIL.1));
    cuff(&mut g, 0, 3.2);
    assert_eq!(g.actors.iter().filter(|a| a.jailed).count(), 1);
}
#[test]
fn scanner_reports_a_coarse_sector_without_exposing_a_hidden_actor() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, (-28., 28.));
    at(&mut g, 1, (-18., 30.));
    assert!(g.use_tool(1, Tool::Smoke));
    assert!(g.use_tool(0, Tool::Scanner));
    let view = g.view_for(Some(0));
    assert!(
        view.actors
            .iter()
            .find(|a| a.seat == 1)
            .unwrap()
            .x
            .is_none()
    );
    let bearing = view.scanner_bearing.unwrap();
    assert!(
        (bearing / std::f32::consts::FRAC_PI_4 - (bearing / std::f32::consts::FRAC_PI_4).round())
            .abs()
            < 0.001
    );
    assert!(view.escape.is_none());
    assert!(g.view_for(Some(2)).scanner_bearing.is_none());
}

#[test]
fn ai_demo_survives_jobs_and_taking_control_is_not_undone_by_a_new_job() {
    let mut g = CityGame::new(
        &[(0, false), (1, true), (2, true), (3, true)],
        CityConfig::default(),
        true,
    );
    g.next_job(&[]);
    assert!(g.automated(0));
    assert!(g.view_for(Some(0)).actors.iter().all(|a| a.x.is_some()));
    g.automate(0, false);
    g.next_job(&[]);
    assert!(!g.automated(0));
    assert!(
        g.view_for(Some(0))
            .actors
            .iter()
            .filter(|a| a.side == Side::Crew)
            .all(|a| a.x.is_none())
    );
}

#[test]
fn opening_briefing_freezes_clock_and_ai_until_every_human_is_ready() {
    let mut g = CityGame::new(
        &[(0, false), (1, false), (2, true)],
        CityConfig::default(),
        false,
    );
    let cop = g.actors[0].at;
    for _ in 0..120 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| true);
    }
    assert_eq!(g.time, 0.);
    assert_eq!(g.actors[0].at, cop);
    assert_eq!(g.night_score, [0, 0]);
    g.ready(0);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(g.briefing());
    g.ready(1);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(!g.briefing());
    assert_eq!(g.view_for(Some(0)).remaining, 360.);
}
#[test]
fn briefing_car_steers_like_the_live_chase() {
    let mut g = CityGame::new(&[(0, false), (1, true)], CityConfig::default(), false);
    let initial = g.actors[0].at;
    let yaw = g.actors[0].car_yaw;
    for _ in 0..60 {
        g.tick(DT, &[(0, Stick { x: 1., z: 0. }, 0.)], &[0, 1], |_, _| false);
    }
    assert_eq!(g.actors[0].at, initial, "D alone should not slide a parked car");
    for _ in 0..60 {
        g.tick(DT, &[(0, Stick { x: 1., z: 1. }, 0.)], &[0, 1], |_, _| false);
    }
    assert!(g.briefing());
    assert!(wrap(g.actors[0].car_yaw - yaw) < -0.1, "W+D should steer right during briefing");
    assert_ne!(g.actors[0].at, initial);
}
#[test]
fn briefing_auto_starts_after_twenty_seconds_and_does_not_repeat_on_night_two() {
    let mut g = CityGame::new(&[(0, false), (1, true)], CityConfig::default(), false);
    for _ in 0..60 * 21 {
        g.tick(DT, &[], &[0, 1], |_, _| false);
    }
    assert!(!g.briefing());
    assert!(g.time < 1.1);
    g.next_job(&[]);
    assert!(!g.briefing());
    assert_eq!(g.time, 0.);
}
#[test]
fn patrol_cars_use_roads_and_officers_must_get_out_to_arrest() {
    let mut g = game();
    g.time = 3.;
    let old = g.actors[0].at;
    for _ in 0..60 {
        g.tick(
            DT,
            &[(0, Stick { x: 0., z: 1. }, 0.)],
            &[0, 1, 2],
            |_, _| false,
        );
    }
    assert!(distance(old, g.actors[0].at) > 5.);
    assert!(on_road(g.actors[0].at));
    assert_eq!(
        g.view_for(Some(0)).actors[0].car,
        Some([g.actors[0].at.0, g.actors[0].at.1])
    );
    let cop = g.actors[0].at;
    at(&mut g, 1, cop);
    hold(&mut g, 0, 2.);
    assert!(
        !g.actors[1].jailed,
        "cars cannot arrest by driving into someone"
    );
    if g.actors[0].in_car {
        assert!(g.toggle_car(0));
    }
    let cop = g.actors[0].at;
    at(&mut g, 1, (cop.0 + 0.5, cop.1));
    cuff(&mut g, 0, 3.2);
    assert!(g.actors[1].jailed);
    assert!(!g.toggle_car(1), "crew cannot take a police car");
}
#[test]
fn jail_has_automatic_bail_and_six_minute_ties_are_draws() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, JAIL);
    cuff(&mut g, 0, 3.2);
    assert!(g.actors[1].jailed);
    at(&mut g, 0, (60., 60.));
    let bail_ticks = ((g.tuning.jail_seconds + 1.) / DT).ceil() as usize;
    for _ in 0..bail_ticks {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert!(!g.actors[1].jailed);
    assert!(
        walkable(g.actors[1].at),
        "bail must release onto navigable ground"
    );
    assert!(!route(g.actors[1].at, POWER, false).is_empty());
    assert!(g.resolved.is_none());
    g.time = LIMIT + 2.99;
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(g.draw);
    assert_eq!(g.scores, [0, 0]);
    assert_eq!(g.standings().len(), 1);
}

#[test]
fn patrol_impact_launches_the_robber_drops_loot_and_forces_the_officer_out() {
    let mut g = game();
    g.time = 3.;
    let cop = g.actors[0].at;
    g.actors[0].drive_speed = 12.;
    g.actors[0].car_yaw = std::f32::consts::FRAC_PI_2;
    g.actors[0].heading = std::f32::consts::FRAC_PI_2;
    at(&mut g, 1, (cop.0 + 1.8, cop.1));
    g.bags = vec![bags::Bag { at: (0., 0.), carrier: Some(1) }];
    g.tick(
        0.05,
        &[(0, Stick { x: 1., z: 0. }, 0.)],
        &[0, 1, 2],
        |_, _| false,
    );
    assert!(
        !g.actors[0].in_car,
        "driver must park and get out after a hit"
    );
    assert_eq!(g.actors[0].drive_speed, 0.);
    assert!(g.actors[1].height > 0. && g.actors[1].stun > 0.9);
    assert!(!g.actors[1].jailed, "ramming is not an automatic arrest");
    assert!(!g.carrying(1) && !g.loose_bags().is_empty());
    let launch = g.actors[1].at;
    let mut peak = 0f32;
    for _ in 0..110 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        peak = peak.max(g.actors[1].height);
    }
    assert!(peak > 0.9);
    assert!(distance(launch, g.actors[1].at) > 4.);
    assert_eq!(g.actors[1].height, 0.);
    assert_eq!(g.actors[1].stun, 0.);
}

#[test]
fn patrol_car_accelerates_and_coasts_instead_of_stopping_between_input_packets() {
    let mut g = game();
    g.time = 3.;
    g.actors[0].car_yaw = std::f32::consts::FRAC_PI_2;
    for _ in 0..60 {
        g.tick(
            DT,
            &[(0, Stick { x: 0., z: 1. }, 0.)],
            &[0, 1, 2],
            |_, _| false,
        );
    }
    let before = g.actors[0].at;
    let speed = g.actors[0].drive_speed;
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(distance(before, g.actors[0].at) > 0.05);
    assert!(g.actors[0].drive_speed < speed);
    for _ in 0..60 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.actors[0].drive_speed, 0.);
}

#[test]
fn stopped_traffic_blocks_the_patrol_car_without_teleporting_its_officer() {
    let mut g = game();
    g.time = 3.;
    g.actors[0].drive_speed = 10.;
    let before = g.actors[0].at;
    let mut checked_vehicle = false;
    g.tick_with_vehicle_traffic(
        DT,
        &[(0, Stick { x: 1., z: 0. }, 0.)],
        &[0, 1, 2],
        |_, _, yaw| {
            checked_vehicle |= yaw.is_some();
            yaw.is_some()
        },
    );
    assert!(checked_vehicle);
    assert_eq!(g.actors[0].at, before);
    assert_eq!(g.actors[0].car_at, before);
    assert_eq!(g.actors[0].drive_speed, 0.);
    assert_eq!(g.actors[0].velocity, (0., 0.));
    assert!(g.actors[0].in_car);
}

#[test]
fn patrol_footprint_stays_with_the_parked_car_when_an_officer_walks_away() {
    let mut g = game();
    let car = g.actors[0].car_at;
    g.actors[0].in_car = false;
    g.actors[0].at = (car.0 + 8., car.1 + 8.);
    let points = g.patrol_traffic_obstacles();
    assert_eq!(points.len(), 3);
    assert!(points.contains(&car));
    assert!(!points.contains(&g.actors[0].at));
}

#[test]
fn red_crossing_wait_applies_to_ai_pedestrians_but_not_human_inputs() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 1, CREW_SPAWN);
    at(&mut g, 2, CREW_SPAWN);
    g.actors[1].automated = true;
    let ai = g.actors[1].at;
    let human = g.actors[2].at;
    g.tick_with_traffic(
        DT,
        &[(2, Stick { x: 1., z: 0. }, 0.)],
        &[0, 1, 2],
        |_, _, _| false,
        |_, _| true,
    );
    assert_eq!(g.actors[1].at, ai);
    assert_eq!(g.actors[1].velocity, (0., 0.));
    assert!(distance(g.actors[2].at, human) > 0.01);
}

fn held_game() -> CityGame {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, (16., 4.));
    at(&mut g, 1, (16.8, 4.));
    g.actors[1].stun = 2.;
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.arrests.len(), 1, "a held E grabs after a short reach");
    g
}
#[test]
fn grabbing_starts_a_cuffing_challenge_without_automatic_slow_motion() {
    let mut g = held_game();
    assert_eq!(g.time_scale(), 1.);
    let crew_view = g.view_for(Some(2));
    assert_eq!(crew_view.slow, 1.);
    let police_view = g.view_for(Some(0));
    assert_eq!(police_view.arrests.len(), 1);
    assert_eq!(police_view.arrests[0].stage, Stage::Cuffing);
    let robber = g.actors[1].at;
    g.tick(
        DT,
        &[(1, Stick { x: 1., z: 0. }, 0.)],
        &[0, 1, 2],
        |_, _| false,
    );
    assert_eq!(g.actors[1].at, robber, "a held robber cannot walk away");
    for _ in 0..60 * 3 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.time_scale(), 1., "slow motion is brief");
}
#[test]
fn an_untraced_cuffing_times_out_and_the_officer_must_wait_to_retry() {
    let mut g = held_game();
    for _ in 0..60 * 11 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert!(g.arrests.is_empty(), "the robber slips free");
    assert!(g.actors[0].cuff_cooldown > 0.);
    assert!(g.actors[1].immune > 0.);
    g.control(0, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(g.arrests.is_empty());
}
#[test]
fn slow_motion_has_a_cooldown_so_back_to_back_grabs_cannot_spam_it() {
    let mut g = held_game();
    let index = g.arrests.len() - 1;
    g.release(index, 0., Release::OfficerGone);
    for a in &mut g.actors {
        a.cuff_cooldown = 0.;
        a.immune = 0.;
    }
    for _ in 0..60 * 3 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    at(&mut g, 2, (16.6, 4.));
    g.actors[2].stun = 2.;
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.arrests.len(), 1);
    assert_eq!(
        g.time_scale(),
        1.,
        "a second grab inside the cooldown does not slow the world"
    );
}
#[test]
fn cuffed_robbers_are_dragged_to_the_car_and_jailed_on_arrival() {
    let mut g = held_game();
    let car = g.actors[0].car_at;
    cuff(&mut g, 0, 2.7);
    assert_eq!(g.arrests[0].stage, Stage::Escort);
    let start = distance(g.actors[0].at, car);
    for _ in 0..60 * 20 {
        let to = g.actors[0].car_at;
        let from = g.actors[0].at;
        let d = distance(from, to).max(0.01);
        g.tick(
            DT,
            &[(
                0,
                Stick {
                    x: (to.0 - from.0) / d,
                    z: (to.1 - from.1) / d,
                },
                0.,
            )],
            &[0, 1, 2],
            |_, _| false,
        );
        if g.arrests.is_empty() {
            break;
        }
        assert!(
            distance(g.actors[1].at, g.actors[0].at) < 2.5,
            "the robber is dragged along"
        );
    }
    assert!(start > arrest::DELIVER_RANGE);
    assert!(g.actors[1].jailed);
}
#[test]
fn a_robber_who_slips_the_cuffs_breaks_free_and_stuns_the_officer() {
    let mut g = held_game();
    cuff(&mut g, 0, 2.7);
    assert_eq!(g.arrests[0].stage, Stage::Escort);
    let mut ticks = 0;
    let mut answered = -1.;
    for _ in 0..60 * 12 {
        let Some(view) = g.arrests.first().and_then(|x| x.cuffs.as_ref()).map(|c| c.view()) else {
            break;
        };
        g.slip_input(1, SlipInput::Aim { angle: view.target });
        if let Some(cue) = view.cue.filter(|c| !c.fake) {
            let shown = view.clock - (crate::city_slip::CUE_SECONDS - cue.left);
            if view.clock - shown > 0.2 && (shown - answered).abs() > 0.01 {
                answered = shown;
                g.slip_input(1, SlipInput::Flick { dir: -cue.dir });
            }
        }
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        if g.arrests.is_empty() {
            break;
        }
        ticks += 1;
    }
    assert!(
        g.arrests.is_empty() && !g.actors[1].jailed,
        "four clean seconds slip the cuffs"
    );
    assert!(ticks < 60 * 5, "{ticks}");
    assert!(g.actors[0].stun > 0.);
    assert!(g.actors[0].cuff_cooldown > 0.);
}
#[test]
fn flicking_wildly_never_slips_the_cuffs() {
    let mut g = held_game();
    cuff(&mut g, 0, 2.7);
    for tick in 0..60 * 11 {
        if let Some(view) = g.arrests.first().and_then(|x| x.cuffs.as_ref()).map(|c| c.view()) {
            g.slip_input(1, SlipInput::Aim { angle: view.target });
        }
        g.slip_input(1, SlipInput::Flick { dir: if tick % 2 == 0 { 1 } else { -1 } });
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    let x = &g.arrests[0];
    assert_eq!(
        x.cuffs.as_ref().and_then(|c| c.outcome),
        Some(crate::city_slip::Outcome::Held)
    );
}
#[test]
fn an_officer_disconnecting_mid_arrest_releases_the_robber() {
    let mut g = held_game();
    g.tick(DT, &[], &[1, 2], |_, _| false);
    assert!(g.arrests.is_empty());
    assert!(!g.actors[1].jailed);
    assert!(g.actors[1].immune > 0.);
}
#[test]
fn a_ram_dazes_the_robber_long_enough_to_reach_them_on_foot() {
    let mut g = game();
    g.time = 3.;
    let cop = g.actors[0].at;
    g.actors[0].drive_speed = 12.;
    g.actors[0].car_yaw = std::f32::consts::FRAC_PI_2;
    g.actors[0].heading = std::f32::consts::FRAC_PI_2;
    at(&mut g, 1, (cop.0 + 1.8, cop.1));
    g.tick(
        0.05,
        &[(0, Stick { x: 1., z: 0. }, 0.)],
        &[0, 1, 2],
        |_, _| false,
    );
    assert!(g.actors[1].stun > 1.5);
    assert!(g.view_for(Some(0)).impacts.iter().any(|i| i.kind == "ram"));
    assert!(!g.struggle(1), "a dazed robber cannot struggle yet");
}
#[test]
fn a_fast_patrol_car_shoves_traffic_aside_and_slows_instead_of_stopping() {
    struct Shoving(u8);
    impl Streets for Shoving {
        fn blocked(&mut self, _: (f32, f32), _: (f32, f32), yaw: Option<f32>) -> bool {
            yaw.is_some() && self.0 == 0
        }
        fn crossing_wait(&mut self, _: (f32, f32), _: (f32, f32)) -> bool {
            false
        }
        fn shove(&mut self, _: (f32, f32), _: (f32, f32), _: f32, _: f32) -> u8 {
            self.0 += 1;
            1
        }
    }
    let mut g = game();
    g.time = 3.;
    g.actors[0].drive_speed = 12.;
    let before = g.actors[0].at;
    g.tick_streets(
        DT,
        &[(0, Stick { x: 0., z: 1. }, 0.)],
        &[0, 1, 2],
        &mut Shoving(0),
    );
    assert!(g.actors[0].drive_speed > 5. && g.actors[0].drive_speed < 10.);
    assert_ne!(g.actors[0].at, before);
}
#[test]
fn turning_is_rate_limited_and_reverse_does_not_flip_the_car() {
    let mut g = game();
    g.time = 3.;
    let skew = 0.25f32;
    for _ in 0..90 {
        g.tick(
            DT,
            &[(
                0,
                Stick {
                    x: skew.sin(),
                    z: skew.cos(),
                },
                0.,
            )],
            &[0, 1, 2],
            |_, _| false,
        );
    }
    assert!(g.actors[0].car_yaw < -0.1, "right input turns the car right");
    let mut g = game();
    g.time = 3.;
    g.tick(
        DT,
        &[(0, Stick { x: 0., z: -1. }, 0.)],
        &[0, 1, 2],
        |_, _| false,
    );
    assert!(wrap(g.actors[0].car_yaw).abs() < 0.2, "no instant U-turn");
}
#[test]
fn a_check_shows_only_to_the_player_working_it() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 1, POWER);
    at(&mut g, 2, BANK);
    g.control(1, true, false);
    g.control(2, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(g.view_for(Some(1)).check.and_then(|c| c.kind), Some(CheckKind::Wires));
    assert!(
        g.view_for(Some(2)).check.is_none(),
        "the vault has no check until the power is cut"
    );
    assert!(g.view_for(Some(0)).check.is_none());
}

fn notice_for(g: &CityGame, seat: u8) -> Option<&'static str> {
    g.view_for(Some(seat)).notice
}

#[test]
fn a_cuffing_that_times_out_tells_both_sides_and_benches_the_officer() {
    let mut g = held_game();
    for _ in 0..60 * 11 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        if g.arrests.is_empty() {
            break;
        }
    }
    assert!(g.arrests.is_empty());
    let (officer, robber) = Release::CuffTimeout.messages();
    assert_eq!(notice_for(&g, 0), Some(officer));
    assert_eq!(notice_for(&g, 1), Some(robber));
    assert_eq!(notice_for(&g, 2), None, "bystanders are not told");
    assert!(g.view_for(Some(0)).cuff_ready > 4., "the failed arrest benches the officer");
    for _ in 0..60 * 4 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(notice_for(&g, 0), None, "the message is short");
}

#[test]
fn a_job_ending_mid_escort_calls_the_arrest_off_with_a_message() {
    let mut g = held_game();
    cuff(&mut g, 0, 2.7);
    assert_eq!(g.arrests[0].stage, Stage::Escort);
    g.end_job("The crew got away");
    for _ in 0..60 * 9 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        if g.arrests.is_empty() {
            break;
        }
    }
    assert!(g.arrests.is_empty(), "a new job frees the robber");
    assert!(!g.actors[1].jailed);
    let (officer, robber) = Release::JobOver.messages();
    assert_eq!(notice_for(&g, 0), Some(officer));
    assert_eq!(notice_for(&g, 1), Some(robber));
    assert!(g.view_for(Some(0)).cuff_ready > 0.);
}

#[test]
fn an_officer_knocked_down_mid_cuff_loses_the_robber_and_both_hear_why() {
    let mut g = held_game();
    g.actors[0].stun = 0.8;
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert!(g.arrests.is_empty());
    let (officer, robber) = Release::OfficerDown.messages();
    assert_eq!(notice_for(&g, 0), Some(officer));
    assert_eq!(notice_for(&g, 1), Some(robber));
    assert!(g.view_for(Some(0)).cuff_ready > 0.);
}

/// Streets where every patrol car move is blocked, except well clear of the jam.
struct Jammed {
    nudges: u32,
}
impl Streets for Jammed {
    fn blocked(&mut self, from: (f32, f32), _to: (f32, f32), yaw: Option<f32>) -> bool {
        yaw.is_some() && distance(from, (16., 16.)) < 6.
    }
    fn crossing_wait(&mut self, _: (f32, f32), _: (f32, f32)) -> bool {
        false
    }
    fn nudge(&mut self, _: (f32, f32), _: (f32, f32), _: f32, _: f32) -> u8 {
        self.nudges += 1;
        1
    }
    fn clear_for_car(&mut self, at: (f32, f32), _: f32) -> bool {
        distance(at, (16., 16.)) > 6.
    }
}

#[test]
fn a_car_stuck_while_its_driver_presses_on_can_be_reset_onto_a_clear_lane() {
    let mut g = game();
    g.time = 3.;
    let mut streets = Jammed { nudges: 0 };
    g.qa_place_car(0, (16., 16.), 0.);
    assert!(!g.reset_car(0, &mut streets), "a moving car cannot be reset");
    let press = [(0, Stick { x: 0., z: 1. }, 0.)];
    for _ in 0..60 * 2 {
        g.tick_streets(DT, &press, &[0, 1, 2], &mut streets);
    }
    assert!(streets.nudges > 0, "pressing into the jam pushes on it");
    assert!(!g.view_for(Some(0)).car_stuck, "not yet");
    for _ in 0..60 * 2 {
        g.tick_streets(DT, &press, &[0, 1, 2], &mut streets);
    }
    assert!(g.view_for(Some(0)).car_stuck, "three seconds of pressing without moving");
    assert!(g.reset_car(0, &mut streets));
    let a = &g.actors[0];
    assert!(distance(a.at, (16., 16.)) > 6. && on_road(a.at), "on a clear lane: {:?}", a.at);
    let quarter = std::f32::consts::FRAC_PI_2;
    assert!((a.car_yaw / quarter - (a.car_yaw / quarter).round()).abs() < 0.001, "facing along the street");
    assert!(!g.view_for(Some(0)).car_stuck);
    assert!(g.view_for(Some(0)).notice.is_some(), "the driver is told");
}

#[test]
fn a_bot_driver_stuck_in_a_jam_resets_itself() {
    let mut g = game();
    g.time = 3.;
    g.automate(0, true);
    let mut streets = Jammed { nudges: 0 };
    g.qa_place_car(0, (16., 16.), 0.);
    let mut freed = false;
    for _ in 0..60 * 8 {
        g.tick_streets(DT, &[], &[0, 1, 2], &mut streets);
        if g.actors[0].in_car && distance(g.actors[0].at, (16., 16.)) > 6. {
            freed = true;
            break;
        }
    }
    assert!(freed, "the bot driver is back on the road: {:?}", g.actors[0].at);
}

#[test]
#[ignore = "known car waypoint orbit; correcting it needs all-format police balance tuning"]
fn bot_driver_passes_grid_waypoints_instead_of_orbiting_them() {
    use crate::board::{Board, Map};
    let seats: Vec<_> = (0u8..6).map(|seat| (seat, true)).collect();
    let ids: Vec<_> = (0u8..6).collect();
    let mut g = CityGame::seeded(
        &seats,
        CityConfig {
            police: 2,
            jobs: 1,
            skill: Some(1.),
            ..Default::default()
        },
        true,
        154022,
    );
    let mut board = Board::for_room(Map::Frozen, &ids, 10, 154022);
    board.use_manual_events();
    let mut first = None;
    let mut max_excursion = 0_f32;
    while g.time < 200. && g.resolved.is_none() {
        board.traffic_tick_city(
            DT * g.time_scale(),
            &g.patrol_traffic_obstacles(),
            &g.patrol_sirens(),
        );
        g.tick_streets(DT, &[], &ids, &mut board);
        if first.is_none() && g.time >= 120. {
            first = Some(g.actors[0].at);
        }
        if let Some(start) = first {
            max_excursion = max_excursion.max(distance(start, g.actors[0].at));
        }
    }
    assert!(
        max_excursion > 10.,
        "bot car circled the same route point: {:?} -> {:?}, max excursion {max_excursion:.1}",
        first,
        g.actors[0].at,
    );
}

/// Drive straight up the z axis, then throw the wheel right. Returns the
/// largest gap between where the car points and where it goes, and the car.
fn hard_turn(start: (f32, f32)) -> (f32, CityGame) {
    let mut g = game();
    g.time = 3.;
    g.qa_place_car(0, start, 0.);
    let go = |g: &mut CityGame, stick: Stick, seconds: f32| {
        for _ in 0..(seconds * 60.) as usize {
            g.tick(DT, &[(0, stick, 0.)], &[0, 1, 2], |_, _| false);
        }
    };
    go(&mut g, Stick { x: 0., z: 1. }, 1.2);
    let mut drift: f32 = 0.;
    for _ in 0..15 {
        g.tick(DT, &[(0, Stick { x: 1., z: 1. }, 0.)], &[0, 1, 2], |_, _| false);
        drift = drift.max(wrap(g.actors[0].car_yaw - g.actors[0].heading).abs());
    }
    (drift, g)
}

#[test]
fn off_the_road_a_car_slides_on_snow_but_stays_drivable() {
    let open = |p: (f32, f32)| (-12..=12).all(|i| on_snow((p.0 + i as f32, p.1)) && on_snow((p.0, p.1 + i as f32)));
    let snow = (-60..60)
        .step_by(2)
        .flat_map(|x| (-60..60).step_by(2).map(move |z| (x as f32, z as f32)))
        .find(|p| open(*p))
        .expect("an open snowfield in the city");
    let road = (16. + 1.05, -2.);
    assert!(on_road(road) && on_snow(snow));
    let (road_drift, _) = hard_turn(road);
    let (snow_drift, mut g) = hard_turn(snow);
    assert!(snow_drift > 0.3 && snow_drift > road_drift * 2., "snow {snow_drift} road {road_drift}");
    assert!(distance(g.actors[0].at, snow) > 5., "the car drives on snow");
    // Steering into the slide gathers it back up.
    for _ in 0..90 {
        g.tick(DT, &[(0, Stick { x: 0., z: 1. }, 0.)], &[0, 1, 2], |_, _| false);
    }
    assert!(wrap(g.actors[0].car_yaw - g.actors[0].heading).abs() < 0.1, "recovered");
    assert!(g.actors[0].drive_speed > 3., "still under way");
    let view = g.view_for(Some(0));
    let me = view.actors.iter().find(|a| a.seat == 0).unwrap();
    assert!(me.driving && (me.snow || !on_snow(g.actors[0].at)));
}

#[test]
fn a_car_nosed_into_a_building_can_be_reset() {
    let mut g = game();
    g.time = 3.;
    let mut streets = Jammed { nudges: 0 };
    g.qa_place_car(0, (17.5, 22.), std::f32::consts::FRAC_PI_2);
    let press = [(0, Stick { x: 1., z: 0. }, 0.)];
    for _ in 0..60 * 4 {
        g.tick_streets(DT, &press, &[0, 1, 2], &mut streets);
    }
    assert!(g.view_for(Some(0)).car_stuck, "stuck against the wall at {:?}", g.actors[0].at);
    assert!(g.reset_car(0, &mut streets), "reset from {:?}", g.actors[0].at);
}
