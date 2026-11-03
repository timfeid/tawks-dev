use super::*;
fn room() -> Room {
    let mut room = Room::new("city-room", 15);
    room.connect("host", "Host", "").unwrap();
    room
}
#[test]
fn city_bots_use_the_lobby_skill_choice() {
    for (choice, expected) in [
        ("relaxed", 0.15),
        ("steady", 0.4),
        ("sharp", 0.7),
        ("ruthless", 1.0),
    ] {
        let mut r = room();
        r.choose_skill("host", choice).unwrap();
        r.choose_city("host", 1, 3, false).unwrap();
        r.start_city("host", false).unwrap();
        assert_eq!(r.city.as_ref().unwrap().config.skill, Some(expected));
    }
}
#[test]
fn city_is_an_independent_selection_and_solo_starts_with_three_bots() {
    let mut r = room();
    assert_eq!(
        r.choose_city("stranger", 1, 3, false),
        Err(RoomError::NotHost)
    );
    r.choose_city("host", 1, 3, false).unwrap();
    assert!(r.view().city_mode);
    assert!(!r.view().party_mode);
    assert!(r.view().preview_board.is_some());
    r.start_city("host", false).unwrap();
    assert_eq!(r.phase, Phase::CityHeist);
    assert_eq!(r.members.len(), 4);
    assert_eq!(r.members.iter().filter(|m| m.is_bot).count(), 3);
    let view = r.view_for(Some(0));
    assert_eq!(
        view.city
            .unwrap()
            .actors
            .iter()
            .find(|a| a.seat == 0)
            .unwrap()
            .side,
        crate::city_heist::Side::Crew
    );
    assert_eq!(
        r.choose_city("host", 2, 3, true),
        Err(RoomError::WrongPhase)
    );
}
#[test]
fn host_can_test_police_and_config_cannot_remove_the_robber_side() {
    let mut r = room();
    r.add_bot("host").unwrap();
    r.choose_city("host", 1, 1, true).unwrap();
    r.start_city("host", false).unwrap();
    assert_eq!(
        r.view_for(Some(0)).city.unwrap().actors[0].side,
        crate::city_heist::Side::Police
    );
    let mut r = room();
    r.add_bot("host").unwrap();
    r.choose_city("host", 2, 1, true).unwrap();
    assert_eq!(
        r.start_city("host", false),
        Err(RoomError::NotEnoughPlayers)
    );
}
#[test]
fn city_to_minigame_to_next_job_and_final_results_work_with_all_ai() {
    let mut r = room();
    r.choose_city("host", 1, 3, false).unwrap();
    r.start_city("host", true).unwrap();
    let mut saw_vote = false;
    let mut saw_playing = false;
    let mut saw_second = false;
    for _ in 0..60 * 1800 {
        r.step();
        if let Some(city) = r.city.as_ref() {
            if city.resolved.is_some() && city.resolved_for == 0. {
                assert_ne!(
                    city.view_for(Some(0)).reason,
                    Some("The crew ran out of time"),
                    "all AI should resolve the heist through play"
                );
            }
        }
        saw_vote |= matches!(r.phase, Phase::Vote { .. });
        saw_playing |= r.phase == Phase::Playing;
        saw_second |= r.phase == Phase::CityHeist && r.city.as_ref().is_some_and(|c| c.round == 2);
        if matches!(r.phase, Phase::GameOver { .. }) {
            break;
        }
    }
    assert!(
        saw_vote && saw_playing && saw_second,
        "all AI must complete both game phases"
    );
    assert!(matches!(r.phase, Phase::GameOver { .. }));
    let city = r.city.as_ref().unwrap();
    assert!(city.scores.iter().sum::<u8>() <= 3);
    for _ in 0..GAME_OVER_TICKS + 1 {
        r.step();
    }
    assert_eq!(r.phase, Phase::Lobby);
    assert!(r.city.is_none());
    assert!(r.city_mode);
    r.choose_mode("host", true).unwrap();
    assert!(!r.city_mode && r.party_mode);
    r.start_party("host").unwrap();
    assert!(
        !r.city_mode,
        "the original board game still starts independently"
    );
}
#[test]
fn starting_a_demo_cannot_take_control_of_another_human() {
    let mut r = room();
    r.connect("friend", "Friend", "").unwrap();
    r.choose_city("host", 1, 1, false).unwrap();
    assert_eq!(r.start_city("host", true), Err(RoomError::WrongPhase));
}

#[test]
fn city_votes_do_not_accept_tawkathon_coin_bets(){
 let mut r=room();r.choose_city("host",1,1,false).unwrap();r.start_city("host",false).unwrap();r.open_vote();
 assert!(matches!(r.phase,Phase::Vote{..}));assert_eq!(r.bet("host",2),Err(RoomError::WrongPhase));
}

