use super::*;
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
fn at(game: &mut CityGame, seat: u8, point: (f32, f32)) {
    let a = game.actors.iter_mut().find(|a| a.seat == seat).unwrap();
    a.at = point;
    a.stun = 0.;
    a.immune = 0.;
    a.in_car = false;
    game.briefing_left = 0.;
}
fn hold(game: &mut CityGame, seat: u8, seconds: f32) {
    for _ in 0..(seconds * 60.) as usize {
        game.control(seat, true, true);
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
    hold(&mut g, 2, 2.5);
    assert!(g.power > 0.);
    hold(&mut g, 1, 12.2);
    assert_eq!(g.carrier, Some(1));
    assert!(g.alarm);
    assert!(g.resolved.is_none());
    at(&mut g, 1, ESCAPE);
    hold(&mut g, 1, 3.1);
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
    assert!(police
        .actors
        .iter()
        .filter(|a| a.side == Side::Crew)
        .all(|a| a.x.is_none() && a.z.is_none()));
    at(&mut g, 1, (JAIL.0 - 1., JAIL.1));
    assert!(g
        .view_for(Some(0))
        .actors
        .iter()
        .find(|a| a.seat == 1)
        .unwrap()
        .x
        .is_some());
}
#[test]
fn arrest_is_proximity_based_and_a_teammate_can_rescue() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, JAIL);
    hold(&mut g, 0, 1.6);
    assert!(g.actors[1].jailed);
    assert!(g.resolved.is_none());
    at(&mut g, 0, (45., 45.));
    at(&mut g, 2, (JAIL.0 - 1., JAIL.1));
    hold(&mut g, 2, 3.1);
    assert!(!g.actors[1].jailed);
    assert!(g.actors[1].immune > 0.);
}
#[test]
fn all_ai_produces_a_bounded_event_result_with_one_or_two_police() {
    for seats in [4, 6, 8] {
        for police in [1, 2] {
            let players: Vec<_> = (0..seats).map(|s| (s, true)).collect();
            let mut g = CityGame::new(
                &players,
                CityConfig {
                    police,
                    jobs: 1,
                    host_police: false,
                },
                true,
            );
            let eligible: Vec<_> = (0..seats).collect();
            for _ in 0..60 * 365 {
                g.tick(DT, &[], &eligible, |_, _| false);
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
                "bots must complete real jobs in {police}v{}: {:?}",seats-police,g.view_for(Some(police))
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
    assert!(g.use_tool(1, Tool::Scanner));
    assert_eq!(
        g.view_for(Some(1)).scanner,
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
        },
        false,
    );
    for _ in 0..60 * 100 {
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
    hold(&mut g, 0, 1.7);
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
    assert!(view
        .actors
        .iter()
        .find(|a| a.seat == 1)
        .unwrap()
        .x
        .is_none());
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
    assert!(g
        .view_for(Some(0))
        .actors
        .iter()
        .filter(|a| a.side == Side::Crew)
        .all(|a| a.x.is_none()));
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
fn tutorial_auto_starts_after_a_minute_and_does_not_repeat_on_night_two() {
    let mut g = CityGame::new(&[(0, false), (1, true)], CityConfig::default(), false);
    for _ in 0..60 * 61 {
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
            &[(0, Stick { x: 1., z: 0. }, 0.)],
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
    hold(&mut g, 0, 1.7);
    assert!(g.actors[1].jailed);
    assert!(!g.toggle_car(1), "crew cannot take a police car");
}
#[test]
fn jail_has_automatic_bail_and_six_minute_ties_are_draws() {
    let mut g = game();
    g.time = 3.;
    at(&mut g, 0, JAIL);
    at(&mut g, 1, JAIL);
    hold(&mut g, 0, 1.7);
    assert!(g.actors[1].jailed);
    at(&mut g, 0, (60., 60.));
    for _ in 0..60 * 19 {
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
    at(&mut g, 1, (cop.0 + 1.8, cop.1));
    g.carrier = Some(1);
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
    assert!(g.carrier.is_none() && g.loot.is_some());
    let launch = g.actors[1].at;
    let mut peak = 0f32;
    for _ in 0..66 {
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
            &[(0, Stick { x: 1., z: 0. }, 0.)],
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
    let mut g=game();g.time=3.;
    g.actors[0].drive_speed=10.;
    let before=g.actors[0].at;
    let mut checked_vehicle=false;
    g.tick_with_vehicle_traffic(DT,&[(0,Stick{x:1.,z:0.},0.)],&[0,1,2],|_,_,yaw| {
        checked_vehicle|=yaw.is_some();yaw.is_some()
    });
    assert!(checked_vehicle);
    assert_eq!(g.actors[0].at,before);
    assert_eq!(g.actors[0].car_at,before);
    assert_eq!(g.actors[0].drive_speed,0.);
    assert_eq!(g.actors[0].velocity,(0.,0.));
    assert!(g.actors[0].in_car);
}

#[test]
fn patrol_footprint_stays_with_the_parked_car_when_an_officer_walks_away() {
    let mut g=game();let car=g.actors[0].car_at;
    g.actors[0].in_car=false;g.actors[0].at=(car.0+8.,car.1+8.);
    let points=g.patrol_traffic_obstacles();
    assert_eq!(points.len(),3);
    assert!(points.contains(&car));
    assert!(!points.contains(&g.actors[0].at));
}

#[test]
fn red_crossing_wait_applies_to_ai_pedestrians_but_not_human_inputs() {
    let mut g=game();g.time=3.;
    at(&mut g,1,CREW_SPAWN);at(&mut g,2,CREW_SPAWN);
    g.actors[1].automated=true;
    let ai=g.actors[1].at;let human=g.actors[2].at;
    g.tick_with_traffic(DT,&[(2,Stick{x:1.,z:0.},0.)],&[0,1,2],|_,_,_|false,|_,_|true);
    assert_eq!(g.actors[1].at,ai);
    assert_eq!(g.actors[1].velocity,(0.,0.));
    assert!(distance(g.actors[2].at,human)>0.01);
}
