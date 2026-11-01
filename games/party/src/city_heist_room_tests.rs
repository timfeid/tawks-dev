use super::*;
fn room() -> Room {
    let mut room = Room::new("city-room", 15);
    room.connect("host", "Host", "").unwrap();
    room
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
    assert_eq!(r.view().bouts.len(),4);assert_eq!(r.view().maps.len(),1);
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
    let browser=room();assert!(browser.view().bouts.len()>4);assert_eq!(browser.view().maps.len(),4);
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