#[test]
fn native_rooms_only_vote_for_games_the_native_client_can_render() {
    let mut r=room();r.enable_native_mode();
    assert_eq!(r.view().bouts.len(),NATIVE_BOUTS.len());assert_eq!(r.view().maps.len(),1);
    assert!(r.view().table_games.is_empty());
    assert_eq!(r.choose_game("host","dodgeball"),Err(RoomError::NativeUnsupported));
    assert_eq!(r.choose_map("host",Map::Volcano),Err(RoomError::NativeUnsupported));
    r.choose_city("host",1,1,false).unwrap();r.start_city("host",true).unwrap();
    for _ in 0..60*365{r.step();}
    for _ in 0..50 {
        r.open_vote();
        let Phase::Vote{options,..}=&r.phase else {panic!("expected native ballot")};
        assert!(!options.is_empty());assert!(options.iter().all(|key|NATIVE_BOUTS.contains(key)));
    }
    let browser=room();assert!(browser.view().bouts.len()>NATIVE_BOUTS.len());assert_eq!(browser.view().maps.len(),4);
}

#[test]
fn native_board_practice_opens_a_real_interactive_host_turn() {
    let mut r=room();for _ in 0..3{r.add_bot("host").unwrap();}
    assert_eq!(r.start_native_board_practice("stranger"),Err(RoomError::NotHost));
    r.start_native_board_practice("host").unwrap();assert_eq!(r.phase,Phase::Board);
    assert_eq!(r.map,Map::Frozen);assert!(!r.opening_minigame);
    assert_eq!(r.board.as_ref().unwrap().turn_seat(),r.seat_of("host").unwrap());
    // Frozen opens with the normal opt-in candy interruption. Practice must
    // preserve it and accept the host's board controls once the offer expires.
    assert!(r.board_paused());
    r.finish_candy_offer_for_test();
    assert_eq!(r.pass_items("host"),Ok(()));
    assert_eq!(r.roll("host"),Ok(()));
}

