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
fn view(g: &CityGame, seat: u8) -> CityView {
    g.view_for(Some(seat))
}
fn primary(v: &CityView) -> Option<TargetKind> {
    v.targets.iter().find(|t| t.primary).map(|t| t.kind)
}

#[test]
fn spare_robber_covers_a_jailed_post_without_moving_the_other_posts() {
    let mut g = CityGame::seeded(
        &[(0, true), (1, true), (2, true), (3, true), (4, true)],
        CityConfig {
            police: 1,
            ..Default::default()
        },
        true,
        42,
    );
    assert_eq!(g.crew_post(1), hud::Post::Vault);
    assert_eq!(g.crew_post(2), hud::Post::Power);
    assert_eq!(g.crew_post(3), hud::Post::Cameras);
    assert_eq!(g.crew_post(4), hud::Post::Lure);
    for (jailed, replacement) in [
        (1, hud::Post::Vault),
        (2, hud::Post::Power),
        (3, hud::Post::Cameras),
    ] {
        g.actors
            .iter_mut()
            .find(|a| a.seat == jailed)
            .unwrap()
            .jailed = true;
        assert_eq!(g.crew_post(4), replacement);
        for (seat, post) in [
            (1, hud::Post::Vault),
            (2, hud::Post::Power),
            (3, hud::Post::Cameras),
        ] {
            if seat != jailed {
                assert_eq!(g.crew_post(seat), post);
            }
        }
        g.actors
            .iter_mut()
            .find(|a| a.seat == jailed)
            .unwrap()
            .jailed = false;
    }
}

#[test]
fn the_crew_is_led_from_power_to_vault_to_van() {
    let mut g = game();
    put(&mut g, 1, (0., 0.));
    // Two robbers split the job: the player takes the vault, a bot the power.
    g.actors[2].automated = true;
    let v = view(&g, 2);
    assert_eq!(v.step, Some(Step::CutPower));
    assert_eq!(primary(&v), Some(TargetKind::Power));
    assert_eq!(view(&g, 1).step, Some(Step::CrackVault));
    g.actors[2].automated = false;
    g.job = Job::Vault;
    g.power = 20.;
    let v = view(&g, 1);
    assert_eq!(v.step, Some(Step::CrackVault));
    assert_eq!(primary(&v), Some(TargetKind::Vault));
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (0., 0.),
        carrier: Some(1),
    }];
    let v = view(&g, 1);
    assert_eq!(v.step, Some(Step::CarryToVan));
    assert_eq!(primary(&v), Some(TargetKind::Van));
    let mate = view(&g, 2);
    assert_eq!(mate.step, Some(Step::CoverCarrier));
    assert_eq!(primary(&mate), Some(TargetKind::Carrier));
    g.bags = vec![bags::Bag {
        at: (0., 2.),
        carrier: None,
    }];
    assert_eq!(view(&g, 2).step, Some(Step::GrabBag));
}

#[test]
fn police_patrol_respond_and_chase_what_they_saw() {
    let mut g = game();
    put(&mut g, 0, (16., 16.));
    let v = view(&g, 0);
    assert_eq!(v.step, Some(Step::Patrol));
    assert_eq!(primary(&v), Some(TargetKind::Car));
    g.alarm = true;
    assert_eq!(view(&g, 0).step, Some(Step::RespondAlarm));
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: (0., 0.),
        carrier: Some(1),
    }];
    put(&mut g, 1, (16., 10.));
    let v = view(&g, 0);
    assert_eq!(v.step, Some(Step::ChaseBag));
    assert_eq!(primary(&v), Some(TargetKind::Carrier));
    g.actors[0].last_seen = Some(((16., 10.), g.time, 0., true));
    put(&mut g, 1, (60., 60.));
    let v = view(&g, 0);
    assert_eq!(
        primary(&v),
        Some(TargetKind::LastSeen),
        "a ghost where the bag was seen"
    );
    assert!(
        v.targets.iter().all(|t| (t.x, t.z) != (60., 60.)),
        "never the real spot"
    );
    g.time += GHOST_SECONDS + 0.1;
    assert_eq!(
        view(&g, 0).step,
        Some(Step::RespondAlarm),
        "the ghost fades"
    );
}