/// Plays a whole arrest through the room's message handlers, as two
/// connected clients would: ram, get out, grab, trace, drag to the car.
fn city_actor(r: &Room, viewer: u8, seat: u8) -> crate::city_heist::CityActorView {
    r.view_for(Some(viewer)).city.unwrap().actors.into_iter().find(|a| a.seat == seat).unwrap()
}
fn cuffed_room() -> (Room, u8, u8) {
    use crate::bout::Stick;
    let mut r = room();
    r.connect("friend", "Friend", "").unwrap();
    r.choose_city("host", 1, 1, true).unwrap();
    r.start_city("host", false).unwrap();
    r.city_ready("host").unwrap();
    r.city_ready("friend").unwrap();
    let cop = r.seat_of("host").unwrap();
    let robber = r.seat_of("friend").unwrap();
    for _ in 0..200 {
        r.step();
    }
    let city = r.city.as_mut().unwrap();
    // A stretch of street between traffic loops, so only the arrest matters.
    city.place(cop, (16., -36.), true, 0.);
    city.place(robber, (16., -24.), false, 0.);
    let actor = |r: &Room, seat: u8| city_actor(r, cop, seat);
    let mut rammed = false;
    for _ in 0..120 {
        r.set_input("host", Stick { x: 0., z: 1. }, 0., 0., false, false);
        r.step();
        if actor(&r, robber).stunned > 0. {
            rammed = true;
            break;
        }
    }
    assert!(rammed, "driving into the robber rams them");
    assert!(!actor(&r, cop).driving, "a ram puts the officer on foot");
    let mut grabbed = false;
    for _ in 0..240 {
        let (me, them) = (actor(&r, cop), actor(&r, robber));
        let (dx, dz) = (them.x.unwrap() - me.x.unwrap(), them.z.unwrap() - me.z.unwrap());
        let d = dx.hypot(dz).max(0.01);
        let stick = if d > 1.2 { Stick { x: dx / d, z: dz / d } } else { Stick::default() };
        r.set_input("host", stick, 0., 0., false, false);
        r.city_action("host", true, false).unwrap();
        r.step();
        if !r.view_for(Some(cop)).city.unwrap().arrests.is_empty() {
            grabbed = true;
            break;
        }
    }
    assert!(grabbed, "holding E beside the dazed robber grabs them");
    let shared = serde_json::to_value(r.view_for(Some(robber))).unwrap();
    assert_eq!(shared["city"]["arrests"][0]["stage"], "cuffing");
    assert_eq!(shared["city"]["slow_left"].as_f64().unwrap(), 0., "cuffing does not trigger Clarity");
    for _ in 0..60 * 5 {
        let view = r.view_for(Some(cop)).city.unwrap();
        let Some(arrest) = view.arrests.first() else { break };
        if arrest.stage == crate::city_arrest::Stage::Escort {
            break;
        }
        let (x, y) = crate::city_arrest::figure_eight(arrest.trace + std::f32::consts::TAU / 2.5 / 60.);
        r.city_trace("host", x, y).unwrap();
        r.step();
    }
    let view = r.view_for(Some(cop)).city.unwrap();
    assert_eq!(view.arrests[0].stage, crate::city_arrest::Stage::Escort);
    (r, cop, robber)
}
#[test]
fn a_scripted_officer_rams_grabs_traces_and_drags_a_human_robber_to_the_car() {
    use crate::bout::Stick;
    let (mut r, cop, robber) = cuffed_room();
    let actor = |r: &Room, seat: u8| city_actor(r, cop, seat);
    let car = actor(&r, cop).car.unwrap();
    for _ in 0..60 * 20 {
        let me = actor(&r, cop);
        let (dx, dz) = (car[0] - me.x.unwrap(), car[1] - me.z.unwrap());
        let d = dx.hypot(dz).max(0.01);
        r.set_input("host", Stick { x: dx / d, z: dz / d }, 0., 0., false, false);
        r.step();
        if actor(&r, robber).jailed {
            break;
        }
    }
    assert!(actor(&r, robber).jailed, "dragging the robber to the car jails them");
}
#[test]
fn a_human_robber_slipping_the_cuffs_breaks_free_through_the_room() {
    use crate::city_slip::SlipInput;
    let (mut r, cop, robber) = cuffed_room();
    let mut answered = -1.;
    for _ in 0..60 * 12 {
        let view = r.view_for(Some(robber)).city.unwrap();
        let Some(arrest) = view.arrests.first() else { break };
        if let Some(cuffs) = arrest.cuffs.as_ref() {
            r.city_slip("friend", SlipInput::Aim { angle: cuffs.target }).unwrap();
            if let Some(cue) = cuffs.cue.as_ref().filter(|c| !c.fake) {
                let shown = cuffs.clock - (crate::city_slip::CUE_SECONDS - cue.left);
                if cuffs.clock - shown > 0.2 && (shown - answered).abs() > 0.01 {
                    answered = shown;
                    r.city_slip("friend", SlipInput::Flick { dir: -cue.dir }).unwrap();
                }
            }
        }
        r.step();
    }
    assert!(r.view_for(Some(robber)).city.unwrap().arrests.is_empty());
    assert!(!city_actor(&r, cop, robber).jailed, "four clean seconds slip the cuffs");
    assert!(city_actor(&r, cop, cop).stunned > 0., "the officer staggers");
}

fn sandbox_host() -> Room {
    let mut r = room();
    r.dev_tools = true;
    r
}
fn city_tool(r: &mut Room, user: &str, tool: crate::city_heist::CityTool) -> Result<(), RoomError> {
    r.dev(user, DevAction::City { tool })
}

#[test]
fn sandbox_tools_never_reach_a_real_match() {
    use crate::city_heist::CityTool;
    // A production room has no dev tools.
    let mut r = room();
    assert_eq!(r.start_city_sandbox("host", true), Err(RoomError::NoDevTools));
    assert_eq!(r.start_sandbox("host"), Err(RoomError::NoDevTools));
    r.choose_city("host", 1, 3, true).unwrap();
    r.start_city("host", false).unwrap();
    assert_eq!(city_tool(&mut r, "host", CityTool::Arrest), Err(RoomError::NoDevTools));
    assert_eq!(city_tool(&mut r, "host", CityTool::Alarm { on: true }), Err(RoomError::NoDevTools));
    assert_eq!(r.dev("host", DevAction::Bout { key: "titan".into() }), Err(RoomError::NoDevTools));
    assert_eq!(r.dev("host", DevAction::BoardSandbox), Err(RoomError::NoDevTools));
    // Even with dev tools on, an ordinary night is not a sandbox.
    let mut r = sandbox_host();
    r.choose_city("host", 1, 3, true).unwrap();
    r.start_city("host", false).unwrap();
    assert!(!r.sandbox);
    assert_eq!(city_tool(&mut r, "host", CityTool::Arrest), Err(RoomError::NoDevTools));
    // Only the host switches a sandbox about.
    let mut r = sandbox_host();
    r.connect("friend", "Friend", "").unwrap();
    r.start_city_sandbox("host", true).unwrap();
    assert_eq!(r.dev("friend", DevAction::CitySandbox { police: false }), Err(RoomError::NotHost));
    assert_eq!(r.dev("friend", DevAction::Bout { key: "titan".into() }), Err(RoomError::NotHost));
}

#[test]
fn the_city_sandbox_has_no_clock_and_its_tools_work() {
    use crate::city_heist::{Behavior, CityTool, StationKind};
    let mut r = sandbox_host();
    r.start_city_sandbox("host", true).unwrap();
    assert!(r.view().sandbox && r.view().dev_tools);
    r.city.as_mut().unwrap().time = crate::city_heist::NIGHT_SECONDS + 10.;
    for _ in 0..60 * 2 {
        r.step();
    }
    let city = r.view_for(Some(0)).city.unwrap();
    assert!(city.resolved.is_none(), "a sandbox night never ends");
    assert_eq!(city.remaining, crate::city_heist::NIGHT_SECONDS, "and shows no countdown");
    assert!(city.briefing.is_none());

    city_tool(&mut r, "host", CityTool::Spawn { police: false, behavior: Behavior::Still }).unwrap();
    let spawned = r.city.as_ref().unwrap().sandbox_spawned();
    assert_eq!(spawned.len(), 1);
    assert!(r.members.iter().any(|m| m.seat == spawned[0] && m.is_bot), "the dummy has a name");
    city_tool(&mut r, "host", CityTool::Arrest).unwrap();
    let city = r.view_for(Some(0)).city.unwrap();
    assert!(
        city.arrests.iter().any(|a| a.officer == 0 && a.robber == spawned[0]),
        "an arrest starts on the dummy at once"
    );
    for _ in 0..60 * 3 {
        r.step();
    }
    assert!(!r.view_for(Some(0)).city.unwrap().arrests.is_empty(), "a still dummy does not struggle free");

    city_tool(&mut r, "host", CityTool::Alarm { on: true }).unwrap();
    assert!(r.view_for(Some(0)).city.unwrap().alarm);
    city_tool(&mut r, "host", CityTool::Alarm { on: false }).unwrap();
    assert!(!r.view_for(Some(0)).city.unwrap().alarm);

    city_tool(&mut r, "host", CityTool::Traffic { level: 0 }).unwrap();
    assert!(r.board.as_ref().unwrap().traffic_view().unwrap().cars.is_empty());
    city_tool(&mut r, "host", CityTool::Traffic { level: 2 }).unwrap();
    assert!(r.board.as_ref().unwrap().traffic_view().unwrap().cars.len() > 30);

    city_tool(&mut r, "host", CityTool::Station { station: StationKind::Power }).unwrap();
    r.step();
    let city = r.view_for(Some(0)).city.unwrap();
    assert_eq!(city.actors.iter().find(|a| a.seat == 0).unwrap().side, crate::city_heist::Side::Crew);
    assert_eq!(
        city.context.map(|c| c.verb),
        Some(crate::city_heist::Verb::Work),
        "the power switch is ready to work"
    );

    city_tool(&mut r, "host", CityTool::Role { police: true }).unwrap();
    assert_eq!(
        r.view_for(Some(0)).city.unwrap().actors.iter().find(|a| a.seat == 0).unwrap().side,
        crate::city_heist::Side::Police
    );
    city_tool(&mut r, "host", CityTool::Despawn).unwrap();
    assert!(r.city.as_ref().unwrap().sandbox_spawned().is_empty());
    assert!(!r.members.iter().any(|m| m.seat == spawned[0]));

    city_tool(&mut r, "host", CityTool::Spawn { police: true, behavior: Behavior::Bot }).unwrap();
    assert_eq!(r.city.as_ref().unwrap().sandbox_spawned().len(), 1, "a spawned bot counts too");
    city_tool(&mut r, "host", CityTool::Despawn).unwrap();
    assert!(r.city.as_ref().unwrap().sandbox_spawned().is_empty(), "and can be removed");
}

#[test]
fn a_fighting_officer_dummy_comes_for_you() {
    use crate::city_heist::{Behavior, CityTool};
    let mut r = sandbox_host();
    r.start_city_sandbox("host", false).unwrap();
    city_tool(&mut r, "host", CityTool::Spawn { police: true, behavior: Behavior::Fight }).unwrap();
    let mut grabbed = false;
    for _ in 0..60 * 10 {
        r.step();
        if r.view_for(Some(0)).city.unwrap().arrests.iter().any(|a| a.robber == 0) {
            grabbed = true;
            break;
        }
    }
    assert!(grabbed, "the fighting officer grabs you");
}