#[test]
fn the_crew_never_learns_police_targets_and_police_never_see_crew_jobs() {
    let mut g = game();
    put(&mut g, 0, (16., 16.));
    put(&mut g, 1, (-60., -60.));
    let police = view(&g, 0);
    assert!(police.targets.iter().all(|t| !matches!(
        t.kind,
        TargetKind::Power | TargetKind::Vault | TargetKind::Van
    )));
    let crew = view(&g, 1);
    assert!(crew.targets.iter().all(|t| !matches!(
        t.kind,
        TargetKind::Car | TargetKind::Patrol | TargetKind::Alarm
    )));
}

#[test]
fn the_context_is_the_action_a_held_button_would_take() {
    let mut g = game();
    put(&mut g, 1, POWER);
    let c = view(&g, 1).context.unwrap();
    assert_eq!((c.verb, c.button), (Verb::Work, "a"));
    put(&mut g, 1, (POWER.0, POWER.1 - 6.));
    assert_eq!(view(&g, 1).context, None, "outside the room, nothing to do");
    put(&mut g, 0, (16., 4.));
    put(&mut g, 1, (16.8, 4.));
    g.actors[1].stun = 2.;
    let c = view(&g, 0).context.unwrap();
    assert_eq!((c.verb, c.button), (Verb::Grab, "a"));
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(view(&g, 0).context.unwrap().verb, Verb::Trace);
    assert_eq!(view(&g, 1).context.unwrap().verb, Verb::Struggle);
    assert_eq!(view(&g, 0).step, Some(Step::Cuff));
    assert_eq!(view(&g, 1).step, Some(Step::Struggle));
}

#[test]
fn an_officer_by_the_car_is_offered_the_car() {
    let mut g = game();
    let car = g.actors[0].car_at;
    put(&mut g, 0, (car.0 + 1., car.1));
    let c = view(&g, 0).context.unwrap();
    assert_eq!((c.verb, c.button), (Verb::EnterCar, "x"));
}

#[test]
fn big_moments_stamp_once_and_fade() {
    let mut g = game();
    put(&mut g, 1, POWER);
    put(&mut g, 0, (60., -60.));
    g.control(1, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    g.qa_pass_check(1);
    let stamps = view(&g, 2).stamps;
    assert_eq!(stamps.iter().filter(|s| s.key == "power_cut").count(), 1);
    assert!(
        view(&g, 0).stamps.iter().all(|s| s.key != "power_cut"),
        "police are not told"
    );
    for _ in 0..(STAMP_SECONDS * 60.) as usize + 2 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert!(view(&g, 2).stamps.iter().all(|s| s.key != "power_cut"));
}

#[test]
fn no_server_text_names_a_key() {
    let mut g = game();
    for seat in [0, 1] {
        let v = view(&g, seat);
        for text in std::iter::once(v.objective).chain(v.targets.iter().map(|t| t.label)) {
            for key in [" E ", " F ", "hold E", "HOLD E", "press E"] {
                assert!(!text.contains(key), "{text}");
            }
        }
    }
    g.job = Job::Carry;
    let v = view(&g, 1);
    assert!(v.targets.iter().all(|t| !t.label.contains("HOLD")));
}

#[test]
fn a_radio_ping_shows_on_every_officers_compass_but_never_the_crews() {
    let mut g = CityGame::new(
        &[(0, false), (1, false), (2, false), (3, false)],
        CityConfig {
            police: 2,
            jobs: 1,
            host_police: true,
            ..Default::default()
        },
        false,
    );
    g.briefing_left = 0.;
    g.time = 3.;
    put(&mut g, 0, (0., 0.));
    put(&mut g, 1, (20., 0.));
    put(&mut g, 2, (0., 14.));
    put(&mut g, 3, (60., 60.));
    assert!(g.use_tool(0, Tool::Scanner));
    let pings = |v: &CityView| {
        v.targets
            .iter()
            .filter(|t| t.kind == TargetKind::Ping)
            .count()
    };
    assert_eq!(pings(&view(&g, 0)), 1);
    assert_eq!(pings(&view(&g, 1)), 1, "the other officer sees it too");
    assert_eq!(pings(&view(&g, 2)), 0);
    let ping = view(&g, 1)
        .targets
        .into_iter()
        .find(|t| t.kind == TargetKind::Ping)
        .unwrap();
    assert!(
        distance((ping.x, ping.z), (0., 14.)) < 6.,
        "coarse, near the robber: {ping:?}"
    );
    g.time += 5.1;
    assert_eq!(pings(&view(&g, 1)), 0, "and fades after five seconds");
}