#[test]
fn a_sandbox_switches_to_the_board_and_to_any_bout_without_a_ballot() {
    let mut r = sandbox_host();
    r.enable_native_mode();
    r.start_city_sandbox("host", true).unwrap();
    r.dev("host", DevAction::BoardSandbox).unwrap();
    assert_eq!(r.phase, Phase::Board);
    assert!(r.sandbox);
    r.dev("host", DevAction::Bout { key: "titan".into() }).unwrap();
    assert!(r.sandbox);
    assert!(!matches!(r.phase, Phase::Vote { .. } | Phase::Lobby));
    assert_eq!(r.snapshot().unwrap()["game"], "titan");
    assert_eq!(r.dev("host", DevAction::Bout { key: "dodgeball".into() }), Err(RoomError::NativeUnsupported), "native rooms stay native");
    r.dev("host", DevAction::CitySandbox { police: false }).unwrap();
    assert_eq!(r.phase, Phase::CityHeist);
    assert!(r.sandbox);
}


#[test]
fn a_reset_car_stays_reset_when_the_driver_lets_go() {
    let mut r = Room::new("native-practice", 73419);
    r.enable_native_mode();
    r.connect("you", "You", "").unwrap();
    for _ in 0..3 {
        let _ = r.add_bot("you");
    }
    r.choose_city("you", 1, 3, true).unwrap();
    r.start_city("you", false).unwrap();
    r.city_ready("you").unwrap();
    let seat = r.members.iter().find(|m| m.user_id == "you").unwrap().seat;
    for _ in 0..60 * 8 {
        r.set_input("you", Stick::default(), 0., 0., false, false);
        r.step();
    }
    r.city.as_mut().unwrap().qa_place_car(seat, (17.5, 22.), std::f32::consts::FRAC_PI_2);
    for _ in 0..60 * 4 {
        r.set_input("you", Stick { x: 1., z: 0. }, std::f32::consts::FRAC_PI_2, 0., false, false);
        r.step();
    }
    let me = |r: &Room| {
        let v = r.city.as_ref().unwrap().view_for(Some(seat));
        let a = v.actors.iter().find(|a| a.seat == seat).unwrap().clone();
        ((a.x.unwrap(), a.z.unwrap()), v.car_stuck)
    };
    assert!(me(&r).1, "stuck");
    r.city_reset_car("you").unwrap();
    let after = me(&r).0;
    for _ in 0..60 {
        r.set_input("you", Stick::default(), std::f32::consts::FRAC_PI_2, 0., false, true);
        r.step();
    }
    assert_eq!(me(&r).0, after, "the reset car stays put");
}

#[test]
fn the_mature_setting_never_changes_how_a_night_plays() {
    let night = |mature: bool| {
        let mut r = room();
        r.choose_mature("host", mature).unwrap();
        r.choose_city("host", 1, 3, false).unwrap();
        r.start_city("host", true).unwrap();
        for _ in 0..60 * 90 {
            r.step();
        }
        let city = r.city.as_ref().unwrap();
        let view = city.view_for(Some(0));
        (
            view.night_score,
            view.actors.iter().map(|a| (a.seat, a.x, a.z)).collect::<Vec<_>>(),
            view.impacts.iter().map(|i| (i.kind, i.victim)).collect::<Vec<_>>(),
        )
    };
    assert_eq!(night(true), night(false));
}
#[test]
fn lobby_size_picks_the_cop_count_and_the_host_can_override_it() {
    let police_in = |r: &Room| {
        r.view_for(Some(0))
            .city
            .unwrap()
            .actors
            .iter()
            .filter(|a| a.side == crate::city_heist::Side::Police)
            .count()
    };
    let mut r = room();
    for _ in 0..5 {
        r.add_bot("host").unwrap();
    }
    r.choose_city("host", 0, 3, false).unwrap();
    r.start_city("host", false).unwrap();
    assert_eq!(police_in(&r), 2, "six players default to 2v4");

    let mut r = room();
    for _ in 0..3 {
        r.add_bot("host").unwrap();
    }
    r.choose_city("host", 2, 3, false).unwrap();
    r.start_city("host", false).unwrap();
    assert_eq!(police_in(&r), 2, "four players may play 2v2");

    let mut r = room();
    for _ in 0..2 {
        r.add_bot("host").unwrap();
    }
    r.choose_city("host", 2, 3, false).unwrap();
    assert_eq!(
        r.start_city("host", false),
        Err(RoomError::NotEnoughPlayers),
        "2v1 is not a format"
    );
    assert_eq!(r.choose_city("host", 3, 3, false), Err(RoomError::InvalidRounds));
}
