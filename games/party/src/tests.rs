use serde_json::Value;

use crate::auth::{self, Claims};
use crate::board::{Taunt, 
    self, Board, COMPASS_SHORT, CURSED_SIDES, GRAPPLE_COINS, KNOCKOUT_COINS, MAX_ITEMS, BoardError, BotMove, Bonus, Highlight, RoundEvent, CANNON_RANGE, CANNON_SCATTER, MAX_BET, MINIGAME_COINS, STORM_COINS, STORM_STEP, REVENGE_COINS, TRAP_RUMOUR, SCRAP_COINS, UNDERDOG_PRIZE, SHOP_COST, SHOP_OFFERS, HAZARD_COINS, Item, MAX_HEALTH, PRIZE_COST,
    START_COINS, SpaceKind, TurnStep, board, distance, gap, prize_spots,
};
use crate::dodgeball::{
    BallState as DodgeBall, COURT_HALF_Z, Dodgeball, OUT_SECONDS, ROUND_SECONDS as BALL_SECONDS,
};
use crate::bout::{BOUTS, Bout, Contestant, Input, Press, Stick, create};
use crate::paint_variant::{PaintPlaylist, PaintVariant};
use crate::paint_variant::{RETAKE_BOMB, RETAKE_DEFUSE, RETAKE_FUSE};
use crate::race::{BOOSTS, FINISH_Z, ROUND_SECONDS as RACE_SECONDS, RockerRally, START_Z};
use crate::tug::{PIT_HALF as TUG_PIT, ROUND_SECONDS as TUG_SECONDS, TugOfWar};
use crate::memory::{
    FIRST_LENGTH as MEMORY_FIRST, MemoryTiles, Phase as MemoryPhase, TILES as MEMORY_TILES,
    recall_seconds as memory_recall_seconds, tile_at as memory_tile_at, tile_centre as memory_tile_centre,
};
use crate::redlight::RedLightRush;
use crate::room::{Phase, Room, RoomError, TICK_DT};
use crate::shooter::{
    ARENA_HALF, FIRE_COOLDOWN, MAX_HP, PaintBlasters, RESPAWN_SECONDS,
    ROUND_SECONDS as BLAST_SECONDS, SHOT_DAMAGE,
};

fn contestants(n: u8, bots: bool) -> Vec<Contestant> {
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

#[test]
fn a_room_runs_countdown_play_results_and_back_to_the_lobby() {
    let mut room = Room::new("lobby-1", 5);
    room.connect("alice", "Alice", "").unwrap();
    assert_eq!(room.start("alice"), Err(RoomError::NotEnoughPlayers));
    room.connect("bob", "Bob", "").unwrap();
    assert_eq!(room.add_bot("bob"), Err(RoomError::NotHost));
    room.add_bot("alice").unwrap();
    room.start("alice").unwrap();
    assert!(matches!(room.phase, Phase::Practice { .. }), "a practice go first");

    let mut saw_playing = false;
    let mut saw_results = false;
    for _ in 0..(60 * 90) {
        room.step();
        saw_playing |= room.phase == Phase::Playing;
        saw_results |= matches!(room.phase, Phase::Results { .. });
        if saw_results && room.phase == Phase::Lobby {
            break;
        }
    }
    assert!(saw_playing && saw_results);
    assert_eq!(room.phase, Phase::Lobby);
    assert!(
        room.members.iter().map(|m| m.wins).sum::<u32>() >= 1,
        "someone won"
    );
}

#[test]
fn people_take_bot_seats_and_characters_stay_unique() {
    let mut room = Room::new("lobby-2", 6);
    room.connect("host", "Host", "").unwrap();
    for _ in 1..crate::room::MAX_SEATS {
        room.add_bot("host").unwrap();
    }
    assert_eq!(room.add_bot("host"), Err(RoomError::Full));
    room.connect("friend", "Friend", "").unwrap();
    assert_eq!(
        room.members.iter().filter(|m| m.is_bot).count(),
        crate::room::MAX_SEATS - 2,
        "a bot made room for a person"
    );
    let taken = room
        .members
        .iter()
        .find(|m| m.user_id != "friend")
        .unwrap()
        .character
        .clone();
    assert_eq!(room.pick("friend", &taken), Err(RoomError::Taken));
    assert_eq!(
        room.pick("friend", "nobody"),
        Err(RoomError::UnknownCharacter)
    );
}

#[test]
fn the_host_passes_on_when_they_leave() {
    let mut room = Room::new("lobby-3", 7);
    room.connect("a", "A", "").unwrap();
    room.connect("b", "B", "").unwrap();
    room.disconnect("a");
    assert_eq!(room.host_id.as_deref(), Some("a"), "a short drop keeps the host");
    for _ in 0..(60 * 21) {
        room.step();
    }
    assert_eq!(room.host_id.as_deref(), Some("b"));
}

#[test]
fn the_lobby_owner_hosts_whenever_they_are_in_the_room() {
    let mut room = Room::new("lobby-owner", 7);
    room.connect("guest", "Guest", "").unwrap();
    assert_eq!(room.host_id.as_deref(), Some("guest"), "first in hosts until the owner arrives");
    room.connect("owner", "Owner", "").unwrap();
    room.set_owner("owner");
    assert_eq!(room.host_id.as_deref(), Some("owner"), "the owner takes it on arrival");
    room.disconnect("owner");
    for _ in 0..(60 * 21) {
        room.step();
    }
    assert_eq!(room.host_id.as_deref(), Some("guest"), "it passes on while the owner is gone");
    room.connect("owner", "Owner", "").unwrap();
    assert_eq!(room.host_id.as_deref(), Some("owner"), "and comes back with them");
}

#[test]
fn a_host_who_reconnects_in_time_stays_host() {
    let mut room = Room::new("lobby-3b", 7);
    room.connect("a", "A", "").unwrap();
    room.connect("b", "B", "").unwrap();
    room.choose_game("a", crate::room::COVEN_KEY).unwrap();
    room.add_bot("a").unwrap();
    room.add_bot("a").unwrap();
    room.start("a").unwrap();
    assert_eq!(room.phase, Phase::Table);
    room.disconnect("a");
    for _ in 0..(60 * 5) {
        room.step();
    }
    room.connect("a", "A", "").unwrap();
    assert_eq!(room.host_id.as_deref(), Some("a"));
    room.disconnect("a");
    for _ in 0..(60 * 21) {
        room.step();
    }
    assert_eq!(room.host_id.as_deref(), Some("b"), "a host gone for good passes it on");
}

#[test]
fn join_passes_are_signed_and_expire() {
    let secret = [7u8; 32];
    let claims = Claims {
        game: None,
        match_id: "m".into(),
        user_id: "u".into(),
        name: "U".into(),
        avatar_url: String::new(),
        exp: 1_000,
        spectator: false,
        guest: false,
        drives: Vec::new(),
        owner: false,
    };
    let token = auth::sign(&claims, &secret);
    assert_eq!(auth::verify(&token, &secret, 999), Some(claims));
    assert_eq!(auth::verify(&token, &secret, 1_000), None, "expired");
    assert_eq!(auth::verify(&token, &[8u8; 32], 999), None, "wrong secret");
    let tampered = token.replacen('e', "f", 1);
    assert_eq!(auth::verify(&tampered, &secret, 999), None, "tampered");
}

fn seats(n: u8) -> Vec<u8> {
    (0..n).collect()
}

/// Play one bot turn at a time until the round hands over to a minigame.
/// Uses an item that has a skill check, and stops its needle dead centre.
pub(crate) fn use_now(board: &mut Board, seat: u8, item: Item, number: Option<u8>) {
    board.use_item_with(seat, item, None, number).unwrap();
    if matches!(board.step(), TurnStep::Check { .. }) {
        board.check(seat, 0.5).unwrap();
    }
}

pub(crate) fn run_board_round(board: &mut Board) {
    for _ in 0..20000 {
        match board.bot_move() {
            BotMove::Walk => {
                if matches!(board.step(),TurnStep::Raiding) {let eligible:Vec<_>=board.view().tokens.iter().map(|token|token.seat).collect();let owner=board.turn_seat();board.bot_raids_tick(&[owner],&eligible);board.raids_tick(TICK_DT,&eligible);}
                else {board.bot_walk_aim(1.0);board.walk_tick(TICK_DT);}
            }
            BotMove::Item {
                item,
                target,
                number,
            } => {
                let seat = board.turn_seat();
                let _ = board.use_item_with(seat, item, target, number);
            }
            BotMove::Pass => {
                let seat = board.turn_seat();
                board.pass_items(seat).unwrap();
            }
            BotMove::Shop { item, upgrade } => {
                let seat = board.turn_seat();
                board.shop_with(seat, item, upgrade).unwrap();
            }
            BotMove::Fire { power } => {
                let seat = board.turn_seat();
                board.fire(seat, power).unwrap();
            }
            BotMove::Lob { yaw, pitch, power } => {
                let seat = board.turn_seat();
                board.lob(seat, yaw, pitch, power).unwrap();
            }
            BotMove::Swing { yaw, stop } => {
                let seat = board.turn_seat();
                board.swing(seat, yaw, stop).unwrap();
            }
            BotMove::Check { stop } => {
                let seat = board.turn_seat();
                board.check(seat, stop).unwrap();
            }
            BotMove::Suck => {
                let seat = board.turn_seat();
                board.suck(seat).unwrap();
            }
            BotMove::Vacuum { yaw, on, blow } => {
                let seat = board.turn_seat();
                board.aim_vacuum_with(seat, yaw, on, blow).unwrap();
                for _ in 0..6 {
                    board.vacuum_tick(1.0 / 60.0);
                }
            }
            BotMove::StowVacuum => {
                let seat = board.turn_seat();
                board.stow_vacuum(seat).unwrap();
            }
            BotMove::ThrowCrate => {
                let seat = board.turn_seat();
                board.throw_crate(seat).unwrap();
            }
            BotMove::ThrowSnowball => {
                let seat = board.turn_seat();
                board.throw_snowball(seat).unwrap();
            }
            BotMove::Roll => {
                let seat = board.turn_seat();
                board.roll(seat).unwrap();
            }
            BotMove::Branch { node } => {
                let seat = board.turn_seat();
                board.choose_branch(seat, node).unwrap();
            }
            BotMove::EndTurn => {
                if board.end_turn() {
                    return;
                }
            }
        }
    }
    panic!("the round never ended");
}

#[test]
fn the_board_is_a_loop_with_shortcuts_and_every_space_leads_on() {
    let spaces = board();
    assert!(spaces.len() >= 20);
    for space in spaces {
        assert!(!space.next.is_empty(), "space {} is a dead end", space.id);
        for next in &space.next {
            assert!((*next as usize) < spaces.len());
        }
    }
    assert!(
        spaces.iter().any(|s| s.next.len() > 1),
        "there is a junction"
    );
    assert!(spaces.iter().any(|s| s.kind == SpaceKind::Item));
    // Every space can be reached from the start.
    for space in spaces {
        assert!(
            distance(0, space.id) < u8::MAX,
            "space {} is cut off",
            space.id
        );
    }
}

#[test]
fn rolling_moves_you_and_the_space_pays_or_charges() {
    let mut board = Board::new(&seats(2), 3, 11);
    board.pass_items(0).unwrap();
    let before = board.view().tokens[0].clone();
    board.roll(0).unwrap();
    let after = board.view().tokens[0].clone();
    if matches!(board.step(), TurnStep::Branch { .. }) {
        return; // A junction asks first; covered by the full-game test.
    }
    assert_ne!(after.space, before.space, "the token moved");
    assert_eq!(*board.step(), TurnStep::Done);
    assert!(
        after.coins != before.coins || !after.items.is_empty(),
        "the space did something"
    );
}

#[test]
fn a_roll_logs_every_space_walked() {
    let mut board = Board::new(&seats(2), 3, 11);
    board.pass_items(0).unwrap();
    let roll = board.roll(0).unwrap();
    let log = board.view().log;
    let path = log
        .iter()
        .rev()
        .find(|entry| entry["kind"] == "move")
        .and_then(|entry| entry["path"].as_array())
        .expect("the walk is logged")
        .clone();
    if matches!(board.step(), TurnStep::Branch { .. }) {
        assert!(path.len() < roll as usize, "it stopped at the junction");
    } else {
        assert_eq!(path.len(), roll as usize);
    }
    assert_eq!(path.last().and_then(|v| v.as_u64()), Some(board.view().tokens[0].space as u64));
}

#[test]
fn a_finished_turn_hands_itself_on() {
    let mut room = Room::new("party-auto", 16);
    room.connect("alice", "Alice", "").unwrap();
    room.add_bot("alice").unwrap();
    room.start_party("alice").unwrap();
    room.finish_opening_for_test();
    let first = room.view().board.as_ref().unwrap().turn_seat;
    for _ in 0..(60 * 60) {
        room.step();
        if room.view().board.as_ref().map(|b| b.turn_seat) != Some(first) {
            return;
        }
        let _ = room.roll("alice");
    }
    panic!("alice never got past her turn without pressing anything");
}

#[test]
fn the_bat_knocks_a_rival_back_and_hurts_them() {
    let mut board = Board::new(&seats(2), 3, 12);
    // Walk seat 1 forward so there is room to be knocked back.
    for _ in 0..2 {
        board.pass_items(board.turn_seat()).unwrap();
        board.roll(board.turn_seat()).unwrap();
        while let TurnStep::Branch { options, .. } = board.step().clone() {
            let seat = board.turn_seat();
            board.choose_branch(seat, options[0].node).unwrap();
        }
        board.end_turn();
    }
    let target = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 1)
        .cloned()
        .unwrap();
    assert!(target.space > 0, "seat 1 has moved off the start");
    let hitter = board.turn_seat();
    board.give_for_test(hitter, Item::Bat);
    let here = token_of(&board, hitter).space;
    bat(&mut board, hitter, 1);
    let after = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 1)
        .cloned()
        .unwrap();
    assert!(after.space != here && crate::board::distance(after.space, here) <= crate::board::BAT_SPACES + 1, "knocked back");
    assert_eq!(after.health, MAX_HEALTH - 1, "and it hurt");
}

#[test]
fn a_grapple_moves_coins_from_them_to_you() {
    let mut board = Board::new(&seats(2), 3, 13);
    board.give_for_test(0, Item::Grapple);
    hit_with(&mut board, 0, Item::Grapple, 1);
    let tokens = board.view().tokens;
    assert_eq!(tokens[0].coins, START_COINS + START_COINS.min(GRAPPLE_COINS));
    assert_eq!(tokens[0].coins + tokens[1].coins, START_COINS * 2, "coins only move between players");
}

#[test]
fn grapples_report_the_real_transfer_and_empty_pockets_without_moving_players() {
    for purse in [0,3,30] {
        let mut board=Board::new(&seats(2),3,13);
        board.place_for_test(0,0,20);
        board.place_for_test(1,4,purse);
        board.give_for_test(0,Item::Grapple);
        hit_with(&mut board,0,Item::Grapple,1);
        let stolen=purse.min(GRAPPLE_COINS);
        assert_eq!(token_of(&board,0).coins,20+stolen);
        assert_eq!(token_of(&board,1).coins,purse-stolen);
        assert_eq!((token_of(&board,0).space,token_of(&board,1).space),(0,4),"the hook reels in coins rather than moving either player");
        let result=board.take_events().into_iter().find(|event|event["kind"]=="item" && event["item"]=="grapple" && event["target"]==1).unwrap();
        assert_eq!(result["coins"],stolen);
        assert_eq!(result["quality"],if purse==0 {"empty"} else {"hit"});
        assert!(result["flight"].as_f64().unwrap()>0.0,"the result carries flight timing for impact feedback");
        assert_eq!(board.step(),&TurnStep::Items,"there is no result confirmation before the next action");
    }
}

#[test]
fn a_shield_blocks_a_grapple_without_transferring_coins() {
    let mut board=Board::new(&seats(2),3,13);
    board.place_for_test(0,0,20);
    board.place_for_test(1,4,30);
    board.raise_shield_for_test(1);
    board.give_for_test(0,Item::Grapple);
    hit_with(&mut board,0,Item::Grapple,1);
    assert_eq!((token_of(&board,0).coins,token_of(&board,1).coins),(20,30));
    assert!(!token_of(&board,1).shield,"the shield absorbs the hook and breaks");
    let result=board.take_events().into_iter().find(|event|event["kind"]=="item" && event["item"]=="grapple" && event["target"]==1).unwrap();
    assert_eq!(result["quality"],"blocked");
    assert_eq!(board.step(),&TurnStep::Items);
}

#[test]
fn losing_your_health_costs_coins_and_a_turn() {
    let mut board = Board::new(&seats(2), 3, 14);
    board.place_for_test(0, 12, 10);
    board.place_for_test(1, 12, 30);
    for _ in 0..MAX_HEALTH {
        board.give_for_test(0, Item::Bat);
        bat(&mut board, 0, 1);
    }
    let token = board.view().tokens.iter().find(|t| t.seat == 1).cloned().unwrap();
    assert_eq!(token.health, MAX_HEALTH, "healed after the knockout");
    // Three perfect swings knock six each loose, and the knockout costs the rest.
    assert_eq!(token.coins, 30 - 3 * 6 - KNOCKOUT_COINS);
    assert_eq!(token.resting, 1, "and sits out a turn");
    let hurts: Vec<_> = board.take_events().into_iter().filter(|e| e["kind"] == "hurt").collect();
    assert_eq!(hurts.len(), MAX_HEALTH as usize, "every hit is reported");
    assert_eq!(hurts.last().unwrap()["health"], 0);
}

#[test]
fn nobody_starts_with_items_and_every_minigame_participant_earns_one() {
    let mut board = Board::new(&seats(4), 1, 31);
    assert!(board.view().tokens.iter().all(|t| t.items.is_empty()));
    run_board_round(&mut board);
    let before: Vec<usize> = board.view().tokens.iter().map(|t| t.items.len()).collect();
    board.finish_round(&[vec![2], vec![0, 3], vec![1]]);
    let after: Vec<usize> = board.view().tokens.iter().map(|t| t.items.len()).collect();
    assert_eq!(after[2], (before[2] + 1).min(MAX_ITEMS), "the winner drew an item");
    assert_eq!((after[0], after[1], after[3]), ((before[0]+1).min(MAX_ITEMS), (before[1]+1).min(MAX_ITEMS), (before[3]+1).min(MAX_ITEMS)), "everyone receives their draw");
}

#[test]
fn minigame_results_set_the_next_turn_order() {
    let mut board = Board::new(&seats(4), 3, 32);
    run_board_round(&mut board);
    board.finish_round(&[vec![2], vec![3, 0], vec![1]]);
    assert_eq!(board.view().order, vec![2, 0, 3, 1], "winner first, ties keep their old order");
    let first = board.turn_seat();
    assert!(first == 2 || board.view().tokens.iter().find(|t| t.seat == 2).unwrap().resting > 0);
}

#[test]
fn bots_play_a_whole_party_and_someone_wins() {
    let mut board = Board::new(&seats(4), 3, 15);
    let mut rounds = 0;
    loop {
        run_board_round(&mut board);
        rounds += 1;
        // Pretend the minigame ended in seat order.
        let placements: Vec<Vec<u8>> = (0..4u8).map(|seat| vec![seat]).collect();
        if !board.finish_round(&placements) {
            break;
        }
        assert!(rounds < 10, "the party ends after its rounds");
    }
    assert_eq!(rounds, 3, "one board round per party round");
    let standings = board.placements();
    assert_eq!(
        standings.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    let scores: Vec<(u32, i32)> = board.standings().iter().map(|(_, score)| *score).collect();
    assert!(
        scores.windows(2).all(|w| w[0] >= w[1]),
        "standings run by prizes then coins"
    );
}

#[test]
fn a_room_runs_a_tawkathon_from_the_board_through_a_bout() {
    let mut room = Room::new("party-1", 16);
    room.connect("alice", "Alice", "").unwrap();
    room.add_bot("alice").unwrap();
    room.start_party("alice").unwrap();
    assert!(matches!(room.phase,Phase::Vote{..}));
    assert!(room.view().opening_minigame);
    assert!(room.view().board.is_some(), "the board is in the room view");

    // With Alice away every seat plays itself, so the party runs on its own.
    room.disconnect("alice");
    let mut saw_minigame = false;
    let mut saw_board_again = false;
    for _ in 0..(60 * 400) {
        room.step();
        saw_minigame |= room.phase == Phase::Playing;
        if saw_minigame && room.phase == Phase::Board && room.board.as_ref().is_some_and(|b|b.round>=2) {
            saw_board_again = true;
            break;
        }
        if matches!(room.phase, Phase::GameOver { .. }) {
            break;
        }
    }
    assert!(saw_minigame, "the round ended in a minigame");
    assert!(saw_board_again, "and the party went back to the board");
    let view = room.view().board.unwrap();
    assert!(view.round >= 2, "the party moved on to the next round");
    assert!(
        view.tokens.iter().any(|t| t.coins != START_COINS),
        "coins moved"
    );
}

#[test]
fn a_lobby_set_for_a_tawkathon_previews_its_board() {
    let mut room = Room::new("lobby-preview", 4);
    room.connect("host", "Host", "").unwrap();
    room.connect("guest", "Guest", "").unwrap();
    let preview = |room: &Room| room.view().preview_board.map(|board| board.map);
    assert_eq!(preview(&room), Some(board::Map::Frozen), "a Tawkathon by default, in the snowy city");
    assert!(room.choose_map("guest", board::Map::Frozen).is_err(), "only the host picks");
    room.choose_map("host", board::Map::Frozen).unwrap();
    assert_eq!(preview(&room), Some(board::Map::Frozen), "the preview follows the pick");
    assert_eq!(
        room.view().preview_board.unwrap().spaces.len(),
        room.view().preview_board.unwrap().spaces.len(),
        "the same board every time it is looked at",
    );
    room.choose_mode("host", false).unwrap();
    assert_eq!(preview(&room), None, "a single bout previews the bout instead");
    room.choose_mode("host", true).unwrap();
    room.start_party("host").unwrap();
    assert_eq!(preview(&room), None, "and nothing once the party is on");
}

#[test]
fn people_rate_each_bout_they_played_once_and_can_add_a_note() {
    let mut room = Room::new("party-feedback", 16);
    room.connect("alice", "Alice", "").unwrap();
    room.connect("bob", "Bob", "").unwrap();
    room.start_party("alice").unwrap();
    assert!(room.feedback("alice", 4, "").is_err(), "nothing to rate before a bout");
    room.disconnect("alice");
    room.disconnect("bob");
    for _ in 0..(60 * 400) {
        room.step();
        if matches!(room.phase, Phase::Results { .. }) {
            break;
        }
    }
    assert!(matches!(room.phase, Phase::Results { .. }), "a bout finished");
    room.connect("carol", "Carol", "").unwrap();
    let ask = |room: &Room, seat: Option<u8>| room.view_for(seat).feedback.map(|ask| ask.number);
    assert_eq!(ask(&room, room.seat_of("alice")), Some(1), "Alice is asked about the bout");
    assert_eq!(ask(&room, room.seat_of("carol")), None, "Carol did not play it");

    assert!(room.feedback("alice", 0, "").is_err(), "ratings run 1 to 5");
    let long = "x".repeat(800);
    let rated = room.feedback("alice", 4, &format!("  {long}  ")).unwrap();
    assert_eq!((rated.rating, rated.bout.number, rated.name.as_str()), (4, 1, "Alice"));
    assert_eq!(rated.note.len(), crate::room::FEEDBACK_NOTE_MAX, "a note is trimmed and capped");
    assert!(rated.party_id.starts_with("party-feedback-1-"), "it names the Tawkathon");
    assert!(room.feedback("alice", 5, "").is_err(), "once per person per bout");
    assert_eq!(ask(&room, room.seat_of("alice")), None, "and then the prompt goes");
    assert!(room.feedback("carol", 5, "").is_err(), "only people who played it");

    // Bob rates with one tap, and the prompt stays for a note.
    let tapped = room.feedback("bob", 2, "").unwrap();
    assert!(!tapped.follow_up && tapped.note.is_empty());
    let bob = room.seat_of("bob");
    assert_eq!(room.view_for(bob).feedback.and_then(|ask| ask.rated), Some(2), "the card remembers the stars");
    assert!(room.feedback("bob", 5, "").is_err(), "a second tap does not rate again");
    let noted = room.feedback("bob", 0, "  more ramps please ").unwrap();
    assert!(noted.follow_up, "a note after the rating tops it up");
    assert_eq!((noted.rating, noted.note.as_str()), (2, "more ramps please"), "it keeps the stars given");
    assert!(room.feedback("bob", 0, "and again").is_err(), "one note each");
    assert_eq!(ask(&room, bob), None, "and then the prompt goes");
    let comment = crate::feedback::comment(&noted);
    assert!(comment.contains("Bob added a note to their 2/5") && comment.contains("> more ramps please"), "{comment}");

    let comment = crate::feedback::comment(&rated);
    assert!(comment.contains("★★★★☆ 4/5") && comment.contains("Alice finished"), "{comment}");
    assert!(comment.contains("\n\n> x"), "the note is quoted: {comment}");
}

#[test]
fn a_bout_played_on_its_own_can_be_rated_too() {
    let mut room = Room::new("single-feedback", 16);
    room.connect("alice", "Alice", "").unwrap();
    room.connect("bob", "Bob", "").unwrap();
    room.choose_mode("alice", false).unwrap();
    let play = |room: &mut Room| {
        room.start("alice").unwrap();
        for _ in 0..(60 * 400) {
            room.step();
            if matches!(room.phase, Phase::Results { .. }) {
                break;
            }
        }
        assert!(matches!(room.phase, Phase::Results { .. }), "a bout finished");
    };
    play(&mut room);
    let ask = room.view_for(room.seat_of("alice")).feedback.expect("Alice is asked about the bout");
    assert_eq!(ask.number, 1);
    let rated = room.feedback("alice", 3, "").unwrap();
    assert_eq!(rated.bout.round, 0, "no board round");
    assert!(rated.party_id.starts_with("single-feedback:bouts:"), "{}", rated.party_id);
    let comment = crate::feedback::comment(&rated);
    assert!(comment.contains(", bout 1: ★★★☆☆ 3/5") && !comment.contains("round"), "{comment}");

    while room.phase != Phase::Lobby {
        room.step();
    }
    assert!(room.view_for(room.seat_of("bob")).feedback.is_some(), "the card outlasts the results");
    play(&mut room);
    let next = room.feedback("alice", 5, "").unwrap();
    assert_eq!(next.bout.number, 2, "bouts on their own keep counting");
    assert_eq!(next.party_id, rated.party_id, "and share one ticket");
}

/// Plays the board with everyone away until the round's bout goes to a vote.
fn room_at_vote(people: &[&str], bots: usize, seed: u64) -> Room {
    let mut room = Room::new("tawkathon-vote", seed);
    for person in people {
        room.connect(person, person, "").unwrap();
    }
    for _ in 0..bots {
        room.add_bot(people[0]).unwrap();
    }
    room.start_party(people[0]).unwrap();
    for person in people {
        room.disconnect(person);
    }
    for _ in 0..(60 * 400) {
        room.step();
        if matches!(room.phase, Phase::Vote { .. }) {
            break;
        }
    }
    assert!(matches!(room.phase, Phase::Vote { .. }), "the round went to a vote");
    for person in people {
        room.connect(person, person, "").unwrap();
    }
    room
}

fn ballot(room: &Room) -> Vec<&'static str> {
    match &room.phase {
        Phase::Vote { options, .. } => options.clone(),
        other => panic!("expected a vote, got {other:?}"),
    }
}

#[test]
fn a_tawkathon_round_offers_three_different_bouts() {
    let room = room_at_vote(&["alice"], 1, 31);
    let options = ballot(&room);
    assert_eq!(options.len(), 3);
    let mut unique = options.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3, "no bout is on the ballot twice");
    assert!(options.iter().all(|key| crate::bout::find(key).is_some()));
}

#[test]
fn the_bout_everyone_votes_for_is_played() {
    let mut room = room_at_vote(&["alice", "bob"], 0, 32);
    let pick = ballot(&room)[2];
    room.vote("alice", pick).unwrap();
    assert!(matches!(room.phase, Phase::Vote { .. }), "the vote waits for bob");
    room.vote("bob", pick).unwrap();
    room.step();
    assert!(matches!(room.phase, Phase::Practice { .. }), "everyone voted, so the bout starts, with a practice go");
    assert_eq!(room.minigame, pick);
    assert_eq!(room.game.as_ref().map(|g| g.key()), Some(pick));
}

#[test]
fn votes_only_count_for_bouts_on_the_ballot() {
    let mut room = room_at_vote(&["alice"], 1, 33);
    let options = ballot(&room);
    let off_ballot = crate::bout::BOUTS.iter().map(|s| s.key).find(|k| !options.contains(k)).unwrap();
    assert_eq!(room.vote("alice", off_ballot), Err(RoomError::UnknownMinigame));
    assert_eq!(room.vote("mallory", options[0]), Err(RoomError::WrongPhase));
}

#[test]
fn bots_vote_and_an_unfinished_vote_still_ends() {
    let mut room = room_at_vote(&["alice"], 2, 34);
    let options = ballot(&room);
    for _ in 0..(60 * 3) {
        room.step();
    }
    match &room.phase {
        Phase::Vote { votes, .. } => assert_eq!(votes.len(), 2, "both bots have voted"),
        other => panic!("alice has not voted, so the vote is still open: {other:?}"),
    }
    for _ in 0..(60 * 12) {
        room.step();
    }
    assert!(!matches!(room.phase, Phase::Vote { .. }), "the vote timed out");
    assert!(options.contains(&room.minigame), "the bout came from the ballot");
}

#[test]
fn the_map_has_shops_hazards_a_teleport_and_prize_spots() {
    let spaces = board();
    for kind in [
        SpaceKind::Shop,
        SpaceKind::Hazard,
        SpaceKind::Teleport,
        SpaceKind::PrizeSpot,
    ] {
        assert!(
            spaces.iter().any(|s| s.kind == kind),
            "the map has a {kind:?} space"
        );
    }
    for space in spaces.iter().filter(|s| s.kind == SpaceKind::Teleport) {
        let link = space.link.expect("a teleport goes somewhere");
        assert!((link as usize) < spaces.len());
    }
    assert!(
        prize_spots().len() >= 2,
        "the prize has somewhere else to move to"
    );
    // The spur is a real detour: some space leads on to a space that is not the next id.
    assert!(
        spaces.iter().any(|s| s.next.iter().any(|n| *n != s.id + 1)),
        "the map winds"
    );
}

/// Takes out the vacuum on `seat`'s turn and follows the chest at `spot` as
/// it drifts, until it comes in or the vacuum runs out. Whether it came in.
fn vacuum_chest(board: &mut Board, seat: u8, spot: u8) -> bool {
    board.turn_to_for_test(seat);
    let before = token_of(board, seat).prizes;
    board.suck(seat).unwrap();
    let at = crate::board::board()[spot as usize].clone();
    for _ in 0..(60 * 6) {
        let out = board.view().vacuum.map(|v| v.out).unwrap_or(0.0);
        let (ox, oz) = Board::chest_drift(out);
        let here = crate::board::board()[token_of(board, seat).space as usize].clone();
        if board.aim_vacuum(seat, (at.x + ox - here.x).atan2(at.z + oz - here.z), true).is_err() {
            break;
        }
        board.vacuum_tick(1.0 / 60.0);
    }
    token_of(board, seat).prizes > before
}

#[test]
fn vacuuming_the_prize_in_buys_it_and_moves_it_on() {
    let mut board = Board::new(&seats(2), 3, 21);
    board.clear_crates_for_test();board.clear_air_for_test();
    let spot = board.view().prize_at;
    board.place_for_test(0, spot, 40);
    assert!(vacuum_chest(&mut board, 0, spot));
    let token = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 0)
        .cloned()
        .unwrap();
    assert_eq!(token.prizes, 1, "the prize was bought");
    assert_eq!(token.coins, 40 - PRIZE_COST, "and paid for");
    assert_ne!(
        board.view().prize_at,
        spot,
        "the prize moved somewhere else"
    );
}

#[test]
fn a_hazard_costs_coins_and_a_heart_and_a_shop_sells_an_item() {
    let mut board = Board::new(&seats(2), 3, 22);
    let hazard = board_space_of(SpaceKind::Hazard);
    board.place_for_test(0, hazard, 30);
    board.land_for_test(0);
    let hurt = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 0)
        .cloned()
        .unwrap();
    assert_eq!(hurt.coins, 30 - HAZARD_COINS);
    assert_eq!(hurt.health, MAX_HEALTH - 1);

    let shop = board_space_of(SpaceKind::Shop);
    let before = hurt.items.len();
    board.place_for_test(0, shop, 30);
    board.land_for_test(0);
    // The shop asks what you want before it sells you anything.
    let TurnStep::Shop { offers, .. } = board.step().clone() else {
        panic!("the shop opened");
    };
    board.shop(0, Some(offers[0].item)).unwrap();
    let shopped = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 0)
        .cloned()
        .unwrap();
    assert_eq!(shopped.items.len(), before + 1, "the shop sold an item");
}

#[test]
fn a_teleport_carries_you_to_its_far_end() {
    let mut board = Board::new(&seats(2), 3, 23);
    let pad = board_space_of(SpaceKind::Teleport);
    let link = board::space(pad).link.unwrap();
    board.place_for_test(0, pad, 10);
    board.land_for_test(0);
    assert_eq!(
        board
            .view()
            .tokens
            .iter()
            .find(|t| t.seat == 0)
            .unwrap()
            .space,
        link
    );
}

fn board_space_of(kind: SpaceKind) -> u8 {
    board()
        .iter()
        .find(|s| s.kind == kind)
        .expect("the map has one")
        .id
}

#[test]
fn no_two_spaces_sit_on_top_of_each_other() {
    let spaces = board();
    for (i, a) in spaces.iter().enumerate() {
        for b in spaces.iter().skip(i + 1) {
            let gap = ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
            assert!(
                gap > 1.6,
                "spaces {} and {} are {gap:.2} apart and would overlap",
                a.id,
                b.id
            );
        }
    }
}

#[test]
fn the_bat_only_hits_someone_within_a_swing() {
    let mut board = Board::new(&seats(2), 3, 4);
    let hitter = board.turn_seat();
    let mark = if hitter == 0 { 1 } else { 0 };
    let spaces = board.view().spaces;
    let reach = |id: u8| {
        let (a, b) = (&spaces[0], &spaces[id as usize]);
        ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt()
    };
    let far = spaces.iter().map(|s| s.id).find(|id| reach(*id) > crate::board::BAT_RANGE + 1.0).unwrap();
    board.place_for_test(hitter, 0, 20);
    board.place_for_test(mark, far, 20);
    let toward = |id: u8| {
        let (a, b) = (&spaces[0], &spaces[id as usize]);
        (b.x - a.x).atan2(b.z - a.z)
    };
    board.give_for_test(hitter, Item::Bat);
    board.use_item(hitter, Item::Bat, None).unwrap();
    board.swing(hitter, toward(far), 0.5).unwrap();
    assert_eq!(token_of(&board, mark).space, far, "a rival across the map is out of reach");

    let near = spaces.iter().map(|s| s.id).find(|id| *id != 0 && reach(*id) <= crate::board::BAT_RANGE).unwrap();
    board.place_for_test(mark, near, 20);
    board.give_for_test(hitter, Item::Bat);
    board.use_item(hitter, Item::Bat, None).unwrap();
    board.swing(hitter, toward(near), 0.5).unwrap();
    let hit = token_of(&board, mark);
    assert_eq!(hit.health, MAX_HEALTH - 1, "the swing landed");

    // Swinging the other way, or with the timing well off, misses.
    board.place_for_test(mark, near, 20);
    board.give_for_test(hitter, Item::Bat);
    board.use_item(hitter, Item::Bat, None).unwrap();
    let (a, b) = (&spaces[0], &spaces[near as usize]);
    board.swing(hitter, (b.x - a.x).atan2(b.z - a.z) + std::f32::consts::PI, 0.5).unwrap();
    assert_eq!(token_of(&board, mark).space, near, "facing away");
    board.give_for_test(hitter, Item::Bat);
    board.use_item(hitter, Item::Bat, None).unwrap();
    board.swing(hitter, (b.x - a.x).atan2(b.z - a.z), 0.0).unwrap();
    assert_eq!(token_of(&board, mark).space, near, "a whiff");
}

fn token_of(board: &Board, seat: u8) -> crate::board::Token {
    board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap()
}

/// Two seats side by side on the start, the first one to move holding `item`.
fn duel(item: Item, seed: u64) -> (Board, u8, u8) {
    let mut board = Board::new(&seats(2), 3, seed);
    let me = board.turn_seat();
    let them = if me == 0 { 1 } else { 0 };
    board.place_for_test(me, 0, 20);
    board.place_for_test(them, 0, 20);
    board.give_for_test(me, item);
    (board, me, them)
}

#[test]
fn dice_items_change_the_roll() {
    for seed in 0..20 {
        let (mut board, me, _) = duel(Item::DoubleDice, seed);
        use_now(&mut board, me, Item::DoubleDice, None);
        board.give_for_test(me, Item::GoldenDie);
        assert_eq!(
            board.use_item_with(me, Item::GoldenDie, None, Some(4)),
            Err(BoardError::DiceSet),
            "one dice item a turn"
        );
        let roll = board.roll(me).unwrap();
        assert!((2..=12).contains(&roll), "two dice add up");
    }
    let (mut board, me, _) = duel(Item::GoldenDie, 1);
    assert_eq!(
        board.use_item_with(me, Item::GoldenDie, None, Some(9)),
        Err(BoardError::PickANumber)
    );
    use_now(&mut board, me, Item::GoldenDie, Some(5));
    assert_eq!(board.roll(me).unwrap(), 5, "the golden die lands where it was set");
}

#[test]
fn a_cursed_player_rolls_low_once() {
    for seed in 0..20 {
        let (mut board, me, them) = duel(Item::Curse, seed);
        hit_with(&mut board, me, Item::Curse, them);
        assert!(token_of(&board, them).cursed);
        board.roll(me).unwrap();
        board.end_turn();
        assert_eq!(board.turn_seat(), them);
        assert!(board.roll(them).unwrap() <= CURSED_SIDES);
        assert!(!token_of(&board, them).cursed, "the curse is spent");
    }
}

#[test]
fn shields_block_bats_and_bombs() {
    let (mut board, me, them) = duel(Item::Bat, 3);
    board.give_for_test(me, Item::Bomb);
    board.raise_shield_for_test(them);
    bat(&mut board, me, them);
    let after = token_of(&board, them);
    assert_eq!(after.health, MAX_HEALTH, "the bat bounced off");
    assert!(!after.shield, "the shield broke");
    board.use_item(me, Item::Bomb, None).unwrap();
    board.calm_for_test();
    let at = token_of(&board, them).space;
    throw_at(&mut board, me, at);
    assert_eq!(token_of(&board, them).health, MAX_HEALTH - 1, "the second hit lands");
}

/// Steps `them` up beside you and swings the bat in hand at them, with perfect timing.
fn bat(board: &mut Board, me: u8, them: u8) {
    board.use_item(me, Item::Bat, None).unwrap();
    let here = token_of(board, me).space;
    board.move_for_test(them, here);
    board.swing(me, 0.0, 0.5).unwrap();
}

/// Throws something at `them`'s space, true and with no wind.
fn hit_with(board: &mut Board, me: u8, item: Item, them: u8) {
    board.use_item(me, item, None).unwrap();
    board.calm_for_test();
    let at = token_of(board, them).space;
    throw_at(board, me, at);
}

/// Throws the bomb in hand so it comes down on a space, with no wind.
fn throw_at(board: &mut Board, seat: u8, space: u8) {
    let me = token_of(board, seat).space;
    let (from, to) = (crate::board::board()[me as usize].clone(), crate::board::board()[space as usize].clone());
    let (dx, dz) = (to.x - from.x, to.z - from.z);
    let pitch = std::f32::consts::FRAC_PI_4;
    // Solve the flight for the power, allowing for the height of the hand.
    let reach = (dx * dx + dz * dz).sqrt();
    let mut power = 0.0f32;
    let mut best = f32::MAX;
    for step in 0..=2000 {
        let p = step as f32 / 2000.0;
        let ((x, z), _) = Board::bomb_landing((from.x, from.z), dx.atan2(dz), pitch, p, (0.0, 0.0));
        let off = ((x - from.x).powi(2) + (z - from.z).powi(2)).sqrt() - reach;
        if off.abs() < best {
            best = off.abs();
            power = p;
        }
    }
    board.lob(seat, dx.atan2(dz), pitch, power).unwrap();
}

#[test]
fn a_bomb_comes_down_where_the_throw_carries_it() {
    let (mut board, me, them) = duel(Item::Bomb, 20);
    board.place_for_test(them, 6, 20);
    board.use_item(me, Item::Bomb, None).unwrap();
    let TurnStep::Lob { wind, fuse, .. } = board.step().clone() else {
        panic!("a bomb in hand waits for the throw");
    };
    assert!(fuse > 0.0 && (wind.0 * wind.0 + wind.1 * wind.1).sqrt() <= crate::board::BOMB_WIND + 0.01);
    board.calm_for_test();
    throw_at(&mut board, me, 6);
    let hit = token_of(&board, them);
    assert_eq!(hit.coins, 20 - crate::board::BOMB_DIRECT_COINS, "thrown true, it lands right on them");
    assert_eq!(hit.health, MAX_HEALTH - 1);
    assert_eq!(*board.step(), TurnStep::Items, "and the turn carries on");

    // The same throw into a stiff wind is carried off them.
    let (mut board, me, them) = duel(Item::Bomb, 20);
    board.place_for_test(them, 6, 20);
    board.use_item(me, Item::Bomb, None).unwrap();
    board.wind_for_test((30.0, 30.0));
    throw_at(&mut board, me, 6);
    assert_eq!(token_of(&board, them).coins, 20, "blown wide");

    // No power at all drops it at your own feet.
    let (mut board, me, _) = duel(Item::Bomb, 20);
    board.use_item(me, Item::Bomb, None).unwrap();
    board.lob(me, 0.0, 0.0, 0.0).unwrap();
    assert_eq!(token_of(&board, me).coins, 20 - crate::board::BOMB_DIRECT_COINS, "dropped, it is yours");

    // And held too long, it goes off in the hand.
    let (mut board, me, _) = duel(Item::Bomb, 20);
    board.use_item(me, Item::Bomb, None).unwrap();
    board.fuse_out(me);
    assert_eq!(token_of(&board, me).health, MAX_HEALTH - 1);
}

#[test]
fn swap_pickpocket_medkit_and_compass_do_their_jobs() {
    let (mut board, me, them) = duel(Item::Swap, 5);
    board.place_for_test(them, 12, 20);
    hit_with(&mut board, me, Item::Swap, them);
    assert_eq!(token_of(&board, me).space, 12);
    assert_eq!(token_of(&board, them).space, 0);

    let (mut board, me, them) = duel(Item::Pickpocket, 5);
    board.place_for_test(them, 8, 20);
    hit_with(&mut board, me, Item::Pickpocket, them);
    assert!(token_of(&board, me).items.is_empty(), "nothing to take from empty pockets");
    board.give_for_test(me, Item::Pickpocket);
    board.give_for_test(them, Item::Bomb);
    hit_with(&mut board, me, Item::Pickpocket, them);
    assert_eq!(token_of(&board, me).items, vec![Item::Bomb]);
    assert!(token_of(&board, them).items.is_empty());

    let (mut board, me, them) = duel(Item::Bat, 5);
    bat(&mut board, me, them);
    board.end_turn();
    board.give_for_test(them, Item::Medkit);
    use_now(&mut board, them, Item::Medkit, None);
    assert_eq!(token_of(&board, them).health, MAX_HEALTH);

    // The compass lands you nearer the prize the better you stop it.
    for (stop, short) in [(0.5, Some(1)), (0.3, Some(COMPASS_SHORT)), (0.15, Some(COMPASS_SHORT * 2)), (0.0, None)] {
        let (mut board, me, _) = duel(Item::Compass, 5);
        let prize = board.view().prize_at;
        let start = token_of(&board, me).space;
        board.use_item(me, Item::Compass, None).unwrap();
        board.check(me, stop).unwrap();
        let at = token_of(&board, me).space;
        match short {
            Some(short) => assert_eq!(distance(at, prize), short, "stopped at {stop}"),
            None => assert_eq!(at, start, "a fumbled compass goes nowhere"),
        }
    }
}

#[test]
fn a_medkit_heals_as_well_as_the_check_goes() {
    for (stop, health) in [(0.5, MAX_HEALTH), (0.3, 3), (0.12, 2), (0.0, 1)] {
        let (mut board, me, _) = duel(Item::Medkit, 5);
        board.place_for_test(me, 0, 20);
        for _ in 0..MAX_HEALTH - 1 {
            board.hurt_for_test(me);
        }
        assert_eq!(token_of(&board, me).health, 1);
        board.use_item(me, Item::Medkit, None).unwrap();
        board.check(me, stop).unwrap();
        assert_eq!(token_of(&board, me).health, health.min(MAX_HEALTH), "stopped at {stop}");
    }
}

#[test]
fn rounds_including_the_final_one_have_no_table_wide_events() {
    let mut board = Board::new(&seats(4), 4, 8);
    assert_eq!(board.view().event, None);
    for round in 2..=4 {
        run_board_round(&mut board);
        assert!(board.finish_round(&[vec![0], vec![1], vec![2], vec![3]]));
        assert_eq!(board.view().round,round);
        assert_eq!(board.view().event,None);
        assert_eq!(board.prize_cost(0),PRIZE_COST);
        assert_eq!(board.open_prizes().len(),1);
        assert!(board.view().air_coins.iter().all(|coin|coin.coins==1));
        assert!(board.view().log.iter().all(|entry|entry["kind"]!="event"));
    }
}

#[test]
fn a_coin_shower_doubles_floating_coin_value_without_automatic_landing_pay() {
    let mut board = Board::new(&seats(2), 3, 6);
    let coin = board_space_of(SpaceKind::Coin);
    board.place_for_test(0, coin, 0);
    board.land_for_test(0);
    assert_eq!(token_of(&board, 0).coins, 0);
    assert!(board.view().air_coins.iter().all(|coin|coin.coins==1));
    board.set_event_for_test(RoundEvent::CoinShower);
    board.place_for_test(0, coin, 0);
    board.land_for_test(0);
    assert_eq!(token_of(&board, 0).coins, 0);
    assert!(board.view().air_coins.iter().all(|coin|coin.coins==2));
}

#[test]
fn the_last_round_hands_out_bonus_prizes() {
    let mut board = Board::new(&seats(3), 1, 8);
    board.give_for_test(2, Item::Shield);
    while board.turn_seat() != 2 {
        let seat = board.turn_seat();
        board.roll(seat).unwrap();
        board.end_turn();
    }
    use_now(&mut board, 2, Item::Shield, None);
    board.roll(2).unwrap();
    board.end_turn();
    let before: Vec<u32> = (0..3).map(|seat| token_of(&board, seat).prizes).collect();
    assert!(!board.finish_round(&[vec![1], vec![0], vec![2]]));
    let bonuses = board.view().bonuses;
    let champion = bonuses.iter().find(|b| b.bonus == Bonus::Champion).unwrap();
    assert_eq!(champion.seats, vec![1]);
    let troublemaker = bonuses.iter().find(|b| b.bonus == Bonus::Troublemaker).unwrap();
    assert_eq!(troublemaker.seats, vec![2]);
    assert!(bonuses.iter().any(|b| b.bonus == Bonus::Explorer));
    let won: u32 = (0..3).map(|seat| token_of(&board, seat).prizes - before[seat as usize]).sum();
    let handed: usize = bonuses.iter().map(|b| b.seats.len()).sum();
    assert_eq!(won as usize, handed, "every bonus is worth a prize");
}

#[test]
fn a_team_bout_splits_the_table_two_ways() {
    for seed in 0..25 {
        let mut board = Board::new(&seats(4), 3, seed);
        let split = board.team_split(&seats(4));
        assert_eq!(split.len(), 4, "everyone is given a side");
        let reds = split.iter().filter(|(_, team)| *team == 0).count();
        assert!(reds >= 1 && reds <= 3, "neither side is empty: {split:?}");
        // Either an even split, or one player against the rest.
        assert!(reds == 2 || reds == 1 || reds == 3, "{split:?}");
        assert!(
            board.view().log.iter().any(|entry| entry["kind"] == "teams"),
            "the split is announced"
        );
    }
}

#[test]
fn one_against_the_rest_is_always_the_leader_alone() {
    let mut board = Board::new(&seats(4), 3, 3);
    board.place_for_test(2, 0, 500);
    for seed in 0..40 {
        let mut board = Board::new(&seats(4), 3, seed);
        board.place_for_test(2, 0, 500);
        let split = board.team_split(&seats(4));
        let alone: Vec<u8> = split.iter().filter(|(_, team)| *team == 1).map(|(seat, _)| *seat).collect();
        if alone.len() == 1 {
            assert_eq!(alone, vec![2], "the player in front stands alone");
        }
    }
}

#[test]
fn a_knockout_leaves_its_coins_on_the_board() {
    let (mut board, me, them) = duel(Item::Bat, 9);
    board.place_for_test(them, 0, 40);
    for _ in 0..MAX_HEALTH {
        board.give_for_test(me, Item::Bat);
        bat(&mut board, me, them);
    }
    // Each swing knocks coins loose too; the knockout's own pile is what is checked.
    let dropped: i64 = board
        .view()
        .log
        .iter()
        .filter(|entry| entry["kind"] == "spill")
        .flat_map(|entry| entry["dropped"].as_array().cloned().unwrap_or_default())
        .map(|pile| pile["coins"].as_i64().unwrap_or(0))
        .sum();
    assert_eq!(dropped, KNOCKOUT_COINS as i64, "the whole purse hit the floor");
    assert!(board.view().loose.len() > 1, "spread over more than one space");

    // Whoever stops on them picks them up. Landing also pays whatever the
    // space itself pays, so only the pickup line is checked exactly.
    // On a space that does not stop you first, as a shop or a cannon would.
    let plain = |space: u8| {
        matches!(
            crate::board::space(space).kind,
            crate::board::SpaceKind::Coin | crate::board::SpaceKind::Loss | crate::board::SpaceKind::Item | crate::board::SpaceKind::Hazard
        )
    };
    let spot = *board.view().loose.iter().find(|row| plain(row.space)).expect("a pile on a plain space");
    board.place_for_test(me, spot.space, 0);
    board.land_for_test(me);
    let picked = board
        .view()
        .log
        .iter()
        .filter(|entry| entry["kind"] == "pickup")
        .map(|entry| entry["coins"].as_i64().unwrap_or(0) as i32)
        .sum::<i32>();
    assert_eq!(picked, spot.coins, "the pile on that space was taken");
    assert!(
        !board.view().loose.iter().any(|row| row.space == spot.space),
        "they are gone once taken"
    );
}

#[test]
fn a_knockout_with_an_empty_purse_leaves_nothing() {
    let (mut board, me, them) = duel(Item::Bat, 11);
    board.place_for_test(them, 0, 0);
    for _ in 0..MAX_HEALTH {
        board.give_for_test(me, Item::Bat);
        bat(&mut board, me, them);
    }
    assert!(board.view().loose.is_empty(), "you cannot drop what you do not have");
}

#[test]
fn stopping_on_someone_settles_it() {
    let mut wins = 0;
    let mut draws = 0;
    for seed in 0..40 {
        let mut board = Board::new(&seats(2), 3, seed);
        let me = board.turn_seat();
        let them = if me == 0 { 1 } else { 0 };
        let space = board_space_of(SpaceKind::Coin);
        board.place_for_test(me, space, 20);
        board.place_for_test(them, space, 20);
        board.land_for_test(me);
        let scrap = board
            .view()
            .log
            .iter()
            .find(|entry| entry["kind"] == "scrap")
            .cloned()
            .expect("the two of them settled it");
        if scrap["winner"].is_null() {
            draws += 1;
            assert_eq!(scrap["coins"], 0, "a tie costs nobody anything");
        } else {
            wins += 1;
            assert_eq!(scrap["coins"], SCRAP_COINS, "the winner takes the purse");
            let loser = if scrap["winner"].as_u64() == Some(me as u64) { them } else { me };
            assert_eq!(token_of(&board, loser).health, MAX_HEALTH - 1, "and a heart");
        }
    }
    assert!(wins > 0 && draws > 0, "both outcomes come up: {wins} wins, {draws} draws");
}

#[test]
fn a_resting_player_is_not_scrapped() {
    let mut board = Board::new(&seats(2), 3, 5);
    let me = board.turn_seat();
    let them = if me == 0 { 1 } else { 0 };
    let space = board_space_of(SpaceKind::Coin);
    board.place_for_test(me, space, 20);
    board.place_for_test(them, space, 20);
    board.rest_for_test(them);
    board.land_for_test(me);
    assert!(
        !board.view().log.iter().any(|entry| entry["kind"] == "scrap"),
        "someone sitting out is left alone"
    );
}

#[test]
fn the_shop_puts_out_a_choice_and_charges_by_rarity() {
    let mut board = Board::new(&seats(2), 3, 17);
    let me = board.turn_seat();
    let shop = board_space_of(SpaceKind::Shop);
    board.place_for_test(me, shop, 60);
    board.land_for_test(me);
    let TurnStep::Shop { offers, .. } = board.step().clone() else {
        panic!("the shop opened, not {:?}", board.step());
    };
    assert_eq!(offers.len(), SHOP_OFFERS, "three on the counter");
    let items: std::collections::HashSet<Item> = offers.iter().map(|offer| offer.item).collect();
    assert_eq!(items.len(), offers.len(), "no duplicates");
    for offer in &offers {
        assert_eq!(offer.price, offer.item.price());
        assert!(offer.price >= SHOP_COST, "nothing is cheaper than the old flat price");
    }

    let wanted = offers[0];
    board.shop(me, Some(wanted.item)).unwrap();
    let after = token_of(&board, me);
    assert_eq!(after.coins, 60 - wanted.price);
    assert!(after.items.contains(&wanted.item));
    assert_eq!(*board.step(), TurnStep::Done, "the turn carries on");
}

#[test]
fn you_can_walk_past_the_shop_and_cannot_overspend() {
    let mut board = Board::new(&seats(2), 3, 21);
    let me = board.turn_seat();
    let shop = board_space_of(SpaceKind::Shop);
    board.place_for_test(me, shop, 0);
    board.land_for_test(me);
    let TurnStep::Shop { offers, .. } = board.step().clone() else {
        panic!("the shop opened");
    };
    assert_eq!(
        board.shop(me, Some(offers[0].item)),
        Err(BoardError::TooDear),
        "an empty purse buys nothing"
    );
    board.shop(me, None).unwrap();
    assert_eq!(*board.step(), TurnStep::Done);
    assert!(token_of(&board, me).items.is_empty());
}

#[test]
fn a_heist_takes_a_prize_off_whoever_has_the_most() {
    let (mut board, me, them) = duel(Item::Heist, 13);
    assert_eq!(
        board.use_item(me, Item::Heist, None),
        Err(BoardError::NothingToTake),
        "there is nothing to take when nobody has a prize"
    );
    board.give_prize_for_test(them, 2);
    use_now(&mut board, me, Item::Heist, None);
    assert_eq!(token_of(&board, them).prizes, 1, "they lost one");
    assert_eq!(token_of(&board, me).prizes, 1, "and it moved across");

    // A shield turns it away, and the prize stays put.
    let (mut board, me, them) = duel(Item::Heist, 14);
    board.give_prize_for_test(them, 1);
    board.raise_shield_for_test(them);
    use_now(&mut board, me, Item::Heist, None);
    assert_eq!(token_of(&board, them).prizes, 1, "the shield held");
    assert_eq!(token_of(&board, me).prizes, 0);
}

#[test]
fn a_bot_never_reaches_for_a_heist_on_its_own_partner() {
    // Found by the eight-bot parties: in pairs, a bot whose partner held the
    // only prize kept trying a heist the board turned down, and the heist
    // hid the medkit behind it in the bag.
    let mut board = Board::new(&seats(4), 3, 41);
    board.set_partners(&[(0, 1), (2, 3)]);
    board.turn_to_for_test(0);
    board.give_prize_for_test(1, 2);
    board.give_for_test(0, Item::Heist);
    board.give_for_test(0, Item::Medkit);
    board.hurt_for_test(0);
    // The vacuum already had its go this turn.
    board.suck(0).unwrap();
    board.stow_vacuum(0).unwrap();
    for _ in 0..20 {
        let action = board.bot_move();
        assert!(
            matches!(action, BotMove::Item { item: Item::Medkit, .. } | BotMove::Pass),
            "nothing to take off a partner, so the medkit: {action:?}"
        );
    }
    board.give_prize_for_test(2, 1);
    assert!(
        (0..20).any(|_| matches!(board.bot_move(), BotMove::Item { item: Item::Heist, .. })),
        "a rival with a prize is worth the heist"
    );
}

#[test]
fn getting_back_at_someone_pays() {
    let (mut board, me, them) = duel(Item::Bat, 31);
    bat(&mut board, me, them);
    assert_eq!(token_of(&board, them).grudge, Some(me), "they remember who did it");
    assert_eq!(token_of(&board, me).grudge, None, "and the hitter holds nothing");

    board.end_turn();
    let before = token_of(&board, them).coins;
    board.give_for_test(them, Item::Bat);
    bat(&mut board, them, me);
    assert_eq!(
        token_of(&board, them).coins,
        before + REVENGE_COINS,
        "paying them back pays"
    );
    assert_eq!(token_of(&board, them).grudge, None, "the score is settled");
    assert_eq!(token_of(&board, me).grudge, Some(them), "and now it is the other way round");
}

#[test]
fn a_grudge_only_pays_against_the_one_who_earned_it() {
    let mut board = Board::new(&seats(3), 3, 33);
    let me = board.turn_seat();
    let them = (me + 1) % 3;
    let other = (me + 2) % 3;
    for seat in [me, them, other] {
        board.place_for_test(seat, 0, 30);
    }
    board.give_for_test(me, Item::Bat);
    bat(&mut board, me, them);
    board.end_turn();
    while board.turn_seat() != them {
        board.end_turn();
    }
    let before = token_of(&board, them).coins;
    board.give_for_test(them, Item::Bat);
    bat(&mut board, them, other);
    assert_eq!(
        token_of(&board, them).coins,
        before,
        "hitting a bystander settles nothing"
    );
    assert_eq!(token_of(&board, them).grudge, Some(me), "the grudge is still owed");
}

#[test]
fn a_taunt_carries_and_then_has_to_wait() {
    let mut room = Room::new("taunts", 0);
    room.connect("alice", "Alice", "").unwrap();
    room.connect("bob", "Bob", "").unwrap();
    room.start_party("alice").unwrap();
    let shouted = |room: &Room| {
        room.view()
            .board
            .map(|board| board.log.iter().filter(|entry| entry["kind"] == "taunt").count())
            .unwrap_or(0)
    };
    room.taunt("bob", Taunt::Laugh).unwrap();
    assert_eq!(shouted(&room), 1, "anyone can shout, not only the turn holder");
    room.taunt("bob", Taunt::Cry).unwrap();
    assert_eq!(shouted(&room), 1, "but not twice in a row");
    for _ in 0..(crate::bout::TICK_HZ * 5) {
        room.step();
    }
    room.taunt("bob", Taunt::Clap).unwrap();
    assert_eq!(shouted(&room), 2, "once the wait is over they can again");
}

#[test]
fn the_underdog_gets_something_to_act_with() {
    let mut board = Board::new(&seats(3), 3, 2);
    board.place_for_test(0, 0, 100);
    board.place_for_test(1, 0, 60);
    board.place_for_test(2, 0, 1);
    board.set_event_for_test_full(RoundEvent::Underdog);
    let last = token_of(&board, 2);
    assert_eq!(last.items.len(), 1, "last place drew an item");
    assert_eq!(board.prize_cost(2), UNDERDOG_PRIZE, "and the prize is within reach");
    assert_eq!(board.prize_cost(0), PRIZE_COST, "for them only");

    // Cheap enough to actually buy with what they have.
    let spot = board.view().prize_at;
    board.place_for_test(2, spot, UNDERDOG_PRIZE);
    assert!(vacuum_chest(&mut board, 2, spot), "and they bought it");
}

#[test]
fn the_last_round_is_worth_playing() {
    let mut board = Board::new(&seats(2), 1, 8);
    board.set_event_for_test(RoundEvent::LastRound);
    assert_eq!(board.prize_cost(0), PRIZE_COST / 2, "the prize is half price");
    assert!(board.open_prizes().len() > 1, "and buyable at every spot");

    let spot = *board.open_prizes().last().unwrap();
    board.place_for_test(0, spot, PRIZE_COST / 2);
    assert!(vacuum_chest(&mut board, 0, spot), "even away from where the chest sits");

    let before = token_of(&board, 1).coins;
    board.finish_round(&[vec![1], vec![0]]);
    assert_eq!(
        token_of(&board, 1).coins - before,
        MINIGAME_COINS[0],
        "even a legacy event fixture no longer doubles the final bout"
    );
}

#[test]
fn a_trap_is_set_at_range_and_only_its_owner_knows() {
    let mut board = Board::new(&seats(3), 3, 19);
    let me = board.turn_seat();
    let them = (me + 1) % 3;
    board.place_for_test(me, 0, 30);
    board.place_for_test(them, 30, 30);
    board.give_for_test(me, Item::Trap);

    // Set where you stand.
    let near = 5;
    board.place_for_test(me, near, 30);
    board.use_item(me, Item::Trap, None).unwrap();
    board.calm_for_test();
    throw_at(&mut board, me, near);

    let mine = board.view_for(Some(me)).traps;
    assert_eq!(mine.len(), 1, "the owner sees their own");
    assert!(mine[0].mine);
    assert_eq!(mine[0].space, near);
    assert!(
        board.view_for(Some(them)).traps.is_empty(),
        "and nobody else has heard about it"
    );
}

#[test]
fn walking_past_a_trap_is_how_you_hear_about_it() {
    let mut board = Board::new(&seats(2), 3, 23);
    let me = board.turn_seat();
    let them = if me == 0 { 1 } else { 0 };
    board.place_for_test(me, 0, 30);
    board.give_for_test(me, Item::Trap);
    let near = 5;
    board.place_for_test(me, near, 30);
    board.use_item(me, Item::Trap, None).unwrap();
    board.calm_for_test();
    throw_at(&mut board, me, near);
    assert!(board.view_for(Some(them)).traps.is_empty());

    // Stopping within a couple of spaces of it is enough to notice.
    let beside = board
        .view()
        .spaces
        .iter()
        .map(|space| space.id)
        .find(|id| *id != near && gap(*id, near) <= TRAP_RUMOUR)
        .unwrap();
    board.place_for_test(them, beside, 30);
    board.land_for_test(them);
    let heard = board.view_for(Some(them)).traps;
    assert_eq!(heard.len(), 1, "now they know something is there");
    assert!(!heard[0].mine, "but not that it is theirs");
}

#[test]
fn a_bet_is_held_and_the_pot_goes_to_the_winner() {
    let mut board = Board::new(&seats(3), 3, 29);
    for seat in 0..3 {
        board.place_for_test(seat, 0, 30);
    }
    board.bet(0, 6).unwrap();
    board.bet(1, 4).unwrap();
    assert_eq!(token_of(&board, 0).coins, 24, "the stake is held straight away");
    assert_eq!(board.view().bets.len(), 2, "and shown on the board");

    // Changing your mind gives the old stake back first.
    board.bet(0, 10).unwrap();
    assert_eq!(token_of(&board, 0).coins, 20);
    assert_eq!(
        board.view().bets.iter().find(|bet| bet.seat == 0).unwrap().coins,
        10
    );

    let before = token_of(&board, 1).coins;
    board.finish_round(&[vec![1], vec![0], vec![2]]);
    assert_eq!(
        token_of(&board, 1).coins - before,
        MINIGAME_COINS[0] + 14,
        "the winner takes the pot on top of the payout"
    );
    assert!(board.view().bets.is_empty(), "and the slate is clear");
}

#[test]
fn a_pot_nobody_won_goes_back() {
    let mut board = Board::new(&seats(3), 3, 31);
    for seat in 0..3 {
        board.place_for_test(seat, 0, 30);
    }
    board.bet(0, 5).unwrap();
    board.bet(2, 5).unwrap();
    board.finish_round(&[vec![1], vec![0], vec![2]]);
    assert_eq!(token_of(&board, 0).coins, 30 + MINIGAME_COINS[1], "stake returned");
    assert_eq!(token_of(&board, 2).coins, 30 + MINIGAME_COINS[2], "stake returned");
}

#[test]
fn you_cannot_stake_more_than_you_have_or_more_than_the_cap() {
    let mut board = Board::new(&seats(2), 3, 33);
    board.place_for_test(0, 0, 3);
    board.bet(0, 50).unwrap();
    assert_eq!(token_of(&board, 0).coins, 0, "an empty purse is as far as it goes");

    let mut board = Board::new(&seats(2), 3, 33);
    board.place_for_test(0, 0, 100);
    board.bet(0, 50).unwrap();
    assert_eq!(token_of(&board, 0).coins, 100 - MAX_BET, "and the cap holds");
}

#[test]
fn the_party_remembers_what_happened() {
    let mut board = Board::new(&seats(3), 1, 41);
    for seat in 0..3 {
        board.place_for_test(seat, 0, 20);
    }
    // One big swing, and one player taken apart.
    board.give_for_test(0, Item::Grapple);
    hit_with(&mut board, 0, Item::Grapple, 1);
    for _ in 0..MAX_HEALTH {
        board.give_for_test(0, Item::Bat);
        bat(&mut board, 0, 1);
    }
    run_board_round(&mut board);
    board.finish_round(&[vec![0], vec![1], vec![2]]);

    let reel = board.view().highlights;
    assert!(!reel.is_empty(), "the party is worth remembering");
    let swing = reel.iter().find(|one| one.highlight == Highlight::BiggestSwing).unwrap();
    assert!(swing.value > 0, "someone took a pile in one go");
    let downs = reel.iter().find(|one| one.highlight == Highlight::MostKnockouts).unwrap();
    assert_eq!(downs.seat, 1, "the one who kept hitting the floor");
    assert!(
        reel.iter().any(|one| one.highlight == Highlight::WorstDice),
        "and somebody had the worst of the dice"
    );
}

#[test]
fn the_storm_rolls_in_and_keeps_moving() {
    let mut board = Board::new(&seats(2), 4, 43);
    assert!(board.view().storm.is_none(), "the first round is calm");

    run_board_round(&mut board);
    board.finish_round(&[vec![0], vec![1]]);
    let first = board.view().storm.expect("it rolled in");
    assert_eq!(
        board.view().storm_to,
        Some(first_ahead(first)),
        "and you can see where it is going"
    );

    run_board_round(&mut board);
    board.finish_round(&[vec![0], vec![1]]);
    let second = board.view().storm.expect("still blowing");
    assert_ne!(second, first, "it moved on");
}

/// Where the storm goes next, walked the same way the board walks it.
fn first_ahead(from: u8) -> u8 {
    let mut at = from;
    for _ in 0..STORM_STEP {
        at = board::space(at).next.first().copied().unwrap_or(at);
    }
    at
}

#[test]
fn walking_into_the_storm_costs_you() {
    let mut board = Board::new(&seats(2), 4, 45);
    run_board_round(&mut board);
    board.finish_round(&[vec![0], vec![1]]);
    let storm = board.view().storm.expect("it rolled in");

    let seat = board.turn_seat();
    board.place_for_test(seat, storm, 30);
    board.land_for_test(seat);
    let after = token_of(&board, seat);
    let blown = board
        .view()
        .log
        .iter()
        .find(|entry| entry["kind"] == "blown")
        .cloned()
        .expect("it caught them");
    assert_eq!(blown["coins"], STORM_COINS, "it costs coins");
    assert_ne!(after.space, storm, "and blows you back out of it");
}

#[test]
fn a_cannon_throws_you_up_the_lap_give_or_take() {
    let mut landed = std::collections::HashSet::new();
    for seed in 0..30 {
        let mut board = Board::new(&seats(2), 3, seed);
        let seat = board.turn_seat();
        let cannon = board_space_of(SpaceKind::Cannon);
        board.place_for_test(seat, cannon, 30);
        board.land_for_test(seat);
        let TurnStep::Cannon { max } = board.step().clone() else {
            panic!("the cannon loaded, not {:?}", board.step());
        };
        assert_eq!(max, CANNON_RANGE);

        board.fire(seat, 8).unwrap();
        let shot = board
            .view()
            .log
            .iter()
            .find(|entry| entry["kind"] == "cannon")
            .cloned()
            .expect("it fired");
        let flew = shot["flew"].as_u64().unwrap() as u8;
        assert!(
            flew >= 8 - CANNON_SCATTER && flew <= 8 + CANNON_SCATTER,
            "it lands near where it was aimed, not {flew}"
        );
        landed.insert(flew);
        assert_ne!(token_of(&board, seat).space, cannon, "and you leave the barrel");
    }
    assert!(landed.len() > 1, "where it lands is not the same every time");
}

#[test]
fn a_well_timed_fuse_flies_true_and_a_botched_one_strays_further() {
    let mut strays = std::collections::HashSet::new();
    for seed in 0..30 {
        for (stop, most) in [(0.5, 0), (0.0, CANNON_SCATTER + 1)] {
            let mut board = Board::new(&seats(2), 3, seed);
            let seat = board.turn_seat();
            let cannon = board_space_of(SpaceKind::Cannon);
            board.place_for_test(seat, cannon, 30);
            board.land_for_test(seat);
            board.fire_timed(seat, 8, stop).unwrap();
            let shot = board.view().log.iter().find(|e| e["kind"] == "cannon").cloned().unwrap();
            let off = (shot["flew"].as_i64().unwrap() - 8).unsigned_abs() as u8;
            assert!(off <= most, "stop {stop} strayed {off}");
            if stop == 0.0 {
                strays.insert(off);
                assert_eq!(shot["quality"], "fail");
            } else {
                assert_eq!(shot["quality"], "perfect");
            }
        }
    }
    assert!(strays.contains(&(CANNON_SCATTER + 1)), "a botched fuse can overshoot past an untimed shot");
}

#[test]
fn a_cannon_will_not_fire_harder_than_it_can() {
    let mut board = Board::new(&seats(2), 3, 3);
    let seat = board.turn_seat();
    let cannon = board_space_of(SpaceKind::Cannon);
    board.place_for_test(seat, cannon, 30);
    board.land_for_test(seat);
    board.fire(seat, 250).unwrap();
    let flew = board
        .view()
        .log
        .iter()
        .find(|entry| entry["kind"] == "cannon")
        .map(|entry| entry["flew"].as_u64().unwrap() as u8)
        .unwrap();
    assert!(flew <= CANNON_RANGE + CANNON_SCATTER, "the charge is capped");
}

#[test]
fn the_hill_is_won_by_whoever_holds_the_top_longest() {
    use crate::bout::{Bout, Stick};
    let mut game = crate::hill::KingOfTheHill::new(&contestants(4, true), 7);
    let mut events = Vec::new();
    // Nobody is on the summit at the start, so nobody is scoring yet.
    let snap = game.snapshot();
    assert!(snap["players"].as_array().unwrap().iter().all(|p| p["held"].as_f64() == Some(0.0)));
    assert!(game.placements().is_none());

    // Seat 0 walks in while the rest stand still.
    for _ in 0..(60 * 61) {
        let stick = Stick { x: 0.0, z: 0.0 };
        let input = crate::bout::Input::from_stick(stick);
        let bot = game.bot_input(0, TICK_DT);
        game.step(&[(0, bot), (1, input)], TICK_DT, &mut events);
    }
    let placed = game.placements().expect("the clock ran out");
    assert_eq!(placed.iter().map(|group| group.len()).sum::<usize>(), 4, "everyone is placed");

    let snap = game.snapshot();
    let held: Vec<f64> = snap["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["held"].as_f64().unwrap_or(0.0))
        .collect();
    assert!(held.iter().any(|seconds| *seconds > 1.0), "somebody held the top: {held:?}");
    let winner = placed[0][0];
    let best = snap["players"]
        .as_array()
        .unwrap()
        .iter()
        .max_by(|a, b| a["held"].as_f64().unwrap().total_cmp(&b["held"].as_f64().unwrap()))
        .unwrap();
    assert_eq!(best["seat"].as_u64().unwrap() as u8, winner, "the longest hold wins");
}

#[test]
fn every_board_is_a_loop_that_leads_everywhere() {
    for map in board::Map::ALL {
        let layout = map.layout();
        assert!(layout.len() >= 30, "{map:?} is big enough to play on");
        assert_eq!(layout[0].kind, SpaceKind::Start, "{map:?} starts at the start");
        assert!(
            board::prize_spots_in(layout).len() >= 3,
            "{map:?} has somewhere for the prize to move to"
        );
        for space in layout {
            assert!(!space.next.is_empty(), "{map:?} space {} leads on", space.id);
            for next in &space.next {
                assert!((*next as usize) < layout.len(), "{map:?} space {} points inside the board", space.id);
            }
            // Every space can get back round to the start.
            assert!(
                board::distance_in(layout, space.id, 0) < u8::MAX,
                "{map:?} space {} can reach the start",
                space.id
            );
        }
    }
}

#[test]
fn a_party_on_the_volcano_plays_through() {
    let mut board = Board::on(board::Map::Volcano, &seats(4), 2, 9);
    assert_eq!(board.view().map, board::Map::Volcano);
    for _ in 0..2 {
        run_board_round(&mut board);
        board.finish_round(&[vec![0], vec![1], vec![2], vec![3]]);
    }
    assert!(board.view().tokens.iter().all(|token| token.stats.spaces > 0), "everyone got round");
}

#[test]
fn every_item_turns_up() {
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(7);
    let mut seen = std::collections::HashSet::new();
    for _ in 0..2000 {
        seen.insert(Item::draw(&mut rng));
    }
    assert_eq!(seen.len(), Item::ALL.len());
}

/// Face a fighter at a point, the way the sim reads yaw.
fn look_at(from: (f32, f32), to: (f32, f32)) -> f32 {
    (from.0 - to.0).atan2(from.1 - to.1)
}

fn stand(game: &mut PaintBlasters, seat: u8, x: f32, z: f32) {
    let fighter = game.fighters.iter_mut().find(|f| f.seat == seat).unwrap();
    fighter.x = x;
    fighter.z = z;
    fighter.y = 0.0;
    fighter.vx = 0.0;
    fighter.vz = 0.0;
}

fn shoot(game: &mut PaintBlasters, seat: u8, yaw: f32) -> Vec<Value> {
    let mut events = Vec::new();
    let input = Input {
        yaw,
        fire: true,
        ..Default::default()
    };
    game.step(&[(seat, input)], TICK_DT, &mut events);
    events
}

#[test]
fn a_clear_shot_splats_a_rival_and_cover_stops_one() {
    let mut game = PaintBlasters::new(&contestants(2, false), 1);
    stand(&mut game, 0, -12.0, 11.0);
    stand(&mut game, 1, 12.0, 11.0);
    let yaw = look_at((-12.0, 11.0), (12.0, 11.0));
    let events = shoot(&mut game, 0, yaw);
    let hit = game.fighters.iter().find(|f| f.seat == 1).unwrap();
    assert_eq!(hit.hp, MAX_HP - SHOT_DAMAGE, "the shot took health off");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "splat" && e["seat"] == 1),
        "a splat was reported"
    );

    // Now put the middle platform between them.
    let mut game = PaintBlasters::new(&contestants(2, false), 1);
    stand(&mut game, 0, -10.0, 0.0);
    stand(&mut game, 1, 10.0, 0.0);
    let yaw = look_at((-10.0, 0.0), (10.0, 0.0));
    let events = shoot(&mut game, 0, yaw);
    let safe = game.fighters.iter().find(|f| f.seat == 1).unwrap();
    assert_eq!(safe.hp, MAX_HP, "the platform took the shot");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "shot" && e["hit"] == false),
        "the miss was still reported"
    );
}

#[test]
fn it_takes_a_few_splats_to_take_someone_out_and_they_come_back() {
    let mut game = PaintBlasters::new(&contestants(2, false), 2);
    stand(&mut game, 0, -12.0, 11.0);
    stand(&mut game, 1, 12.0, 11.0);
    let yaw = look_at((-12.0, 11.0), (12.0, 11.0));
    let hits = (MAX_HP / SHOT_DAMAGE) as usize;
    assert!(hits >= 3, "a single hit never takes anyone out");
    let mut events = Vec::new();
    for _ in 0..hits {
        events.extend(shoot(&mut game, 0, yaw));
        // Wait out the trigger before the next one.
        for _ in 0..((FIRE_COOLDOWN / TICK_DT) as usize + 1) {
            game.step(
                &[(
                    0,
                    Input {
                        yaw,
                        ..Default::default()
                    },
                )],
                TICK_DT,
                &mut events,
            );
        }
    }
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "down" && e["seat"] == 1 && e["by"] == 0),
        "seat 1 went down"
    );
    let shooter = game.fighters.iter().find(|f| f.seat == 0).unwrap().clone();
    assert_eq!(shooter.kills, 1, "the splat was scored");
    let down = game.fighters.iter().find(|f| f.seat == 1).unwrap().clone();
    assert_eq!(down.deaths, 1);
    assert_eq!(down.hp, 0, "they are out until they come back");

    let mut events = Vec::new();
    for _ in 0..((RESPAWN_SECONDS / TICK_DT) as usize + 2) {
        game.step(&[], TICK_DT, &mut events);
    }
    let back = game.fighters.iter().find(|f| f.seat == 1).unwrap();
    assert_eq!(back.hp, MAX_HP, "they came back with full health");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "respawn" && e["seat"] == 1)
    );
}

#[test]
fn the_arena_keeps_everyone_inside_and_out_of_the_blocks() {
    let mut game = PaintBlasters::new(&contestants(2, false), 3);
    stand(&mut game, 0, 0.0, 10.0);
    let mut events = Vec::new();
    // Walk into the middle platform for a couple of seconds.
    let into_middle = Input {
        stick: Stick { x: 0.0, z: -1.0 },
        yaw: look_at((0.0, 10.0), (0.0, 0.0)),
        ..Default::default()
    };
    for _ in 0..120 {
        game.step(&[(0, into_middle)], TICK_DT, &mut events);
    }
    let stopped = game.fighters.iter().find(|f| f.seat == 0).unwrap().clone();
    assert!(
        stopped.z > 3.0,
        "the platform blocked the way, z was {}",
        stopped.z
    );

    // And now into the outer wall.
    stand(&mut game, 0, 0.0, 10.0);
    let into_wall = Input {
        stick: Stick { x: 0.0, z: -1.0 },
        yaw: look_at((0.0, 14.0), (0.0, 40.0)),
        ..Default::default()
    };
    for _ in 0..420 {
        game.step(&[(0, into_wall)], TICK_DT, &mut events);
    }
    let held = game.fighters.iter().find(|f| f.seat == 0).unwrap();
    assert!(held.z <= ARENA_HALF, "the wall held, z was {}", held.z);
}

#[test]
fn bots_shoot_it_out_and_the_round_ends_with_a_winner() {
    let mut game = PaintBlasters::new(&contestants(4, true), 7);
    let mut events = Vec::new();
    let mut steps = 0;
    while game.placements().is_none() && steps < (BLAST_SECONDS / TICK_DT) as usize + 120 {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    let placements = game.placements().expect("the round ended");
    assert_eq!(
        placements.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "splat"),
        "the bots hit each other"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "down"),
        "the bots took each other out"
    );
    let splats: u32 = game.fighters.iter().map(|f| f.kills).sum();
    assert!(splats >= 2, "a fight broke out, {splats} splats");
}

fn face(from: (f32, f32), to: (f32, f32)) -> f32 {
    (from.0 - to.0).atan2(from.1 - to.1)
}

fn place_ball(game: &mut Dodgeball, id: u8, x: f32, z: f32) {
    let ball = game.balls.iter_mut().find(|b| b.id == id).unwrap();
    ball.x = x;
    ball.z = z;
    ball.y = 0.33;
    ball.vx = 0.0;
    ball.vy = 0.0;
    ball.vz = 0.0;
    ball.state = DodgeBall::Idle;
}

fn stand_player(game: &mut Dodgeball, seat: u8, x: f32, z: f32, yaw: f32) {
    let player = game.players.iter_mut().find(|p| p.seat == seat).unwrap();
    player.body.x = x;
    player.body.z = z;
    player.body.y = 0.0;
    player.body.vx = 0.0;
    player.body.vz = 0.0;
    player.yaw = yaw;
}

fn hold(game: &mut Dodgeball, seat: u8, id: u8) {
    game.balls.iter_mut().find(|b| b.id == id).unwrap().state = DodgeBall::Held(seat);
}

fn idle(game: &mut Dodgeball, seconds: f32, inputs: &[(u8, Input)]) -> Vec<Value> {
    let mut events = Vec::new();
    for _ in 0..(seconds / TICK_DT) as usize {
        game.step(inputs, TICK_DT, &mut events);
    }
    events
}

#[test]
fn walking_over_a_ball_picks_it_up_and_a_throw_knocks_a_rival_out() {
    let mut game = Dodgeball::new(&contestants(2, false), 1);
    stand_player(&mut game, 0, 0.0, 6.0, face((0.0, 6.0), (0.0, -6.0)));
    stand_player(&mut game, 1, 0.0, -6.0, face((0.0, -6.0), (0.0, 6.0)));
    place_ball(&mut game, 0, 0.0, 5.6);
    let picked = idle(&mut game, 0.1, &[]);
    assert!(
        picked
            .iter()
            .any(|e| e["kind"] == "pickup" && e["seat"] == 0),
        "we picked the ball up"
    );

    let yaw = face((0.0, 6.0), (0.0, -6.0));
    let throw = Input {
        yaw,
        fire: true,
        ..Default::default()
    };
    let mut events = Vec::new();
    game.step(&[(0, throw)], TICK_DT, &mut events);
    assert!(
        events.iter().any(|e| e["kind"] == "throw"),
        "the ball went out of our hands"
    );
    events.extend(idle(
        &mut game,
        1.0,
        &[(
            0,
            Input {
                yaw,
                ..Default::default()
            },
        )],
    ));
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "out" && e["seat"] == 1 && e["by"] == 0),
        "the throw knocked seat 1 out"
    );
    let thrower = game.players.iter().find(|p| p.seat == 0).unwrap();
    assert_eq!(thrower.hits, 1, "the hit was scored");
    let hit = game.players.iter().find(|p| p.seat == 1).unwrap();
    assert!(!hit.playing(), "they are off the court");

    let back = idle(&mut game, OUT_SECONDS + 0.2, &[]);
    assert!(
        back.iter().any(|e| e["kind"] == "back" && e["seat"] == 1),
        "they came back on"
    );
}

#[test]
fn catching_a_throw_sends_the_thrower_off_instead() {
    let mut game = Dodgeball::new(&contestants(2, false), 2);
    stand_player(&mut game, 0, 0.0, 7.0, face((0.0, 7.0), (0.0, -7.0)));
    stand_player(&mut game, 1, 0.0, -7.0, face((0.0, -7.0), (0.0, 7.0)));
    hold(&mut game, 0, 0);
    let out_yaw = face((0.0, 7.0), (0.0, -7.0));
    let back_yaw = face((0.0, -7.0), (0.0, 7.0));
    let mut events = Vec::new();
    game.step(
        &[(
            0,
            Input {
                yaw: out_yaw,
                fire: true,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );

    // Reach for it as it arrives.
    let mut caught = false;
    for _ in 0..90 {
        let close = game
            .balls
            .iter()
            .any(|b| matches!(b.state, DodgeBall::Live(_)) && (b.z + 7.0).abs() < 2.4);
        let input = Input {
            yaw: back_yaw,
            fire: close,
            ..Default::default()
        };
        game.step(&[(1, input)], TICK_DT, &mut events);
        if events.iter().any(|e| e["kind"] == "catch") {
            caught = true;
            break;
        }
    }
    assert!(caught, "seat 1 caught the throw");
    let catcher = game.players.iter().find(|p| p.seat == 1).unwrap();
    assert_eq!(catcher.hits, 1, "the catch counted");
    let thrower = game.players.iter().find(|p| p.seat == 0).unwrap();
    assert!(!thrower.playing(), "the thrower is off the court");
}

#[test]
fn balls_start_on_the_centre_line_and_settle_on_the_floor() {
    let game = Dodgeball::new(&contestants(2, false), 3);
    assert!(game.balls.len() >= 3, "there are balls to fight over");
    assert!(
        game.balls.iter().all(|b| b.z.abs() < 0.01),
        "they sit on the centre line"
    );
    assert!(
        game.players.iter().any(|p| p.body.z > COURT_HALF_Z - 4.0)
            && game
                .players
                .iter()
                .any(|p| p.body.z < -(COURT_HALF_Z - 4.0)),
        "the two sides face off"
    );
}

#[test]
fn bots_play_a_whole_game_of_dodgeball_and_someone_wins() {
    let mut game = Dodgeball::new(&contestants(4, true), 11);
    let mut events = Vec::new();
    let mut steps = 0;
    while game.placements().is_none() && steps < (BALL_SECONDS / TICK_DT) as usize + 120 {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    let placements = game.placements().expect("the round ended");
    assert_eq!(
        placements.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "throw"),
        "the bots threw balls"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "out"),
        "the bots knocked each other out"
    );
    let hits: u32 = game.players.iter().map(|p| p.hits).sum();
    assert!(hits >= 2, "a game broke out, {hits} hits");
}

#[test]
fn alternating_pumps_drive_the_rocker_and_boost_pads_reward_the_lane() {
    let mut game = RockerRally::new(&contestants(1, false), 5);
    let mut events = Vec::new();
    let mut side = -1.0;
    for tick in 0..180 {
        if tick % 9 == 0 {
            side = -side;
        }
        game.step(
            &[(0, Input::from_stick(Stick { x: 0.0, z: side }))],
            TICK_DT,
            &mut events,
        );
    }
    assert!(
        game.racers[0].z > START_Z + 12.0,
        "alternating pumps moves the racer"
    );
    assert!(
        events.iter().any(|event| event["kind"] == "pump"),
        "pumps are reported to clients"
    );

    let (x, z) = BOOSTS[0];
    let racer = &mut game.racers[0];
    racer.x = x;
    racer.z = z - 0.2;
    racer.speed = 5.0;
    game.step(&[(0, Input::default())], TICK_DT, &mut events);
    assert!(
        events.iter().any(|event| event["kind"] == "boost"),
        "hitting a pad makes a boost event"
    );
    assert!(
        game.racers[0].speed > 7.0,
        "the boost adds a meaningful burst of speed"
    );
}

#[test]
fn rocker_rally_recovers_when_a_player_misses_the_rhythm() {
    let mut game = RockerRally::new(&contestants(1, false), 23);
    let mut events = Vec::new();
    // Establish the left/right rhythm and earn one pump.
    game.step(
        &[(0, Input::from_stick(Stick { x: 0.0, z: -1.0 }))],
        TICK_DT,
        &mut events,
    );
    for _ in 0..9 {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(
        &[(0, Input::from_stick(Stick { x: 0.0, z: 1.0 }))],
        TICK_DT,
        &mut events,
    );
    let before_pause = game.racers[0].speed;
    assert!(before_pause > 0.0, "the first rhythm gave speed");

    // A long pause used to leave last_pump stale, rejecting every future tap.
    for _ in 0..60 {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(
        &[(0, Input::from_stick(Stick { x: 0.0, z: -1.0 }))],
        TICK_DT,
        &mut events,
    );
    for _ in 0..9 {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(
        &[(0, Input::from_stick(Stick { x: 0.0, z: 1.0 }))],
        TICK_DT,
        &mut events,
    );
    assert!(
        game.racers[0].speed > before_pause,
        "a new rhythm works after a missed beat"
    );
}

#[test]
fn rocker_rally_bots_finish_and_rank_every_racer() {
    let mut game = RockerRally::new(&contestants(4, true), 19);
    let mut events = Vec::new();
    for _ in 0..((RACE_SECONDS / TICK_DT) as usize + 90) {
        let inputs: Vec<(u8, Input)> = (0..4)
            .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    let placements = game.placements().expect("the race ends");
    assert_eq!(
        placements.iter().flatten().count(),
        4,
        "every racer has a place"
    );
    assert!(
        events.iter().any(|event| event["kind"] == "finish"),
        "someone crossed the line at {FINISH_Z}"
    );
}

#[test]
fn red_light_rush_bots_finish_and_rank_the_field() {
    let mut game = RedLightRush::new(&contestants(4, true), 43);
    let mut events = Vec::new();
    for _ in 0..((55.0 / TICK_DT) as usize + 120) {
        let inputs: Vec<(u8, Input)> = (0..4)
            .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    assert_eq!(
        game.placements()
            .expect("the rush ends")
            .iter()
            .flatten()
            .count(),
        4
    );
}

#[test]
fn target_frenzy_bots_hit_real_targets_and_score_them() {
    let mut game = PaintPlaylist::new(PaintVariant::TargetFrenzy, &contestants(3, true), 53);
    let opening = game.snapshot();
    let opening_positions: Vec<_> = opening["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|player| {
            (
                player["seat"].as_u64().unwrap(),
                player["x"].as_f64().unwrap(),
                player["z"].as_f64().unwrap(),
            )
        })
        .collect();
    let active: Vec<_> = opening["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|target| target["up"] == true)
        .collect();
    assert_eq!(
        active.len(),
        3,
        "each player sees the same prompt in their own lane"
    );
    let mut owners: Vec<_> = active
        .iter()
        .map(|target| target["owner"].as_u64().unwrap())
        .collect();
    owners.sort_unstable();
    assert_eq!(
        owners,
        vec![0, 1, 2],
        "every lane has exactly one active target"
    );
    let mut events = Vec::new();
    for _ in 0..1_500 {
        let inputs: Vec<(u8, Input)> = (0..3)
            .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
    }
    assert!(
        events.iter().any(|event| event["kind"] == "target_hit"),
        "bots can hit the target props"
    );
    assert!(
        !events.iter().any(|event| event["kind"] == "down"),
        "Target Frenzy shots never damage another player"
    );
    assert!(
        events
            .iter()
            .filter(|event| event["kind"] == "target_hit")
            .all(|event| event["points"].as_u64().unwrap_or(0) > 0),
        "each claimed light awards reaction points"
    );
    let ending = game.snapshot();
    let ending_positions: Vec<_> = ending["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|player| {
            (
                player["seat"].as_u64().unwrap(),
                player["x"].as_f64().unwrap(),
                player["z"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ending_positions, opening_positions,
        "players stay locked in their firing booths"
    );
    let total = ending["scores"]
        .as_object()
        .unwrap()
        .values()
        .filter_map(|score| score.as_u64())
        .sum::<u64>();
    assert!(total > 0, "target hits become the round score");
}

#[test]
fn popgun_knockout_is_a_one_hit_no_respawn_round() {
    let mut game = PaintPlaylist::new(PaintVariant::PopgunKnockout, &contestants(3, true), 59);
    let mut events = Vec::new();
    for _ in 0..1_500 {
        let inputs: Vec<(u8, Input)> = (0..3)
            .map(|seat| (seat, game.bot_input(seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    assert!(
        events.iter().any(|event| event["kind"] == "down"),
        "a direct Popgun hit knocks a player out"
    );
    assert!(
        !events.iter().any(|event| event["kind"] == "respawn"),
        "knocked-out Popgun players do not respawn"
    );
    assert_eq!(game.snapshot()["limit"], 45.0);
    assert_eq!(
        game.placements()
            .expect("the knockout round ends")
            .iter()
            .flatten()
            .count(),
        3
    );
}

#[test]
fn site_retake_assigns_teams_and_resolves_the_objective_round() {
    let seats = contestants(4, true);
    let mut game = PaintPlaylist::new(PaintVariant::SiteRetake, &seats, 67);
    let opening = game.snapshot();
    let teams: Vec<_> = opening["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|player| player["team"].as_u64().unwrap())
        .collect();
    assert_eq!(teams.iter().filter(|&&team| team == 0).count(), 2);
    assert_eq!(teams.iter().filter(|&&team| team == 1).count(), 2);
    assert_eq!(opening["objective"]["active"], true);
    let mut events = Vec::new();
    for _ in 0..4_000 {
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        if game.placements().is_some() {
            break;
        }
    }
    assert!(
        !events.iter().any(|event| event["kind"] == "respawn"),
        "retake eliminations last for the round"
    );
    assert_eq!(
        game.placements()
            .expect("retake reaches a team result")
            .iter()
            .flatten()
            .count(),
        4
    );
}

#[test]
fn every_registered_minigame_has_a_live_bot_simulation_and_result() {
    // This is intentionally registry-driven: adding a menu item without a
    // game factory, a valid snapshot, bot controls, or a terminating round
    // fails one test instead of silently shipping a dead selection.
    assert_eq!(
        BOUTS.len(),
        62,
        "the registry holds every implemented bout"
    );
    for (index, key) in BOUTS.iter().map(|spec| spec.key).enumerate() {
        let seats = contestants(4, true);
        let mut game =
            create(key, &seats, 900 + index as u64).expect("registered game has a factory");
        assert_eq!(game.key(), key);
        assert_eq!(
            game.snapshot()["game"],
            key,
            "{key} reports its own snapshot kind"
        );
        let mut events = Vec::new();
        // Paint playlists have a ninety-second timer; this leaves margin for
        // the fixed simulation step as well as the survival modes' endgame.
        for _ in 0..6_000 {
            let inputs: Vec<(u8, Input)> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            if game.placements().is_some() {
                break;
            }
        }
        let placements = game
            .placements()
            .unwrap_or_else(|| panic!("{key} did not complete under bot simulation"));
        assert_eq!(
            placements.iter().flatten().count(),
            seats.len(),
            "{key} ranks every seat"
        );
    }
}

fn retake(count: u8) -> Box<dyn Bout> {
    crate::bout::create("retake", &contestants(count, true), 5).unwrap()
}

/// Run the round with nobody pressing anything.
fn tick(game: &mut dyn Bout, seconds: f32, events: &mut Vec<Value>) {
    for _ in 0..(seconds / TICK_DT) as usize {
        if game.placements().is_some() {
            return;
        }
        game.step(&[], TICK_DT, events);
    }
}

#[test]
fn the_bomb_goes_off_on_its_own_and_hands_the_round_to_the_side_defending_it() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(2, false),
        5,
    );
    let mut events = Vec::new();
    tick(&mut game, RETAKE_FUSE - 1.0, &mut events);
    assert!(
        game.placements().is_none(),
        "the round runs while the fuse burns"
    );
    assert_eq!(game.snapshot()["objective"]["state"], "ticking");

    tick(&mut game, 2.0, &mut events);
    assert!(
        events.iter().any(|e| e["kind"] == "exploded"),
        "the bomb went off"
    );
    assert_eq!(game.snapshot()["objective"]["state"], "exploded");
    let placements = game.placements().expect("the round ended");
    let defending: Vec<u8> = (0..2)
        .filter(|s| game.team_for_test(*s) == Some(0))
        .collect();
    assert_eq!(placements[0], defending, "the side on the bomb won");
}

#[test]
fn standing_on_the_bomb_long_enough_defuses_it_and_wins_the_retake() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(2, false),
        5,
    );
    let retaker = (0..2).find(|s| game.team_for_test(*s) == Some(1)).unwrap();
    game.place_for_test(retaker, RETAKE_BOMB.0, RETAKE_BOMB.1);
    let mut events = Vec::new();
    tick(&mut game, RETAKE_DEFUSE - 1.0, &mut events);
    assert!(
        events.iter().any(|e| e["kind"] == "defuse_start"),
        "the defuse started"
    );
    assert!(
        game.placements().is_none(),
        "it takes a moment to cut the wire"
    );

    tick(&mut game, 1.5, &mut events);
    assert!(
        events.iter().any(|e| e["kind"] == "defused"),
        "the bomb was defused"
    );
    let placements = game.placements().expect("the round ended");
    assert!(placements[0].contains(&retaker), "the retaking side won");
}

#[test]
fn stepping_off_the_bomb_puts_the_wire_back() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(2, false),
        5,
    );
    let retaker = (0..2).find(|s| game.team_for_test(*s) == Some(1)).unwrap();
    game.place_for_test(retaker, RETAKE_BOMB.0, RETAKE_BOMB.1);
    let mut events = Vec::new();
    tick(&mut game, 2.0, &mut events);
    assert!(
        game.defuse_progress_for_test() > 1.0,
        "the defuse was under way"
    );

    game.place_for_test(retaker, RETAKE_BOMB.0, RETAKE_BOMB.1 + 12.0);
    tick(&mut game, 0.2, &mut events);
    assert_eq!(
        game.defuse_progress_for_test(),
        0.0,
        "walking away resets it"
    );
    assert!(events.iter().any(|e| e["kind"] == "defuse_stop"));
}

#[test]
fn clearing_the_site_does_not_win_the_retake_by_itself() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(2, false),
        5,
    );
    let defender = (0..2).find(|s| game.team_for_test(*s) == Some(0)).unwrap();
    game.drop_for_test(defender);
    let mut events = Vec::new();
    tick(&mut game, 3.0, &mut events);
    assert!(
        game.placements().is_none(),
        "the bomb is still ticking with nobody guarding it"
    );

    // Only the defuse finishes it.
    let retaker = (0..2).find(|s| game.team_for_test(*s) == Some(1)).unwrap();
    game.place_for_test(retaker, RETAKE_BOMB.0, RETAKE_BOMB.1);
    tick(&mut game, RETAKE_DEFUSE + 0.5, &mut events);
    assert!(events.iter().any(|e| e["kind"] == "defused"));
}

#[test]
fn losing_everyone_who_could_defuse_ends_the_retake() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(2, false),
        5,
    );
    let retaker = (0..2).find(|s| game.team_for_test(*s) == Some(1)).unwrap();
    game.drop_for_test(retaker);
    let mut events = Vec::new();
    tick(&mut game, 0.2, &mut events);
    let placements = game.placements().expect("the round ended");
    assert!(
        !placements[0].contains(&retaker),
        "the side on the bomb won"
    );
}

#[test]
fn playing_the_retake_alone_still_gives_you_a_live_bomb() {
    let mut game = crate::paint_variant::PaintPlaylist::new(
        crate::paint_variant::PaintVariant::SiteRetake,
        &contestants(1, false),
        5,
    );
    assert_eq!(game.team_for_test(0), Some(1), "on your own you retake");
    let mut events = Vec::new();
    tick(&mut game, 5.0, &mut events);
    assert!(
        game.placements().is_none(),
        "the round does not end on the first tick"
    );
    let fuse = game.snapshot()["objective"]["fuse"]
        .as_f64()
        .unwrap_or_default();
    assert!(
        fuse < RETAKE_FUSE as f64 && fuse > 0.0,
        "the fuse is burning, {fuse}"
    );
}

#[test]
fn bots_retake_the_site_or_lose_it_to_the_bomb() {
    let mut game = retake(4);
    let mut events = Vec::new();
    let mut steps = 0;
    while game.placements().is_none() && steps < ((RETAKE_FUSE + 5.0) / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    let placements = game
        .placements()
        .expect("the round ended one way or the other");
    assert_eq!(
        placements.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "defused"
            || e["kind"] == "exploded"
            || e["kind"] == "retake_failed"),
        "the round ended on the objective"
    );
}

#[test]
fn crouching_slows_you_down_and_ducks_you_under_a_shot() {
    let mut game = PaintBlasters::new(&contestants(2, false), 3);
    let walk = Input {
        stick: Stick { x: 0.0, z: -1.0 },
        ..Default::default()
    };
    let sneak = Input {
        crouch: true,
        ..walk
    };
    let mut events = Vec::new();

    // A clear lane down the side of the arena, away from the cover.
    let lane = look_at((-17.0, -10.0), (-17.0, 0.0));
    let walk = Input { yaw: lane, ..walk };
    let sneak = Input { yaw: lane, ..sneak };
    stand(&mut game, 0, -17.0, -10.0);
    for _ in 0..90 {
        game.step(&[(0, walk)], TICK_DT, &mut events);
    }
    let ran = game.fighters.iter().find(|f| f.seat == 0).unwrap().z + 10.0;

    stand(&mut game, 0, -17.0, -10.0);
    for _ in 0..90 {
        game.step(&[(0, sneak)], TICK_DT, &mut events);
    }
    let crept = game.fighters.iter().find(|f| f.seat == 0).unwrap().z + 10.0;
    assert!(
        crept < ran * 0.6,
        "crouching is slower: {crept} against {ran}"
    );

    // A shot at chest height goes over a crouching head.
    stand(&mut game, 0, -12.0, 11.0);
    stand(&mut game, 1, 12.0, 11.0);
    let yaw = look_at((-12.0, 11.0), (12.0, 11.0));
    let aim = Input {
        yaw,
        fire: true,
        ..Default::default()
    };
    let duck = Input {
        crouch: true,
        ..Default::default()
    };
    // Hold the crouch for a moment before the shot comes in.
    for _ in 0..10 {
        game.step(&[(1, duck)], TICK_DT, &mut events);
    }
    game.step(&[(0, aim), (1, duck)], TICK_DT, &mut events);
    let ducked = game.fighters.iter().find(|f| f.seat == 1).unwrap().hp;
    assert_eq!(ducked, MAX_HP, "the shot passed over a crouching player");

    stand(&mut game, 1, 12.0, 11.0);
    for _ in 0..((FIRE_COOLDOWN / TICK_DT) as usize + 1) {
        game.step(
            &[(
                0,
                Input {
                    yaw,
                    ..Default::default()
                },
            )],
            TICK_DT,
            &mut events,
        );
    }
    game.step(&[(0, aim)], TICK_DT, &mut events);
    let standing = game.fighters.iter().find(|f| f.seat == 1).unwrap().hp;
    assert!(standing < MAX_HP, "standing up, the same shot lands");
}

fn run_bots(game: &mut dyn Bout, count: u8, seconds: f32, events: &mut Vec<Value>) -> f32 {
    let mut steps = 0usize;
    let cap = (seconds / TICK_DT) as usize;
    while game.placements().is_none() && steps < cap {
        let inputs: Vec<(u8, Input)> = (0..count)
            .map(|s| (s, game.bot_input(s, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, events);
        steps += 1;
    }
    steps as f32 * TICK_DT
}

#[test]
fn crumble_tiles_give_way_under_you_and_drop_you_through() {
    use crate::crumble::{CRACK_SECONDS, CrumbleRun, DWELL_SECONDS, FALL_SECONDS, TileState};
    let mut game = CrumbleRun::new(&contestants(2, false), 4);
    let (x, z) = (game.players[0].run.x, game.players[0].run.z);
    let mut events = Vec::new();
    for _ in 0..((DWELL_SECONDS + 0.1) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(
        game.tile_state(x, z),
        Some(TileState::Cracking),
        "standing still cracks the tile"
    );
    assert!(events.iter().any(|e| e["kind"] == "crack"));

    // Stay put and the floor goes, taking you with it.
    for _ in 0..((CRACK_SECONDS + FALL_SECONDS + 1.0) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(
        matches!(
            game.tile_state(x, z),
            Some(TileState::Falling) | Some(TileState::Gone)
        ),
        "the tile fell away"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "drop" && e["seat"] == 0),
        "we went through it"
    );
}

#[test]
fn crumble_bots_keep_moving_and_the_round_ends_with_a_survivor() {
    use crate::crumble::CrumbleRun;
    let mut game = CrumbleRun::new(&contestants(4, true), 9);
    let mut events = Vec::new();
    let ended = run_bots(&mut game, 4, 90.0, &mut events);
    let placements = game.placements().expect("the floor ran out eventually");
    assert_eq!(
        placements.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    assert!(
        ended > 4.0,
        "bots lasted longer than a few seconds, {ended}"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "tile_fall"),
        "the floor came apart"
    );
}

#[test]
fn the_hot_potato_passes_on_a_touch_and_takes_out_whoever_holds_it() {
    use crate::potato::{HotPotato, PASS_LOCK};
    let mut game = HotPotato::new(&contestants(3, false), 6);
    let holder = game.holder_seat().expect("someone starts with the bomb");
    let mark = game
        .players
        .iter()
        .map(|p| p.seat)
        .find(|s| *s != holder)
        .unwrap();
    let mut events = Vec::new();
    // Stand them on top of each other once the pass lock clears.
    for _ in 0..((PASS_LOCK + 0.1) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    let at = game
        .players
        .iter()
        .find(|p| p.seat == holder)
        .map(|p| (p.run.x, p.run.z))
        .unwrap();
    if let Some(target) = game.players.iter_mut().find(|p| p.seat == mark) {
        target.run.x = at.0 + 0.6;
        target.run.z = at.1;
    }
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.holder_seat(), Some(mark), "the bomb changed hands");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "pass" && e["to"] == mark)
    );

    // Let the fuse run down on the new holder.
    game.fuse = 0.05;
    for _ in 0..6 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "boom" && e["seat"] == mark),
        "it went off on them"
    );
    let out = game.players.iter().find(|p| p.seat == mark).unwrap();
    assert!(!out.playing(), "they are out of the game");
}

#[test]
fn a_balloon_dash_pops_one_balloon_and_only_one() {
    use crate::balloon::{BALLOONS, BalloonBrawl};
    let mut game = BalloonBrawl::new(&contestants(2, false), 8);
    game.players[0].run = crate::ground::Runner::at(0.0, 0.0);
    game.players[1].run = crate::ground::Runner::at(0.0, 1.6);
    let mut events = Vec::new();
    let dash = Input {
        stick: Stick { x: 0.0, z: 1.0 },
        fire: true,
        ..Default::default()
    };
    game.step(&[(0, dash)], TICK_DT, &mut events);
    game.step(
        &[(
            0,
            Input {
                fire: false,
                ..dash
            },
        )],
        TICK_DT,
        &mut events,
    );
    assert!(
        events.iter().any(|e| e["kind"] == "pop" && e["seat"] == 1),
        "the dash burst a balloon"
    );
    assert_eq!(game.players[1].balloons, BALLOONS - 1);

    // The safe window stops the same dash taking a second one.
    for _ in 0..6 {
        game.step(
            &[(
                0,
                Input {
                    fire: false,
                    ..dash
                },
            )],
            TICK_DT,
            &mut events,
        );
    }
    assert_eq!(
        game.players[1].balloons,
        BALLOONS - 1,
        "one dash, one balloon"
    );
}

#[test]
fn balloon_bots_brawl_until_someone_is_left_holding_balloons() {
    use crate::balloon::BalloonBrawl;
    let mut game = BalloonBrawl::new(&contestants(4, true), 12);
    let mut events = Vec::new();
    run_bots(&mut game, 4, 90.0, &mut events);
    let placements = game.placements().expect("the brawl finished");
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
    assert!(events.iter().any(|e| e["kind"] == "pop"), "balloons went");
}

/// Shove a standing rival with `speed` behind you. Reports how fast they are
/// sent away, how long they lose their footing for, what the shover has left,
/// and the speed the two of them actually met at.
fn sumo_shove_at(speed: f32) -> (f32, f32, f32, f32) {
    use crate::sumo::Sumo;
    let mut game = Sumo::new(&contestants(2, false), 14);
    game.players[0].run = crate::ground::Runner::at(0.0, -1.0);
    game.players[0].run.vz = speed;
    game.players[1].run = crate::ground::Runner::at(0.0, 0.0);
    let mut events = Vec::new();
    game.step(&[], TICK_DT, &mut events);
    let hit = events
        .iter()
        .find(|e| e["kind"] == "bump" && e["a"] == 0 && e["b"] == 1)
        .unwrap_or_else(|| panic!("running in at {speed} made contact"));
    let closing = hit["strength"].as_f64().unwrap() as f32 * crate::ground::RUN_SPEED * 1.5;
    (
        game.players[1].run.vz,
        game.players[1].stagger,
        game.players[0].run.speed(),
        closing,
    )
}

#[test]
fn a_sumo_shove_carries_the_speed_you_ran_in_with() {
    let (fast, stagger, shover, closing) = sumo_shove_at(8.0);
    assert!(
        fast > closing,
        "a rival at a standstill takes the whole hit, {fast} out of {closing}"
    );
    assert!(stagger > 0.0, "and loses their footing for a moment");
    assert!(shover < 1.5, "the shove stops the fighter who landed it");

    let (slow, _, _, _) = sumo_shove_at(2.5);
    assert!(
        fast > slow * 2.0,
        "running in faster shoves harder: {fast} against {slow}"
    );
}

#[test]
fn the_shrinking_platform_puts_out_whoever_stands_too_far_out() {
    use crate::sumo::{ARENA_RADIUS, Sumo};
    let mut game = Sumo::new(&contestants(2, false), 15);
    game.players[0].run = crate::ground::Runner::at(0.0, 0.0);
    game.players[1].run = crate::ground::Runner::at(0.0, ARENA_RADIUS * 0.75);
    let mut events = Vec::new();
    tick(&mut game, 20.0, &mut events);
    assert!(
        game.players.iter().all(|p| p.standing()),
        "there is room for everyone to start with"
    );
    assert!(game.radius < ARENA_RADIUS, "the platform is closing in");

    tick(&mut game, 25.0, &mut events);
    assert!(
        events.iter().any(|e| e["kind"] == "over" && e["seat"] == 1),
        "the rim passed the fighter standing out wide"
    );
    assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
}

#[test]
fn a_bigger_field_gets_a_wider_platform() {
    use crate::sumo::{ARENA_RADIUS, Sumo};
    assert_eq!(Sumo::new(&contestants(4, false), 1).radius, ARENA_RADIUS, "four play it as built");
    let eight = Sumo::new(&contestants(8, false), 1);
    assert!((eight.radius / ARENA_RADIUS - 2f32.sqrt()).abs() < 1e-4, "eight get the room of two fours");
    assert_eq!(eight.setup()["radius"], serde_json::json!(eight.radius), "and the client is told");
    assert_eq!(crate::bout::crowd_count(3, 8), 6);
    assert_eq!(crate::bout::crowd_count(3, 2), 3, "never fewer than built");
}

#[test]
fn sumo_bots_shove_each_other_off_and_leave_one_standing() {
    use crate::sumo::{ROUND_SECONDS, Sumo};
    let mut game = Sumo::new(&contestants(4, true), 17);
    let mut events = Vec::new();
    let ended = run_bots(&mut game, 4, ROUND_SECONDS + 5.0, &mut events);
    let placements = game.placements().expect("the round ended");
    assert_eq!(
        placements.first().map(|g| g.len()),
        Some(1),
        "one fighter is left on the platform"
    );
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
    assert!(
        events.iter().any(|e| e["kind"] == "bump"),
        "they went after each other"
    );
    assert_eq!(
        events.iter().filter(|e| e["kind"] == "over").count(),
        3,
        "three of them went over the edge"
    );
    assert!(
        (30.0..ROUND_SECONDS).contains(&ended),
        "the round took a while but beat the clock, {ended}"
    );
}

#[test]
fn a_tank_shell_wrecks_a_rival_and_bounces_off_the_walls() {
    use crate::tank::{TankClash, YARD_HALF};
    let mut game = TankClash::new(&contestants(2, false), 5);
    // Face them off down a clear lane with nothing in between.
    game.tanks[0].x = -8.0;
    game.tanks[0].z = 19.0;
    game.tanks[1].x = 8.0;
    game.tanks[1].z = 19.0;
    let yaw = (game.tanks[0].x - game.tanks[1].x).atan2(game.tanks[0].z - game.tanks[1].z);
    let mut events = Vec::new();
    game.step(
        &[(
            0,
            Input {
                yaw,
                fire: true,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );
    assert!(
        events.iter().any(|e| e["kind"] == "shell"),
        "the gun went off"
    );
    for _ in 0..90 {
        game.step(
            &[(
                0,
                Input {
                    yaw,
                    ..Default::default()
                },
            )],
            TICK_DT,
            &mut events,
        );
    }
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "wreck" && e["seat"] == 1 && e["by"] == 0),
        "the shell found them"
    );
    assert_eq!(game.tanks[0].kills, 1);
    assert!(!game.tanks[1].rolling(), "the wreck sits out for a moment");

    // A shell into a wall comes back rather than stopping.
    let mut game = TankClash::new(&contestants(2, false), 5);
    game.tanks[0].x = 0.0;
    game.tanks[0].z = YARD_HALF - 4.0;
    let into_wall = (0.0f32 - 0.0).atan2(game.tanks[0].z - (YARD_HALF + 5.0));
    let mut events = Vec::new();
    game.step(
        &[(
            0,
            Input {
                yaw: into_wall,
                fire: true,
                ..Default::default()
            },
        )],
        TICK_DT,
        &mut events,
    );
    for _ in 0..40 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(events.iter().any(|e| e["kind"] == "ricochet"), "it bounced");
}

#[test]
fn tank_bots_trade_shots_and_the_round_is_scored_on_wrecks() {
    use crate::tank::{ROUND_SECONDS as TANK_SECONDS, TankClash};
    let mut game = TankClash::new(&contestants(4, true), 17);
    let mut events = Vec::new();
    run_bots(&mut game, 4, TANK_SECONDS + 3.0, &mut events);
    let placements = game.placements().expect("the horn went");
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
    assert!(events.iter().any(|e| e["kind"] == "shell"), "they fired");
    assert!(
        events.iter().any(|e| e["kind"] == "wreck"),
        "somebody got hit"
    );
}

#[test]
fn the_titan_only_takes_hits_while_it_is_winded() {
    use crate::titan::{CORE_RANGE, TITAN_HEALTH, TITAN_RADIUS, TitanSiege};
    let mut game = TitanSiege::new(&contestants(2, false), 7);
    game.raiders[0].run = crate::ground::Runner::at(0.0, TITAN_RADIUS + CORE_RANGE - 1.0);
    let swing = Input {
        fire: true,
        ..Default::default()
    };
    let mut events = Vec::new();
    game.step(&[(0, swing)], TICK_DT, &mut events);
    assert_eq!(
        game.health, TITAN_HEALTH,
        "swinging at a guarded titan does nothing"
    );
    assert!(events.iter().any(|e| e["kind"] == "swing"));

    // Wait for it to swing and be caught out, then hit the core.
    let mut steps = 0;
    while !game.open() && steps < (20.0 / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
        steps += 1;
    }
    assert!(game.open(), "the titan wound up and got winded");
    game.raiders[0].run = crate::ground::Runner::at(game.x, game.z + TITAN_RADIUS + 1.0);
    game.raiders[0].cooldown = 0.0;
    // They may have been floored while waiting for the opening.
    game.raiders[0].down_for = 0.0;
    // A swing off the core's throb glances off.
    while game.on_beat() {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(&[(0, swing)], TICK_DT, &mut events);
    assert_eq!(game.health, TITAN_HEALTH, "an off-beat swing glances off");
    assert!(events.iter().any(|e| e["kind"] == "glance"));
    game.raiders[0].cooldown = 0.0;
    while !game.on_beat() {
        game.step(&[], TICK_DT, &mut events);
    }
    game.raiders[0].run = crate::ground::Runner::at(game.x, game.z + TITAN_RADIUS + 1.0);
    game.raiders[0].down_for = 0.0;
    game.step(&[(0, swing)], TICK_DT, &mut events);
    assert_eq!(
        game.health,
        TITAN_HEALTH - crate::titan::CRITICAL_DAMAGE,
        "a strike on the throb is a critical"
    );
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "strike_core" && e["seat"] == 0)
    );
    assert_eq!(game.raiders[0].hits, 1);
}

#[test]
fn the_titan_floors_anyone_who_stands_in_its_attack() {
    use crate::titan::TitanSiege;
    let mut game = TitanSiege::new(&contestants(2, false), 3);
    game.raiders[0].run = crate::ground::Runner::at(0.0, 4.0);
    game.raiders[1].run = crate::ground::Runner::at(0.0, 5.0);
    let mut events: Vec<Value> = Vec::new();
    let mut steps = 0;
    while !events.iter().any(|e| e["kind"] == "floored") && steps < (25.0 / TICK_DT) as usize {
        // Stand still right next to it and take whatever comes.
        game.raiders[0].run.x = 0.0;
        game.raiders[0].run.z = game.z + 4.0;
        game.step(&[], TICK_DT, &mut events);
        steps += 1;
    }
    assert!(
        events.iter().any(|e| e["kind"] == "floored"),
        "standing in it hurts"
    );
    assert!(game.raiders.iter().any(|r| r.downs > 0));
}

#[test]
fn a_siege_party_of_bots_wears_the_titan_down() {
    use crate::titan::TitanSiege;
    let mut game = TitanSiege::new(&contestants(4, true), 23);
    let mut events = Vec::new();
    run_bots(&mut game, 4, 110.0, &mut events);
    let placements = game.placements().expect("the siege ended");
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
    assert!(
        events.iter().any(|e| e["kind"] == "strike_core"),
        "they got hits in"
    );
    let hits: u32 = game.raiders.iter().map(|r| r.hits).sum();
    assert!(hits >= 5, "the titan took a beating, {hits} hits");
}

fn bots_at(count: u8, skill: f32) -> Vec<Contestant> {
    (0..count)
        .map(|seat| Contestant::bot(seat, skill))
        .collect()
}

/// How many paint hits a table of bots lands in half a minute.
fn splats_at(skill: f32) -> usize {
    let mut game = PaintBlasters::new(&bots_at(4, skill), 31);
    let mut events = Vec::new();
    for _ in 0..(30.0 / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
    }
    events.iter().filter(|e| e["kind"] == "splat").count()
}

#[test]
fn the_skill_setting_decides_how_hard_the_bots_play() {
    let relaxed = splats_at(0.15);
    let ruthless = splats_at(1.0);
    assert!(
        ruthless > relaxed * 2,
        "sharp bots hit far more often: {ruthless} against {relaxed}"
    );
    assert!(relaxed > 0, "even the relaxed ones play, {relaxed} hits");
}

#[test]
fn every_game_takes_a_skill_setting_without_falling_over() {
    for key in BOUTS.iter().map(|spec| spec.key) {
        for skill in [0.0f32, 0.5, 1.0] {
            let mut game = create(key, &bots_at(4, skill), 77).expect("a factory");
            let mut events = Vec::new();
            for _ in 0..(6.0 / TICK_DT) as usize {
                let inputs: Vec<(u8, Input)> =
                    (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
                game.step(&inputs, TICK_DT, &mut events);
            }
            let snap = game.snapshot();
            assert!(
                snap.is_object(),
                "{key} still reports a snapshot at skill {skill}"
            );
        }
    }
}

#[test]
fn a_room_deals_coven_to_its_people_and_keeps_hands_private() {
    use crate::coven::Action;
    use crate::room::COVEN_KEY;
    let mut room = Room::new("coven-room", 5);
    let full = crate::room::MAX_SEATS;
    for i in 0..full {
        room.connect(&format!("u{i}"), &format!("Player {i}"), "").unwrap();
    }
    assert_eq!(room.max_seats_for_test(), full);
    assert_eq!(room.connect("extra", "Extra", ""), Err(RoomError::Full));
    room.choose_game("u0", COVEN_KEY).unwrap();
    assert_eq!(room.max_seats_for_test(), crate::coven::MAX_PLAYERS);
    room.connect("extra", "Extra", "").unwrap();
    assert_eq!(room.choose_game("u0", "bumper"), Err(RoomError::TooManyPlayers));
    room.add_bot("u0").unwrap();
    room.start("u0").unwrap();
    assert_eq!(room.phase, Phase::Table);
    assert!(room.members.iter().all(|m| m.in_round));

    let (game, mine) = room.table_view(room.table_seat_of("u1")).unwrap();
    assert_eq!(game, "coven");
    // Everyone who filled the room, the one past it, and a bot.
    assert_eq!(mine["players"].as_array().unwrap().len(), full + 2);
    assert_eq!(mine["hand"].as_array().unwrap().len(), crate::coven::HAND_AT_START);
    assert_eq!(mine["seats"].as_array().unwrap().len(), full + 2);
    let (_, watching) = room.table_view(None).unwrap();
    assert!(watching["hand"].is_null());

    for _ in 0..(crate::coven::DAWN_SECONDS as usize + 1) * 60 {
        room.step();
    }
    let (_, view) = room.table_view(None).unwrap();
    assert_eq!(view["phase"]["kind"], "turn");
    let seat = view["phase"]["seat"].as_u64().unwrap() as usize;
    let user = format!("u{}", room.table_order_for_test()[seat]);
    let other = format!("u{}", room.table_order_for_test()[(seat + 1) % 7]);
    let draw = serde_json::to_value(Action::Draw).unwrap();
    assert!(matches!(room.table_act(&other, draw.clone()), Err(RoomError::Table(_))));
    room.table_act(&user, draw).unwrap();
    assert!(room.take_dirty());
}

fn timberline_walk(map: &crate::shooter::Timberline, from: (f32, f32), stick: Stick, jump: bool, seconds: f32) -> (crate::fpmove::Body, f32) {
    use crate::fpmove::{Body, step_body};
    let mut body = Body {
        x: from.0,
        z: from.1,
        y: crate::shooter::ground_height(&map.blocks, from.0, from.1),
        on_ground: true,
        ..Default::default()
    };
    let input = Input { stick, jump, ..Default::default() };
    let mut peak: f32 = body.y;
    let dt = 1.0 / 60.0;
    for _ in 0..(seconds / dt) as usize {
        step_body(&mut body, input, dt, &map.blocks, 26.0, 26.0);
        peak = peak.max(body.y);
    }
    (body, peak)
}

#[test]
fn timberline_stairs_climb_onto_the_ridge_without_jumping() {
    let map = crate::shooter::Timberline::build();
    let (body, _) = timberline_walk(&map, (9.5, -10.5), Stick { x: 0.0, z: -1.0 }, false, 3.0);
    assert!(body.z < -17.5, "walked past the top step onto the ridge, z={}", body.z);
    assert!((body.y - 2.4).abs() < 0.05, "standing on the ridge, y={}", body.y);
}

#[test]
fn timberline_bridge_leaves_room_to_walk_underneath_and_bumps_heads() {
    let map = crate::shooter::Timberline::build();
    let (under, _) = timberline_walk(&map, (0.0, -14.0), Stick { x: 0.0, z: 1.0 }, false, 1.0);
    assert!(under.y.abs() < 0.01, "stayed on the meadow floor, y={}", under.y);
    assert!(under.z > -9.0 && under.x.abs() < 0.2, "walked straight on under the bridge: ({}, {})", under.x, under.z);

    let (_, peak) = timberline_walk(&map, (0.0, -12.0), Stick::default(), true, 0.6);
    assert!(peak + crate::fpmove::PLAYER_HEIGHT <= 1.96, "head stopped at the bridge's underside, peak={peak}");
}

#[test]
fn timberline_canopies_block_sight_and_spawns_start_clear() {
    let map = crate::shooter::Timberline::build();
    let from = (-20.5, 6.5, -9.0);
    let to = (-19.9, 1.0, -8.4);
    assert!(!crate::fpmove::can_see(from, to, &map.blocks, 26.0, 26.0), "leaves hide whoever stands under the tree");

    for &(x, z) in &map.spawns {
        let (body, _) = timberline_walk(&map, (x, z), Stick::default(), false, 0.2);
        assert!((body.x - x).abs() < 0.01 && (body.z - z).abs() < 0.01, "spawn ({x}, {z}) is not inside anything");
    }
    assert_eq!(map.blocks.len(), map.kinds.len());
}

#[test]
fn whoever_is_nearest_pockets_the_coin_and_gold_is_worth_five() {
    use crate::coins::{Coin, CoinScramble, GOLD};
    let mut game = CoinScramble::new(&contestants(2, false), 2);
    game.players[0].run = crate::ground::Runner::at(0.0, 0.0);
    game.players[1].run = crate::ground::Runner::at(6.0, 0.0);
    game.coins.push(Coin {
        id: 900,
        x: 0.5,
        z: 0.0,
        y: 0.0,
        fall: 0.0,
        value: GOLD,
        scatter: 0.0,
    });
    let mut events = Vec::new();
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.purse(0), GOLD, "the nearest player pocketed it");
    assert_eq!(game.purse(1), 0, "nobody else got a share");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "grab" && e["seat"] == 0 && e["value"] == GOLD),
        "the grab was announced"
    );
}

#[test]
fn a_charge_shakes_coins_out_of_the_purse_it_hits() {
    use crate::coins::{CoinScramble, SAFE_SECONDS, SPILL_MAX};
    let mut game = CoinScramble::new(&contestants(2, false), 4);
    game.players[0].run = crate::ground::Runner::at(0.0, 0.0);
    game.players[1].run = crate::ground::Runner::at(0.0, 1.5);
    game.players[1].purse = 10;
    let mut events = Vec::new();
    let charge = Input {
        stick: Stick { x: 0.0, z: 1.0 },
        fire: true,
        ..Default::default()
    };
    game.step(&[(0, charge)], TICK_DT, &mut events);
    game.step(
        &[(
            0,
            Input {
                fire: false,
                ..charge
            },
        )],
        TICK_DT,
        &mut events,
    );
    let spill = events
        .iter()
        .find(|e| e["kind"] == "spill")
        .expect("the charge connected");
    let loose = spill["coins"].as_u64().unwrap_or(0) as u32;
    assert!(loose > 0 && loose <= SPILL_MAX, "coins came loose, {loose}");
    assert_eq!(
        game.purse(1),
        10 - loose,
        "the purse is lighter by what spilled"
    );
    assert_eq!(
        game.coins.iter().filter(|coin| coin.scatter > 0.0).count(),
        loose as usize,
        "and they are bouncing on the floor"
    );
    assert!(
        game.coins.iter().all(|coin| !coin.loose()),
        "nobody scoops them up the instant they land"
    );

    // The breather after a charge stops a second rush emptying the purse.
    for _ in 0..((SAFE_SECONDS * 0.5) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    let held = game.purse(1);
    game.players[0].cooldown = 0.0;
    game.players[0].run = crate::ground::Runner::at(0.0, 0.0);
    game.players[1].run = crate::ground::Runner::at(0.0, 1.5);
    game.step(&[(0, charge)], TICK_DT, &mut events);
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.purse(1), held, "the second charge came too soon");
}

#[test]
fn coin_scramble_rains_gold_late_and_the_lead_turns_over() {
    use crate::coins::{CoinScramble, GOLD_FROM, ROUND_SECONDS as SCRAMBLE_SECONDS};
    let mut game = CoinScramble::new(&contestants(4, true), 5);
    let mut events = Vec::new();
    let mut leaders: Vec<u8> = Vec::new();
    let mut first_gold: Option<f32> = None;
    let mut steps = 0usize;
    while game.placements().is_none() && steps < (120.0 / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
        if first_gold.is_none() && events.iter().any(|e| e["kind"] == "gold") {
            first_gold = Some(game.time);
        }
        if let Some(seat) = game.leader() {
            if leaders.last() != Some(&seat) {
                leaders.push(seat);
            }
        }
    }
    let played = steps as f32 * TICK_DT;
    assert!(
        (45.0..=75.0).contains(&played),
        "a round runs about a minute, {played}"
    );
    assert!(
        first_gold.unwrap_or(0.0) >= SCRAMBLE_SECONDS - GOLD_FROM,
        "the big coins come late, {first_gold:?}"
    );
    assert!(
        events.iter().filter(|e| e["kind"] == "gold").count() >= 3,
        "and more than one of them falls"
    );
    assert!(leaders.len() >= 2, "the lead changed hands, {leaders:?}");
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "spill" && e["coins"].as_u64().unwrap_or(0) > 0),
        "charges shook coins loose"
    );
    assert!(
        game.players.iter().all(|p| p.purse >= 8),
        "every bot filled a purse, {:?}",
        game.players.iter().map(|p| p.purse).collect::<Vec<_>>()
    );
    let placements = game.placements().expect("the clock ran out");
    assert_eq!(
        placements.iter().map(|g| g.len()).sum::<usize>(),
        4,
        "everyone is placed"
    );
    let purses: Vec<u32> = placements
        .iter()
        .map(|group| game.purse(group[0]))
        .collect();
    assert!(
        purses.windows(2).all(|pair| pair[0] >= pair[1]),
        "the fullest purse wins, {purses:?}"
    );
}

/// Alternate left and right for `seconds`, with `gap` ticks between presses.
fn heave(game: &mut TugOfWar, seats: &[u8], seconds: f32, gap: usize, events: &mut Vec<Value>) {
    let mut side = -1.0f32;
    for tick in 0..(seconds / TICK_DT) as usize {
        let mut press = None;
        if tick % gap == 0 {
            side = -side;
            press = Some(if side > 0.0 { Press::Right } else { Press::Left });
        }
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|seat| (*seat, Input { press, ..Input::from_stick(Stick { x: side, z: 0.0 }) }))
            .collect();
        game.step(&inputs, TICK_DT, events);
        if game.placements().is_some() {
            return;
        }
    }
}

#[test]
fn alternating_presses_haul_the_rope_and_the_same_press_twice_does_nothing() {
    let mut game = TugOfWar::new(&contestants(2, false), 11);
    let mut events = Vec::new();
    heave(&mut game, &[0], 3.0, 6, &mut events);
    let hauled = game.rope;
    assert!(hauled < -0.3, "alternating presses pull the rope: {hauled}");
    assert!(
        game.pullers[0].heaves > 4,
        "every clean press is counted: {}",
        game.pullers[0].heaves
    );

    for _ in 0..180 {
        game.step(
            &[(0, Input::from_stick(Stick { x: -1.0, z: 0.0 }))],
            TICK_DT,
            &mut events,
        );
    }
    assert_eq!(game.rope, hauled, "holding one side does not move the rope");
}

#[test]
fn a_room_counts_every_tug_press_sent_between_two_ticks() {
    let mut room = Room::new("lobby-tug", 21);
    room.connect("a", "A", "").unwrap();
    room.connect("b", "B", "").unwrap();
    room.choose_game("a", crate::tug::KEY).unwrap();
    room.start("a").unwrap();
    room.queue_press("a", Press::Left);
    room.queue_press("a", Press::Right);
    for _ in 0..(60 * 40) {
        if room.phase == Phase::Playing {
            break;
        }
        room.step();
    }
    assert_eq!(room.phase, Phase::Playing);
    let heaves = |room: &Room| {
        room.snapshot().expect("tug is running")["players"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["seat"] == room.members[0].seat)
            .map(|p| p["heaves"].as_u64().unwrap())
            .unwrap()
    };
    assert_eq!(heaves(&room), 0, "practice taps do not carry into the bout");

    room.queue_press("a", Press::Left);
    room.step();
    for _ in 0..6 {
        room.step();
    }
    room.queue_press("a", Press::Right);
    room.queue_press("a", Press::Left);
    room.step();
    room.step();
    assert_eq!(heaves(&room), 2, "both taps that landed between ticks pull");
}

#[test]
fn tug_of_war_reads_the_sides_off_the_contestants() {
    let picked: Vec<Contestant> = [1u8, 1, 0, 0]
        .iter()
        .enumerate()
        .map(|(seat, team)| Contestant::person(seat as u8).on_team(*team))
        .collect();
    let game = TugOfWar::new(&picked, 3);
    let sides: Vec<Option<u8>> = (0..4).map(|seat| game.team_of(seat)).collect();
    assert_eq!(
        sides,
        vec![Some(1), Some(1), Some(0), Some(0)],
        "the board's split is used, not the order the seats arrived in"
    );
    assert!(
        game.pullers[0].x > 0.0 && game.pullers[2].x < 0.0,
        "each side lines up at its own end"
    );

    let lobby = TugOfWar::new(&bots_at(4, 0.4), 3);
    assert_eq!(
        (0..4).map(|seat| lobby.team_of(seat)).collect::<Vec<_>>(),
        vec![Some(0), Some(1), Some(0), Some(1)],
        "with no split given the sides alternate down the list"
    );
}

#[test]
fn the_losing_side_goes_in_the_water_and_the_winners_are_grouped_together() {
    let seats = contestants(4, false);
    let mut game = TugOfWar::new(&seats, 17);
    let mut events = Vec::new();
    // Seats 0 and 2 are red; nobody on blue touches a key.
    heave(&mut game, &[0, 2], TUG_SECONDS, 7, &mut events);
    assert_eq!(game.dunked, Some(1), "blue loses the rope");
    for _ in 0..(2.0 / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(
        game.placements().expect("the round is called"),
        vec![vec![0, 2], vec![1, 3]],
        "the winning side is one group, best first, so both of them are paid"
    );
    assert!(
        events.iter().any(|event| event["kind"] == "splash"),
        "the losing side makes a splash"
    );
    for puller in game.pullers.iter().filter(|p| p.team == 1) {
        assert!(puller.y < -1.0, "blue drops off the dock: {}", puller.y);
        assert!(
            puller.x.abs() < TUG_PIT,
            "blue ends up over the water: {}",
            puller.x
        );
    }
    for puller in game.pullers.iter().filter(|p| p.team == 0) {
        assert_eq!(puller.y, 0.0, "red keeps its footing");
    }
}

#[test]
fn tug_of_war_bots_pull_a_round_out_and_the_rope_decides_it() {
    let seats = contestants(4, true);
    let mut game = TugOfWar::new(&seats, 29);
    let mut events = Vec::new();
    let mut seconds = 0.0;
    while game.placements().is_none() && seconds < 90.0 {
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        seconds += TICK_DT;
    }
    let placements = game.placements().expect("bots play the round out");
    assert_eq!(placements.len(), 2, "a team bout ends with two sides");
    assert_eq!(placements.iter().flatten().count(), 4);
    assert!(
        (25.0..60.0).contains(&seconds),
        "a round of bots runs about a minute: {seconds}"
    );
    let heaves = events.iter().filter(|e| e["kind"] == "heave").count();
    assert!(heaves > 100, "both sides keep hauling: {heaves}");
    assert!(
        game.pullers.iter().all(|p| p.heaves > 10),
        "every bot finds a rhythm"
    );
}

#[test]
fn the_host_picks_the_board_before_the_party_starts() {
    use crate::board::Map;
    let mut room = Room::new("party-map", 21);
    room.connect("alice", "Alice", "").unwrap();
    room.connect("bob", "Bob", "").unwrap();
    assert_eq!(room.choose_map("bob", Map::Volcano), Err(RoomError::NotHost));
    room.choose_map("alice", Map::Volcano).unwrap();
    assert_eq!(room.view().map, Map::Volcano);
    room.start_party("alice").unwrap();
    assert_eq!(room.view().board.as_ref().unwrap().map, Map::Volcano);
    assert_eq!(room.choose_map("alice", Map::Island), Err(RoomError::WrongPhase));
}

#[test]
fn the_lone_last_place_rolls_a_small_extra_die_from_round_two() {
    let mut board = Board::new(&seats(3), 5, 31);
    let last = board.turn_seat();
    for seat in seats(3) {
        board.place_for_test(seat, 0, if seat == last { 0 } else { 10 });
    }
    assert_eq!(board.handicapped(), None, "nobody is behind before a round is played");
    board.set_round_for_test(2);
    assert_eq!(board.handicapped(), Some(last));
    assert_eq!(board.view().handicap, Some(last));

    let other = seats(3).into_iter().find(|seat| *seat != last).unwrap();
    board.place_for_test(other, 0, 0);
    assert_eq!(board.handicapped(), None, "a shared last place gets no help");
    board.place_for_test(other, 0, 10);

    let roll = board.roll(last).unwrap();
    assert!((2..=crate::board::DICE_SIDES + crate::board::HANDICAP_SIDES).contains(&roll));
}

#[test]
fn the_new_round_events_can_all_be_drawn() {
    for event in [RoundEvent::Fog, RoundEvent::TollRoad, RoundEvent::MarketDay, RoundEvent::Earthquake] {
        assert!(RoundEvent::DRAWN.contains(&event), "{event:?} turns up");
    }
    assert!(!RoundEvent::DRAWN.contains(&RoundEvent::LastRound), "the finale is never drawn");
}

/// Roll a fixed number, taking the lap at any junction, and return the
/// spaces walked over the whole turn.
fn walk_fixed(board: &mut Board, seat: u8, number: u8) -> Vec<u8> {
    board.give_for_test(seat, Item::GoldenDie);
    use_now(board, seat, Item::GoldenDie, Some(number));
    board.roll(seat).unwrap();
    while let TurnStep::Branch { options, .. } = board.step().clone() {
        board.choose_branch(seat, options[0].node).unwrap();
    }
    board
        .view()
        .log
        .iter()
        .filter(|entry| entry["kind"] == "move" && entry["seat"] == seat)
        .flat_map(|entry| entry["path"].as_array().cloned().unwrap_or_default())
        .filter_map(|space| space.as_u64().map(|id| id as u8))
        .collect()
}

#[test]
fn fog_hides_everyone_elses_purse_from_each_seat() {
    let mut board = Board::new(&seats(3), 5, 31);
    board.place_for_test(1, 0, 42);
    board.set_event_for_test_full(RoundEvent::Fog);

    let seen = serde_json::to_value(board.view_for(Some(0))).unwrap();
    for token in seen["tokens"].as_array().unwrap() {
        if token["seat"] == 0 {
            assert_eq!(token["coins"], START_COINS, "you still see your own purse");
        } else {
            assert!(token["coins"].is_null(), "seat {} is hidden", token["seat"]);
        }
    }
    let watching = serde_json::to_value(board.view_for(None)).unwrap();
    assert!(
        watching["tokens"].as_array().unwrap().iter().all(|token| token["coins"].is_null()),
        "a spectator sees nobody's purse"
    );
    assert_eq!(token_of(&board, 1).coins, 42, "the coins are still there, just unseen");

    board.set_event_for_test_full(RoundEvent::Windfall);
    let clear = serde_json::to_value(board.view_for(Some(0))).unwrap();
    assert!(
        clear["tokens"].as_array().unwrap().iter().all(|token| token["coins"].is_number()),
        "the fog lifts with the next event"
    );
}

#[test]
fn the_toll_road_charges_for_every_space_walked() {
    let mut board = Board::new(&seats(2), 5, 33);
    let me = board.turn_seat();
    board.set_event_for_test_full(RoundEvent::TollRoad);
    board.place_for_test(me, 1, 50);
    let walked = walk_fixed(&mut board, me, 5);
    assert_eq!(walked.len(), 5);
    let paid: i64 = board
        .view()
        .log
        .iter()
        .filter(|entry| entry["kind"] == "toll_road" && entry["seat"] == me)
        .map(|entry| entry["coins"].as_i64().unwrap())
        .sum();
    assert_eq!(paid, 5 * crate::board::TOLL_ROAD_COINS as i64, "a coin a space, the stop included");

    let mut broke = Board::new(&seats(2), 5, 33);
    let me = broke.turn_seat();
    broke.set_event_for_test_full(RoundEvent::TollRoad);
    broke.place_for_test(me, 1, 2);
    walk_fixed(&mut broke, me, 5);
    let paid: i64 = broke
        .view()
        .log
        .iter()
        .filter(|entry| entry["kind"] == "toll_road")
        .map(|entry| entry["coins"].as_i64().unwrap())
        .sum();
    assert!(paid <= 2, "nobody pays more than they have");

    let mut calm = Board::new(&seats(2), 5, 33);
    let me = calm.turn_seat();
    calm.place_for_test(me, 1, 50);
    walk_fixed(&mut calm, me, 5);
    assert!(!calm.view().log.iter().any(|entry| entry["kind"] == "toll_road"), "only on the toll road");
}

#[test]
fn market_day_puts_out_more_at_half_price() {
    let mut board = Board::new(&seats(2), 5, 17);
    let me = board.turn_seat();
    board.set_event_for_test_full(RoundEvent::MarketDay);
    let shop = board_space_of(SpaceKind::Shop);
    board.place_for_test(me, shop, 60);
    board.land_for_test(me);
    let TurnStep::Shop { offers, .. } = board.step().clone() else {
        panic!("the shop opened, not {:?}", board.step());
    };
    assert_eq!(offers.len(), crate::board::MARKET_OFFERS, "five on the counter");
    let items: std::collections::HashSet<Item> = offers.iter().map(|offer| offer.item).collect();
    assert_eq!(items.len(), offers.len(), "no duplicates");
    for offer in &offers {
        assert_eq!(offer.price, (offer.item.price() + 1) / 2, "half price, rounded up");
        assert!(offer.price < offer.item.price());
    }
    let wanted = offers[0];
    board.shop(me, Some(wanted.item)).unwrap();
    assert_eq!(token_of(&board, me).coins, 60 - wanted.price, "charged the market price");
}

#[test]
fn an_earthquake_closes_the_shortcuts_for_players_and_bots() {
    for map in board::Map::ALL {
        let layout = map.layout();
        let junction = layout
            .iter()
            .find(|space| space.next.len() > 1 && space.next.iter().all(|way| !layout[*way as usize].gated))
            .expect("the map has an open shortcut");
        let mut quake = Board::on(map, &seats(2), 5, 41);
        let me = quake.turn_seat();
        quake.set_event_for_test_full(RoundEvent::Earthquake);
        quake.place_for_test(me, junction.id, 50);
        let walked = walk_fixed(&mut quake, me, 3);
        assert!(!matches!(quake.step(), TurnStep::Branch { .. }), "{map:?}: nobody is asked");
        assert_eq!(walked.first(), Some(&junction.next[0]), "{map:?}: the long way round");
        assert!(!quake.view().log.iter().any(|entry| entry["kind"] == "branch"));

        let mut calm = Board::on(map, &seats(2), 5, 41);
        let me = calm.turn_seat();
        calm.place_for_test(me, junction.id, 50);
        calm.give_for_test(me, Item::GoldenDie);
        use_now(&mut calm, me, Item::GoldenDie, Some(3));
        calm.roll(me).unwrap();
        assert!(matches!(calm.step(), TurnStep::Branch { .. }), "{map:?}: otherwise the junction asks");
    }

    let mut bots = Board::on(board::Map::Volcano, &seats(4), 5, 43);
    bots.set_event_for_test_full(RoundEvent::Earthquake);
    let mut events = Vec::new();
    for _ in 0..3 {
        run_board_round(&mut bots);
        events.extend(bots.take_events());
    }
    assert!(events.iter().any(|entry| entry["kind"] == "move"), "the bots did walk");
    assert!(
        !events.iter().any(|entry| entry["kind"] == "branch"),
        "bots never get to pick a shortcut either"
    );
}

fn memory_spot(game: &MemoryTiles, seat: u8) -> (f32, f32) {
    let snap = game.snapshot();
    let player = snap["players"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == seat)
        .unwrap()
        .clone();
    (player["x"].as_f64().unwrap() as f32, player["z"].as_f64().unwrap() as f32)
}

/// Walk one seat onto a tile and stomp it, everyone else standing still.
fn stomp_tile(game: &mut MemoryTiles, seat: u8, tile: usize, events: &mut Vec<Value>) {
    let (tx, tz) = memory_tile_centre(tile);
    for _ in 0..600 {
        let (x, z) = memory_spot(game, seat);
        let (dx, dz) = (tx - x, tz - z);
        let far = dx.hypot(dz);
        if far < 0.3 {
            let stomp = Input { fire: true, ..Default::default() };
            game.step(&[(seat, stomp)], TICK_DT, events);
            return;
        }
        let pace = far.min(1.0) / far;
        let walk = Input::from_stick(Stick { x: dx * pace, z: dz * pace });
        game.step(&[(seat, walk)], TICK_DT, events);
    }
    panic!("seat {seat} never reached tile {tile}");
}

fn until_recall(game: &mut MemoryTiles, events: &mut Vec<Value>) {
    while game.phase() != MemoryPhase::Recall {
        game.step(&[], TICK_DT, events);
    }
}

fn wrong_tile(right: usize) -> usize {
    (right + 1) % MEMORY_TILES
}

#[test]
fn memory_tiles_have_gaps_between_them_that_count_as_no_tile() {
    for tile in 0..MEMORY_TILES {
        let (x, z) = memory_tile_centre(tile);
        assert_eq!(memory_tile_at(x, z), Some(tile));
    }
    let (left, _) = memory_tile_centre(0);
    let (right, _) = memory_tile_centre(1);
    assert_eq!(memory_tile_at((left + right) / 2.0, 0.0), None, "the gap is not a tile");
    assert_eq!(memory_tile_at(9.0, 9.0), None, "the corners are off the grid");
}

#[test]
fn memory_tiles_light_the_sequence_and_stomps_only_count_on_the_walk_back() {
    let mut game = MemoryTiles::new(&contestants(2, false), 5);
    let mut events = Vec::new();
    let sequence = game.sequence().to_vec();
    assert_eq!(sequence.len(), MEMORY_FIRST);
    assert!(sequence.windows(2).all(|pair| pair[0] != pair[1]), "no tile lights twice in a row");

    stomp_tile(&mut game, 0, sequence[0], &mut events);
    assert_eq!(game.phase(), MemoryPhase::Show, "the walk took less than the showing");
    assert!(!events.iter().any(|e| e["kind"] == "stomp"), "a stomp while it is showing does nothing");

    until_recall(&mut game, &mut events);
    let lit: Vec<u64> = events
        .iter()
        .filter(|e| e["kind"] == "light")
        .map(|e| e["tile"].as_u64().unwrap())
        .collect();
    assert_eq!(lit, sequence.iter().map(|t| *t as u64).collect::<Vec<_>>(), "every tile lit in order");
    assert!(game.snapshot()["limit"].as_f64().is_some(), "the walk back is on the clock");

    for tile in &sequence {
        stomp_tile(&mut game, 0, *tile, &mut events);
    }
    assert!(events.iter().any(|e| e["kind"] == "done" && e["seat"] == 0));
    // Seat 1 never moves, so the clock runs out on them and seat 0 is left.
    for _ in 0..(memory_recall_seconds(sequence.len()) / TICK_DT) as usize + 2 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(events.iter().any(|e| e["kind"] == "timeout" && e["seat"] == 1));
    assert_eq!(game.placements(), Some(vec![vec![0], vec![1]]));
}

#[test]
fn a_wrong_tile_puts_you_out_and_the_sequence_grows_for_the_rest() {
    let mut game = MemoryTiles::new(&contestants(3, false), 9);
    let mut events = Vec::new();
    let first = game.sequence().to_vec();
    until_recall(&mut game, &mut events);

    stomp_tile(&mut game, 0, wrong_tile(first[0]), &mut events);
    assert!(events.iter().any(|e| e["kind"] == "wrong" && e["seat"] == 0));
    for seat in [1, 2] {
        for tile in &first {
            stomp_tile(&mut game, seat, *tile, &mut events);
        }
    }
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.phase(), MemoryPhase::Break, "both who got it right go on");
    while game.phase() == MemoryPhase::Break {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.round(), 2);
    let second = game.sequence().to_vec();
    assert_eq!(second.len(), first.len() + 1, "one tile longer each round");
    assert_eq!(second[..first.len()], first[..], "the old sequence is kept and extended");

    until_recall(&mut game, &mut events);
    let before = memory_spot(&game, 0);
    let walk = Input::from_stick(Stick { x: 1.0, z: 0.0 });
    game.step(&[(0, walk)], TICK_DT, &mut events);
    assert_eq!(memory_spot(&game, 0), before, "someone who is out stands still");

    stomp_tile(&mut game, 1, second[0], &mut events);
    stomp_tile(&mut game, 1, wrong_tile(second[1]), &mut events);
    stomp_tile(&mut game, 2, second[0], &mut events);
    stomp_tile(&mut game, 2, second[1], &mut events);
    for _ in 0..(memory_recall_seconds(second.len()) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(
        game.placements(),
        Some(vec![vec![2], vec![1], vec![0]]),
        "out in the same round, the one who got further places higher"
    );
}

#[test]
fn everyone_out_on_the_same_step_shares_the_place() {
    let mut game = MemoryTiles::new(&contestants(3, false), 2);
    let mut events = Vec::new();
    until_recall(&mut game, &mut events);
    for _ in 0..(memory_recall_seconds(MEMORY_FIRST) / TICK_DT) as usize + 2 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.placements(), Some(vec![vec![0, 1, 2]]));
}

fn memory_bots(seats: &[Contestant], seed: u64) -> (MemoryTiles, f32, Vec<Value>) {
    let mut game = MemoryTiles::new(seats, seed);
    let mut events = Vec::new();
    let mut seconds = 0.0;
    while game.placements().is_none() {
        assert!(seconds < 150.0, "bots finish a game of memory");
        let inputs: Vec<(u8, Input)> = seats
            .iter()
            .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        seconds += TICK_DT;
    }
    (game, seconds, events)
}

#[test]
fn sharp_memory_bots_outlast_clumsy_ones() {
    let mut sharp_wins = 0;
    for seed in 0..30 {
        let seats = vec![Contestant::bot(0, 1.0), Contestant::bot(1, 0.15)];
        let (game, _, events) = memory_bots(&seats, seed);
        if game.placements().unwrap()[0] == vec![0] {
            sharp_wins += 1;
        }
        assert!(
            !events.iter().any(|e| e["kind"] == "timeout"),
            "bots walk the sequence in time, they only misremember it"
        );
    }
    assert!(sharp_wins >= 24, "the sharp bot usually wins: {sharp_wins} of 30");
}

#[test]
fn a_table_of_bots_plays_memory_out_inside_the_registry_budget() {
    for skill in [0.15, 0.4, 0.7, 1.0] {
        for seed in 0..12 {
            let (_, seconds, events) = memory_bots(&bots_at(4, skill), seed);
            assert!(seconds < 95.0, "skill {skill} seed {seed} took {seconds}s");
            assert!(
                events.iter().any(|e| e["kind"] == "stomp"),
                "skill {skill} seed {seed}: the bots walk tiles"
            );
        }
    }
}

#[test]
fn rising_water_floods_the_floor_and_drowns_whoever_stays_on_it() {
    use crate::rising::{RisingWater, WADE, water_at};
    let mut game = RisingWater::new(&contestants(3, false), 3);
    let mut events = Vec::new();
    assert!(water_at(0.0) < 0.0, "the floor starts dry");
    tick(&mut game, 5.0, &mut events);
    assert!(game.players.iter().all(|p| p.up()), "nobody is under yet");
    assert!(game.water > water_at(0.0), "the water is coming up");

    tick(&mut game, 30.0, &mut events);
    assert!(game.water > WADE, "the floor is under");
    let placements = game.placements().expect("everyone on the floor went under");
    assert_eq!(placements, vec![vec![0, 1, 2]], "going under together is a tie");
    assert_eq!(
        events.iter().filter(|e| e["kind"] == "dunked").count(),
        3,
        "each of them went under"
    );
}

#[test]
fn rising_water_jumps_land_on_the_platform_above() {
    use crate::rising::{Footing, RisingWater, STEP};
    let mut game = RisingWater::new(&contestants(2, false), 5);
    let pad = game.pads[0].clone();
    assert!((pad.y - STEP).abs() < 1e-3, "the first ring is one step up");
    game.players[0].run.x = pad.x;
    game.players[0].run.z = pad.z;
    let mut events = Vec::new();
    let jump = Input {
        jump: true,
        ..Default::default()
    };
    game.step(&[(0, jump)], TICK_DT, &mut events);
    assert_eq!(game.players[0].footing, Footing::Air, "off the ground");
    assert!(events.iter().any(|e| e["kind"] == "jump" && e["seat"] == 0));
    tick(&mut game, 1.0, &mut events);
    assert_eq!(game.players[0].footing, Footing::Pad(0), "caught by the platform");
    assert!((game.players[0].y - pad.y).abs() < 1e-3);
    assert_eq!(game.players[1].footing, Footing::Floor, "the other one never jumped");
}

#[test]
fn rising_water_platforms_crumble_under_you_and_come_back_later() {
    use crate::rising::{CRUMBLE_SECONDS, Footing, PadState, REGROW_SECONDS, RisingWater};
    let mut game = RisingWater::new(&contestants(2, false), 8);
    let top = game.pads.len() - 1;
    let pad = game.pads[top].clone();
    game.players[0].run.x = pad.x;
    game.players[0].run.z = pad.z;
    game.players[0].y = pad.y;
    game.players[0].footing = Footing::Pad(top);
    let mut events = Vec::new();
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.pads[top].state, PadState::Crumbling, "standing on it starts it going");
    assert!(events.iter().any(|e| e["kind"] == "crack" && e["seat"] == 0));

    tick(&mut game, CRUMBLE_SECONDS + 0.1, &mut events);
    assert_eq!(game.pads[top].state, PadState::Gone, "nobody can camp the top");
    assert!(events.iter().any(|e| e["kind"] == "crumble"));
    assert_ne!(game.players[0].footing, Footing::Pad(top));
    assert!(game.players[0].y < pad.y, "we dropped with it");

    tick(&mut game, REGROW_SECONDS, &mut events);
    assert_eq!(
        game.pads[top].state,
        PadState::Solid,
        "a platform still clear of the water grows back"
    );
}

#[test]
fn rising_water_bots_climb_and_the_last_one_dry_wins() {
    use crate::rising::{RisingWater, STEP};
    let mut game = RisingWater::new(&contestants(4, true), 11);
    let mut events = Vec::new();
    let mut highest = 0.0f32;
    let mut steps = 0;
    while game.placements().is_none() && steps < 6_000 {
        let inputs: Vec<(u8, Input)> = (0..4)
            .map(|s| (s, game.bot_input(s, TICK_DT)))
            .collect();
        game.step(&inputs, TICK_DT, &mut events);
        highest = game.players.iter().map(|p| p.y).fold(highest, f32::max);
        steps += 1;
    }
    let placements = game.placements().expect("the water got everyone but one");
    assert_eq!(placements.iter().flatten().count(), 4, "everyone is placed");
    assert!(highest > STEP * 8.0, "bots climbed the tower, {highest}");
    assert!(
        steps as f32 * TICK_DT > 20.0,
        "bots outlasted the flooded floor"
    );
    assert!(events.iter().any(|e| e["kind"] == "crumble"));
    let first_out = events.iter().find(|e| e["kind"] == "dunked").unwrap()["seat"]
        .as_u64()
        .unwrap() as u8;
    assert_ne!(placements[0], vec![first_out], "the first one under did not win");
}

#[test]
fn rising_water_sharper_bots_stay_dry_longer() {
    use crate::rising::RisingWater;
    let mut sharp_wins = 0;
    for seed in 0..12u64 {
        let seats = vec![
            Contestant::bot(0, 1.0),
            Contestant::bot(1, 0.15),
            Contestant::bot(2, 0.15),
        ];
        let mut game = RisingWater::new(&seats, seed);
        let mut events = Vec::new();
        run_bots(&mut game, 3, 100.0, &mut events);
        let placements = game.placements().expect("the round ended");
        if placements[0].contains(&0) {
            sharp_wins += 1;
        }
    }
    assert!(sharp_wins >= 7, "the ruthless bot won {sharp_wins} of 12");
}

fn press(choice: u8) -> Input {
    Input {
        fire: true,
        yaw: choice as f32,
        ..Default::default()
    }
}

#[test]
fn the_first_right_answer_scores_and_a_wrong_guess_sits_you_out() {
    use crate::buzzer::BuzzerRound;
    let mut game = BuzzerRound::new(&contestants(3, false), 11);
    let mut events = Vec::new();
    let answer = game.answer();
    let wrong = (answer + 1) % 4;
    assert!(game.snapshot()["answer"].is_null(), "the answer stays off the wire while asking");

    game.step(&[(0, press(wrong))], TICK_DT, &mut events);
    assert!(!game.revealing(), "a wrong guess leaves the question open");
    assert_eq!(game.snapshot()["players"][0]["state"], "wrong");
    assert!(game.snapshot()["players"][0]["choice"].is_null(), "nobody sees what was picked yet");

    // A second press from the same seat is ignored, right or not.
    game.step(&[(0, press(answer))], TICK_DT, &mut events);
    assert!(!game.revealing());

    game.step(&[(1, press(answer))], TICK_DT, &mut events);
    assert!(game.revealing(), "the first right answer closes the question");
    let snap = game.snapshot();
    assert_eq!(snap["answer"], answer);
    assert_eq!(snap["scorers"], serde_json::json!([1]));
    let points: Vec<u64> = snap["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["points"].as_u64().unwrap())
        .collect();
    assert_eq!(points, vec![0, 1, 0]);
    assert_eq!(snap["players"][0]["choice"], wrong);
    assert_eq!(snap["players"][2]["state"], "silent");
    assert!(events.iter().any(|e| e["kind"] == "buzz" && e["seat"] == 0 && e["right"] == false));
    assert!(events.iter().any(|e| e["kind"] == "reveal" && e["answer"] == answer));
}

#[test]
fn answers_on_the_same_tick_both_score_and_a_bad_press_is_ignored() {
    use crate::buzzer::BuzzerRound;
    let mut game = BuzzerRound::new(&contestants(2, false), 12);
    let mut events = Vec::new();
    let answer = game.answer();
    let off_grid = Input { fire: true, yaw: 1.5, ..Default::default() };
    let out_of_range = Input { fire: true, yaw: 7.0, ..Default::default() };
    game.step(&[(0, off_grid), (1, out_of_range)], TICK_DT, &mut events);
    assert!(events.is_empty(), "neither press names an option");

    game.step(&[(0, press(answer)), (1, press(answer))], TICK_DT, &mut events);
    assert!(game.revealing());
    assert_eq!(game.snapshot()["scorers"], serde_json::json!([0, 1]));
}

#[test]
fn a_question_nobody_gets_times_out_and_the_next_one_follows_the_reveal() {
    use crate::buzzer::{ANSWER_SECONDS, BuzzerRound, REVEAL_SECONDS};
    let mut game = BuzzerRound::new(&contestants(2, false), 13);
    let mut events = Vec::new();
    tick(&mut game, ANSWER_SECONDS - 0.5, &mut events);
    assert!(!game.revealing(), "still time to answer");
    tick(&mut game, 0.6, &mut events);
    assert!(game.revealing(), "the clock ran out");
    assert_eq!(game.snapshot()["players"][0]["state"], "silent");
    tick(&mut game, REVEAL_SECONDS + 0.1, &mut events);
    assert_eq!(game.round(), 1);
    assert!(!game.revealing());
    assert!(game.snapshot()["answer"].is_null());
}

#[test]
fn everyone_guessing_wrong_ends_the_question_early() {
    use crate::buzzer::BuzzerRound;
    let mut game = BuzzerRound::new(&contestants(2, false), 14);
    let mut events = Vec::new();
    let wrong = (game.answer() + 1) % 4;
    game.step(&[(0, press(wrong)), (1, press(wrong))], TICK_DT, &mut events);
    assert!(game.revealing());
    assert_eq!(game.snapshot()["scorers"], serde_json::json!([]));
}

#[test]
fn the_most_points_after_the_last_question_wins_and_level_scores_tie() {
    use crate::buzzer::{BuzzerRound, QUESTIONS, REVEAL_SECONDS};
    let mut game = BuzzerRound::new(&contestants(3, false), 15);
    let mut events = Vec::new();
    for question in 0..QUESTIONS {
        assert_eq!(game.round(), question);
        // Seat 0 takes every question but the first, which goes to seat 1.
        let seat = if question == 0 { 1 } else { 0 };
        game.step(&[(seat, press(game.answer()))], TICK_DT, &mut events);
        assert!(game.revealing());
        assert!(game.placements().is_none(), "the bout waits out the reveal");
        tick(&mut game, REVEAL_SECONDS + 0.1, &mut events);
    }
    assert_eq!(game.placements(), Some(vec![vec![0], vec![1], vec![2]]));

    let mut quiet = BuzzerRound::new(&contestants(2, false), 16);
    tick(&mut quiet, 200.0, &mut events);
    assert_eq!(quiet.placements(), Some(vec![vec![0, 1]]), "nobody scoring is a tie");
}

#[test]
fn the_questions_are_drawn_from_a_clean_bank() {
    use crate::buzzer::BuzzerRound;
    for (text, right, wrong) in BuzzerRound::bank() {
        let mut all = vec![*right, wrong[0], wrong[1], wrong[2]];
        all.sort();
        all.dedup();
        assert_eq!(all.len(), 4, "{text} has four different options");
        assert!(!text.contains('\u{2014}'), "{text} is plain copy");
    }
    let one = BuzzerRound::new(&contestants(2, false), 1).snapshot();
    let other = BuzzerRound::new(&contestants(2, false), 2).snapshot();
    assert_ne!(
        (one["question"].clone(), one["options"].clone()),
        (other["question"].clone(), other["options"].clone()),
        "the seed picks the questions"
    );
}

#[test]
fn sharper_bots_answer_right_more_often_and_nobody_answers_instantly() {
    use crate::buzzer::BuzzerRound;
    let rate = |skill: f32| {
        let (mut right, mut asked) = (0u32, 0u32);
        for seed in 0..40 {
            let seats = vec![Contestant::bot(0, skill), Contestant::person(1)];
            let mut game = BuzzerRound::new(&seats, seed);
            let mut events = Vec::new();
            let mut since = 0.0f32;
            let mut round = usize::MAX;
            while game.placements().is_none() {
                if game.round() != round && !game.revealing() {
                    round = game.round();
                    since = 0.0;
                }
                let bot = game.bot_input(0, TICK_DT);
                if bot.fire {
                    assert!(since >= 1.4, "a bot buzzed after only {since}s");
                }
                events.clear();
                game.step(&[(0, bot)], TICK_DT, &mut events);
                since += TICK_DT;
                for event in &events {
                    if event["kind"] == "buzz" {
                        asked += 1;
                        right += (event["right"] == true) as u32;
                    }
                }
            }
        }
        right as f32 / asked.max(1) as f32
    };
    let ruthless = rate(1.0);
    let relaxed = rate(0.15);
    assert!(ruthless > 0.85, "a ruthless bot is nearly always right: {ruthless}");
    assert!(relaxed < 0.65, "a relaxed bot misses plenty: {relaxed}");
}

#[test]
fn a_room_keeps_the_choice_a_press_was_sent_with() {
    let mut room = Room::new("lobby-buzzer", 21);
    room.connect("a", "A", "").unwrap();
    room.connect("b", "B", "").unwrap();
    room.choose_game("a", crate::buzzer::KEY).unwrap();
    room.start("a").unwrap();
    for _ in 0..(60 * 40) {
        if room.phase == Phase::Playing {
            break;
        }
        room.step();
    }
    assert_eq!(room.phase, Phase::Playing);
    room.queue_fire("a", 2.0, 0.0);
    // The stick update that follows must not wipe the pick before the tick.
    room.set_input("a", Stick { x: 0.0, z: 0.0 }, 0.0, 0.0, false, false);
    room.queue_fire("b", 2.0, 0.0);
    room.step();
    let snap = room.snapshot().expect("a bout is running");
    assert_eq!(snap["stage"], "reveal", "both seats guessed");
    for player in snap["players"].as_array().unwrap() {
        assert_eq!(player["choice"], 2, "{player} kept the option it pressed");
    }
}

mod minecart {
    use super::*;
    use crate::minecart::{CART_LENGTH, GATE_GAP, JUNCTIONS, KNOCK, MinecartRush, STUN};

    /// Lean toward the short branch of whichever junction comes next.
    fn lean_fast(game: &MinecartRush, seat: u8) -> Input {
        let cart = game.cart(seat);
        let junction = (cart.leg / 2).min(JUNCTIONS - 1);
        let x = if game.course.fast[junction] == 0 { -1.0 } else { 1.0 };
        Input::from_stick(Stick { x, z: 0.0 })
    }

    fn ride_alone(lean: bool) -> (f32, u8) {
        let mut game = MinecartRush::new(&contestants(1, false), 4);
        let mut events = Vec::new();
        while game.placements().is_none() {
            let input = if lean { lean_fast(&game, 0) } else { Input::default() };
            game.step(&[(0, input)], TICK_DT, &mut events);
        }
        let cart = game.cart(0);
        (cart.ride.expect("the cart reached the finish"), cart.wrong_turns)
    }

    #[test]
    fn leaning_into_the_short_branch_beats_riding_the_switches_as_set() {
        let (quick, quick_wrong) = ride_alone(true);
        let (slow, slow_wrong) = ride_alone(false);
        assert_eq!(quick_wrong, 0, "every lean took the short branch");
        assert_eq!(
            slow_wrong as usize, JUNCTIONS,
            "an unleaning rider is sent the long way every time"
        );
        assert!(slow > quick + 3.0, "wrong turns cost real time: {quick} against {slow}");
    }

    /// Two carts rolling on the second straight, `gap` apart.
    fn on_the_straight(rear_speed: f32, front_speed: f32, gap: f32) -> MinecartRush {
        let mut game = MinecartRush::new(&contestants(2, false), 1);
        for seat in 0..2 {
            let cart = game.cart_mut(seat);
            cart.released_at = Some(0.0);
            cart.leg = 2;
        }
        let rear = game.cart_mut(0);
        rear.along = 5.0;
        rear.speed = rear_speed;
        let front = game.cart_mut(1);
        front.along = 5.0 + gap;
        front.speed = front_speed;
        game
    }

    #[test]
    fn ramming_a_rival_knocks_both_carts_back() {
        let mut game = on_the_straight(11.0, 2.0, CART_LENGTH - 0.1);
        let mut events = Vec::new();
        game.step(&[], TICK_DT, &mut events);
        assert!(events.iter().any(|e| e["kind"] == "crash"), "{events:?}");
        let (rear, front) = (game.cart(0).clone(), game.cart(1).clone());
        assert!(front.along < 5.0 + CART_LENGTH - KNOCK + 0.5, "the front cart lost ground");
        assert!(rear.along < 5.0, "and so did the one that rammed it");
        assert!(front.along - rear.along >= CART_LENGTH - 0.01, "they are pulled apart");
        assert!(rear.stun > 0.0 && front.stun > 0.0 && rear.speed == 0.0 && front.speed == 0.0);
        assert_eq!((rear.crashes, front.crashes), (1, 1));

        // Both stay put until the stun wears off, then roll again.
        let parked = (game.cart(0).along, game.cart(1).along);
        tick(&mut game, STUN * 0.5, &mut events);
        assert_eq!((game.cart(0).along, game.cart(1).along), parked);
        tick(&mut game, STUN, &mut events);
        assert!(game.cart(1).along > parked.1, "the front cart is rolling again");
    }

    #[test]
    fn rolling_gently_into_a_cart_buffers_up_behind_it_instead() {
        let mut game = on_the_straight(6.0, 5.0, CART_LENGTH - 0.1);
        let mut events = Vec::new();
        game.step(&[], TICK_DT, &mut events);
        assert!(!events.iter().any(|e| e["kind"] == "crash"));
        let (rear, front) = (game.cart(0), game.cart(1));
        assert!(front.along - rear.along >= CART_LENGTH - 0.01);
        assert!(rear.speed <= front.speed + 0.001);
        assert_eq!(rear.stun, 0.0);
    }

    #[test]
    fn carts_on_opposite_branches_crash_where_the_branches_meet() {
        let mut game = MinecartRush::new(&contestants(2, false), 1);
        for seat in 0..2 {
            let cart = game.cart_mut(seat);
            cart.released_at = Some(0.0);
            cart.leg = 1;
            cart.side = seat as usize;
        }
        // One just made it onto the straight; the other is about to.
        game.cart_mut(1).leg = 2;
        game.cart_mut(1).along = 0.2;
        game.cart_mut(1).speed = 1.0;
        let length = game.course.leg_length(1, 0);
        game.cart_mut(0).along = length - 0.5;
        game.cart_mut(0).speed = 12.0;
        let mut events = Vec::new();
        game.step(&[], TICK_DT, &mut events);
        assert!(events.iter().any(|e| e["kind"] == "crash"), "{events:?}");
    }

    #[test]
    fn the_gate_lets_carts_out_one_at_a_time_and_times_each_from_its_release() {
        let mut game = MinecartRush::new(&contestants(4, false), 9);
        let mut events = Vec::new();
        game.step(&[], TICK_DT, &mut events);
        let out = |game: &MinecartRush| game.carts.iter().filter(|c| c.released_at.is_some()).count();
        assert_eq!(out(&game), 1, "only one cart leaves at the start");
        tick(&mut game, GATE_GAP * 3.5, &mut events);
        assert_eq!(out(&game), 4);
        let mut releases: Vec<f32> = game.carts.iter().map(|c| c.released_at.unwrap()).collect();
        releases.sort_by(f32::total_cmp);
        assert!(releases.windows(2).all(|pair| pair[1] - pair[0] >= GATE_GAP - 0.01));

        // Everyone rides the same line with nobody leaning, so with no
        // crashes they all take as long, whatever order they left in.
        while game.placements().is_none() {
            game.step(&[], TICK_DT, &mut events);
        }
        let rides: Vec<f32> = game.carts.iter().map(|c| c.ride.unwrap()).collect();
        let longest = rides.iter().cloned().fold(f32::MIN, f32::max);
        let shortest = rides.iter().cloned().fold(f32::MAX, f32::min);
        assert!(longest - shortest < 0.1, "a later start is not a slower ride: {rides:?}");
    }

    #[test]
    fn the_quickest_ride_wins_and_unfinished_carts_rank_by_distance() {
        let mut game = MinecartRush::new(&contestants(3, false), 2);
        let mut events = Vec::new();
        // Seat 0 leans the right way, seat 1 never leans, and seat 2 holds
        // the brake the whole way.
        while game.placements().is_none() {
            let inputs = [
                (0, lean_fast(&game, 0)),
                (1, Input::default()),
                (2, Input::from_stick(Stick { x: 0.0, z: 1.0 })),
            ];
            game.step(&inputs, TICK_DT, &mut events);
        }
        let placed = game.placements().unwrap();
        assert_eq!(placed, vec![vec![0], vec![1], vec![2]]);
        assert!(game.cart(2).ride.is_none(), "the clock ran out on the cart that never rolled");
    }

    #[test]
    fn sharper_bots_take_fewer_wrong_turns() {
        let wrong = |skill: f32| -> u32 {
            (0..12u64)
                .map(|seed| {
                    let seats = vec![Contestant::bot(0, skill)];
                    let mut game = MinecartRush::new(&seats, seed);
                    let mut events = Vec::new();
                    while game.placements().is_none() {
                        let input = game.bot_input(0, TICK_DT);
                        game.step(&[(0, input)], TICK_DT, &mut events);
                    }
                    game.cart(0).wrong_turns as u32
                })
                .sum()
        };
        let (clumsy, ruthless) = (wrong(0.0), wrong(1.0));
        assert!(ruthless < clumsy, "ruthless {ruthless} against clumsy {clumsy}");
        assert!(ruthless <= 6, "a ruthless bot rarely misreads a junction: {ruthless}");
    }
}

fn relay_place(game: &mut crate::relay::BombRelay, seat: u8, x: f32, z: f32) {
    let player = game.players.iter_mut().find(|p| p.seat == seat).unwrap();
    player.run = crate::ground::Runner::at(x, z);
}

#[test]
fn the_relay_bomb_only_passes_to_the_runner_waiting_at_the_far_end() {
    use crate::relay::{BombRelay, PEN_Z};
    let mut game = BombRelay::new(&contestants(3, false), 4);
    let holder = game.holder.expect("a bomb is lit at the start");
    let receiver = game.receiver.expect("someone waits for it");
    let bystander = (0..3).find(|s| *s != holder && *s != receiver).unwrap();
    let heading = game.heading;
    let pen = game.player(receiver).map(|p| (p.run.x, p.run.z)).unwrap();
    assert_eq!(pen, (0.0, heading * PEN_Z), "the receiver waits in the far pen");
    let start = game.player(holder).unwrap().run.z;
    assert_eq!(start, -heading * PEN_Z, "the carrier starts at the other end");
    game.fuse = 100.0;

    let mut events = Vec::new();
    let here = game.player(holder).map(|p| (p.run.x, p.run.z)).unwrap();
    relay_place(&mut game, bystander, here.0 + 0.6, here.1);
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.holder, Some(holder), "touching someone on the bench does nothing");

    relay_place(&mut game, holder, pen.0, pen.1 - heading * 1.2);
    game.step(&[], TICK_DT, &mut events);
    assert_eq!(game.holder, Some(receiver), "the bomb changed hands");
    assert_eq!(game.heading, -heading, "and turns back down the lane");
    assert_eq!(game.receiver, Some(bystander), "the next in line is called up");
    let called = game.player(bystander).unwrap().run;
    assert_eq!((called.x, called.z), (0.0, -heading * PEN_Z), "to the pen at the other end");
    assert!(
        events.iter().any(|e| e["kind"] == "pass" && e["from"] == holder && e["to"] == receiver),
        "the pass was announced"
    );
    let snap = game.snapshot();
    let passer = snap["players"].as_array().unwrap().iter().find(|p| p["seat"] == holder).unwrap();
    assert_eq!(passer["waiting"], true, "the old carrier steps off the lane");
}

#[test]
fn the_relay_fuse_puts_out_the_carrier_and_burns_shorter_each_round() {
    use crate::relay::{BombRelay, FUSE_JITTER, RELIGHT_SECONDS, round_fuse};
    assert!(round_fuse(2) < round_fuse(1) && round_fuse(3) < round_fuse(2));
    let mut game = BombRelay::new(&contestants(3, false), 8);
    let holder = game.holder.unwrap();
    let due = game.receiver.unwrap();
    let mut events = Vec::new();
    game.fuse = 0.05;
    for _ in 0..6 {
        game.step(&[], TICK_DT, &mut events);
    }
    assert!(events.iter().any(|e| e["kind"] == "boom" && e["seat"] == holder));
    assert!(!game.player(holder).unwrap().playing(), "the carrier is out");
    assert!(game.placements().is_none(), "two are still in");

    for _ in 0..((RELIGHT_SECONDS + 0.1) / TICK_DT) as usize {
        game.step(&[], TICK_DT, &mut events);
    }
    assert_eq!(game.round, 2);
    assert_eq!(game.holder, Some(due), "the runner who was due takes the next bomb");
    assert_ne!(game.receiver, Some(holder), "nobody knocked out is called up");
    assert!(game.fuse <= round_fuse(2) * (1.0 + FUSE_JITTER), "a shorter fuse: {}", game.fuse);

    game.fuse = 0.05;
    for _ in 0..6 {
        game.step(&[], TICK_DT, &mut events);
    }
    let placed = game.placements().expect("one runner is left");
    assert_eq!(placed.len(), 3, "everyone went out at a different time");
    assert_eq!(placed[2], vec![holder], "the first out is last");
    assert_eq!(placed[1], vec![due], "the second out is second last");
}

#[test]
fn a_relay_gate_stops_a_carrier_off_the_gap_and_lets_one_through_it() {
    use crate::relay::{BombRelay, GATE_GAP, GATE_THICK, GATES, gate_x};
    let band = GATE_THICK / 2.0 + crate::ground::RUNNER_RADIUS;
    let gate_z = GATES[1].0;
    let push = |heading: f32| Input::from_stick(Stick { x: 0.0, z: heading });

    let mut game = BombRelay::new(&contestants(2, false), 3);
    let holder = game.holder.unwrap();
    game.fuse = 100.0;
    let heading = game.heading;
    let mut events = Vec::new();
    // Whichever side of the middle gate the gap is, stand on the other.
    let wall_x = if gate_x(1, game.time) > 0.0 { -4.2 } else { 4.2 };
    relay_place(&mut game, holder, wall_x, gate_z - heading * (band + 0.4));
    let mut blocked = 0;
    for _ in 0..12 {
        game.step(&[(holder, push(heading))], TICK_DT, &mut events);
        let at = game.player(holder).unwrap().run;
        if (at.x - gate_x(1, game.time)).abs() > GATE_GAP / 2.0 {
            assert!((at.z - gate_z) * heading <= -band + 1e-3, "the wall holds them back");
            blocked += 1;
        }
    }
    assert!(blocked > 0, "they met the wall");

    let mut game = BombRelay::new(&contestants(2, false), 3);
    let holder = game.holder.unwrap();
    game.fuse = 100.0;
    let heading = game.heading;
    let gap = gate_x(1, game.time);
    relay_place(&mut game, holder, gap, gate_z - heading * (band + 0.05));
    for _ in 0..30 {
        game.step(&[(holder, push(heading))], TICK_DT, &mut events);
    }
    let at = game.player(holder).unwrap().run;
    assert!((at.z - gate_z) * heading > band, "lined up with the gap, they run through");
}

#[test]
fn relay_bots_run_the_course_until_one_is_left() {
    use crate::relay::BombRelay;
    let mut game = BombRelay::new(&contestants(4, true), 12);
    let mut events = Vec::new();
    let ended = run_bots(&mut game, 4, 100.0, &mut events);
    let placements = game.placements().expect("the fuses ran out");
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
    assert_eq!(placements[0].len(), 1, "one runner is left");
    let booms = events.iter().filter(|e| e["kind"] == "boom").count();
    assert_eq!(booms, 3, "a bomb went off for each runner put out");
    let passes = events.iter().filter(|e| e["kind"] == "pass").count();
    assert!(passes >= 4, "bots make it through the gates: {passes} passes");
    assert!(ended < 70.0, "the shrinking fuse keeps it short: {ended}");
}

#[test]
fn sharp_relay_bots_run_a_leg_faster_than_relaxed_ones() {
    use crate::relay::BombRelay;
    let leg = |skill: f32| {
        let seats = vec![Contestant::bot(0, skill), Contestant::bot(1, skill)];
        let mut total = 0.0;
        for seed in 0..6 {
            let mut game = BombRelay::new(&seats, seed);
            game.fuse = 100.0;
            let mut events = Vec::new();
            let mut seconds = 0.0;
            while !events.iter().any(|e: &Value| e["kind"] == "pass") && seconds < 30.0 {
                let inputs: Vec<(u8, Input)> =
                    (0..2).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
                game.step(&inputs, TICK_DT, &mut events);
                seconds += TICK_DT;
            }
            assert!(seconds < 30.0, "a skill {skill} bot finished a leg");
            total += seconds;
        }
        total / 6.0
    };
    let (sharp, relaxed) = (leg(1.0), leg(0.15));
    assert!(sharp < relaxed, "sharp {sharp} against relaxed {relaxed}");
}

/// Every bout runs with a full room of bots for half a minute, and a
/// Tawkathon sets up with a full room on every board.
#[test]
fn every_bout_and_board_takes_a_full_room() {
    let contestants: Vec<crate::bout::Contestant> = (0..crate::room::MAX_SEATS as u8)
        .map(|seat| crate::bout::Contestant { seat, is_bot: true, skill: 0.5, team: seat % 2, ally: None })
        .collect();
    let mut broken = Vec::new();
    for spec in crate::bout::BOUTS {
        let run = std::panic::catch_unwind(|| {
            let mut game = (spec.create)(&contestants, 7);
            let mut events = Vec::new();
            for _ in 0..(60 * 30) {
                let inputs: Vec<(u8, crate::bout::Input)> =
                    (0..crate::room::MAX_SEATS as u8).map(|seat| (seat, game.bot_input(seat, 1.0 / 60.0))).collect();
                game.step(&inputs, 1.0 / 60.0, &mut events);
                let _ = game.snapshot();
                if game.placements().is_some() {
                    break;
                }
            }
        });
        if run.is_err() {
            broken.push(spec.key);
        }
    }
    for map in board::Map::ALL {
        let seats: Vec<u8> = (0..crate::room::MAX_SEATS as u8).collect();
        if std::panic::catch_unwind(|| {
            let board = Board::on(map, &seats, 5, 3);
            let _ = board.view_for(None);
        })
        .is_err()
        {
            broken.push(map.label());
        }
    }
    assert!(broken.is_empty(), "broken with a full room: {broken:?}");
}

#[test]
fn the_sandbox_is_a_board_with_no_bouts_and_bots_that_stand_still() {
    let mut room = Room::new("sandbox", 3);
    room.connect("dev", "Dev", "").unwrap();
    assert_eq!(room.start_sandbox("dev"), Err(crate::room::RoomError::NoDevTools), "only on a dev stack");
    room.dev_tools = true;
    room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
    assert!(room.members.len() >= 4, "bots come along as targets");
    let me = room.seat_of("dev").unwrap();
    assert_eq!(room.board.as_ref().unwrap().turn_seat(), me, "and you go first");

    use crate::room::DevAction;
    room.dev("dev", DevAction::Give { item: Item::Bomb }).unwrap();
    room.dev("dev", DevAction::Coins { coins: 50 }).unwrap();
    room.dev("dev", DevAction::Teleport { space: 9, seat: None }).unwrap();
    if !matches!(room.board.as_ref().unwrap().step(), TurnStep::Items) {
        // Space 9 asked something of us, as a shop would: answer and carry on.
        let board = room.board.as_mut().unwrap();
        match board.step().clone() {
            TurnStep::Shop { .. } => board.shop(me, None).unwrap(),
            TurnStep::Cannon { .. } => board.fire(me, 1).unwrap(),
            _ => {}
        }
    }
    let bot = room.members.iter().find(|m| m.is_bot).unwrap().seat;
    room.dev("dev", DevAction::Teleport { space: 4, seat: Some(bot) }).unwrap();
    assert_eq!(room.board.as_ref().unwrap().view().tokens.iter().find(|t| t.seat == bot).unwrap().space, 4, "a bot dragged to a space");
    let mine = room.board.as_ref().unwrap().view().tokens.into_iter().find(|t| t.seat == me).unwrap();
    assert!(mine.items.contains(&Item::Bomb));
    assert_eq!((mine.space, mine.coins >= 50), (9, true));
    let bots: Vec<(u8, u8)> = room.board.as_ref().unwrap().view().tokens.iter().filter(|t| t.seat != me).map(|t| (t.seat, t.space)).collect();

    // A few laps of turns: the board never goes to a vote, the bots never
    // move, and the turn keeps coming back round.
    for _ in 0..3 {
        room.board.as_mut().unwrap().roll(me).unwrap();
        for _ in 0..(60 * 30) {
            room.step();
            let board = room.board.as_mut().unwrap();
            if board.turn_seat() != me {
                continue;
            }
            // Your own turn waits for you in the sandbox, at a fork or a shop too.
            match board.step().clone() {
                TurnStep::Items => break,
                TurnStep::Branch { options, .. } => board.choose_branch(me, options[0].node).unwrap(),
                TurnStep::Shop { .. } => board.shop(me, None).unwrap(),
                TurnStep::Cannon { .. } => board.fire(me, 1).unwrap(),
                _ => {}
            }
        }
        assert_eq!(room.phase, Phase::Board, "no bouts");
        assert_eq!(room.board.as_ref().unwrap().turn_seat(), me, "back to you");
    }
    let after: Vec<(u8, u8)> = room.board.as_ref().unwrap().view().tokens.iter().filter(|t| t.seat != me).map(|t| (t.seat, t.space)).collect();
    assert_eq!(bots, after, "the bots stood still");
    room.dev("dev", DevAction::Exit).unwrap();
    assert_eq!(room.phase, Phase::Lobby);
}

#[test]
fn rocker_rally_steers_the_way_the_stick_points() {
    let mut game = RockerRally::new(&contestants(1, false), 5);
    let mut events = Vec::new();
    let x = |game: &RockerRally| game.snapshot()["players"][0]["x"].as_f64().unwrap();
    let start = x(&game);
    for _ in 0..30 {
        game.step(&[(0, Input::from_stick(Stick { x: 1.0, z: 0.0 }))], 1.0 / 60.0, &mut events);
    }
    assert!(x(&game) > start, "right steers to +x, the right of the screen");
    for _ in 0..60 {
        game.step(&[(0, Input::from_stick(Stick { x: -1.0, z: 0.0 }))], 1.0 / 60.0, &mut events);
    }
    assert!(x(&game) < start, "and left to the left");
}

#[test]
fn jumping_to_a_shop_in_the_sandbox_opens_it() {
    let mut room = Room::new("sandbox-shop", 3);
    room.connect("dev", "Dev", "").unwrap();
    room.dev_tools = true;
    room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
    let shop = room.board.as_ref().unwrap().view().spaces.iter().find(|s| s.kind == crate::board::SpaceKind::Shop).unwrap().id;
    room.dev("dev", crate::room::DevAction::Teleport { space: shop, seat: None }).unwrap();
    assert!(matches!(room.board.as_ref().unwrap().step(), TurnStep::Shop { .. }), "the shop opens as if you walked in");
}

#[test]
fn a_sandbox_jump_after_done_still_lands_in_the_storm_and_logs_the_catch() {
    let mut room=Room::new("sandbox-storm-jump",3);
    room.connect("dev","Dev","").unwrap();
    room.dev_tools=true;
    room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
    let seat=room.seat_of("dev").unwrap();
    let storm=12;
    let board=room.board.as_mut().unwrap();
    board.place_for_test(seat,0,20);
    board.land_for_test(seat);
    assert_eq!(board.step(),&TurnStep::Done);
    let before_coins=token_of(board,seat).coins;
    board.storm_for_test(storm);
    board.take_events();
    room.dev("dev",crate::room::DevAction::Teleport{space:storm,seat:None}).unwrap();
    let board=room.board.as_mut().unwrap();
    let token=token_of(board,seat);
    assert_ne!(token.space,storm,"the catch blows the jumper back");
    assert_eq!(token.coins,before_coins-crate::board::STORM_COINS);
    assert_eq!(board.step(),&TurnStep::Items,"sandbox stays interactive after the catch");
    let events=board.take_events();
    let jump=events.iter().position(|event|event["kind"]=="dev_teleport").unwrap();
    let blown=events.iter().position(|event|event["kind"]=="blown").unwrap();
    assert!(jump<blown,"renderer sees the jump to the storm before the blowback");
    assert_eq!(events[jump]["space"],storm);
    assert_eq!(events[blown]["from"],storm);
    assert_eq!(events[blown]["to"],token.space);
    let other=board.view().tokens.iter().find(|token|token.seat!=seat).unwrap().seat;
    let other_coins=token_of(board,other).coins;
    room.dev("dev",crate::room::DevAction::Teleport{space:storm,seat:Some(other)}).unwrap();
    let board=room.board.as_mut().unwrap();
    assert_eq!(token_of(board,other).space,storm,"dragging a rival is placement, not a landing");
    assert_eq!(token_of(board,other).coins,other_coins);
    assert!(board.take_events().iter().all(|event|event["kind"]!="blown"));
}

#[test]
fn mature_visuals_do_not_enable_round_wide_modifiers() {
    for mature in [false,true] {
        for seed in 0..12 {
            let mut board=Board::new(&seats(4),6,seed);
            board.mature=mature;
            for _ in 0..5 {
                run_board_round(&mut board);
                board.finish_round(&[vec![0],vec![1],vec![2],vec![3]]);
                assert_eq!(board.view().event,None);
                assert_eq!(board.view().bounty,None);
                assert_eq!(board.view().plague,None);
                assert_eq!(board.view().underdog,None);
            }
        }
    }
}

#[test]
fn a_bounty_pays_whoever_hits_the_leader_first() {
    let (mut board, me, them) = duel(Item::Bat, 4);
    board.bounty_for_test(them);
    let before = token_of(&board, me).coins;
    bat(&mut board, me, them);
    assert_eq!(token_of(&board, me).coins, before + crate::board::BOUNTY_COINS);
    assert_eq!(board.view().bounty, None, "paid once");
}

#[test]
fn the_plague_bites_every_turn_and_passes_on_by_landing_beside_someone() {
    let mut board = Board::new(&seats(2), 6, 8);
    let first = board.turn_seat();
    let other = if first == 0 { 1 } else { 0 };
    board.plague_for_test(other);
    board.end_turn();
    assert_eq!(token_of(&board, other).health, MAX_HEALTH - 1, "it bites at the start of their turn");
    let here = token_of(&board, first).space;
    board.move_for_test(other, here);
    board.land_for_test(other);
    assert_eq!(board.view().plague, Some(first), "and passes to whoever they land beside");
}

#[test]
fn vacuuming_requires_precise_aim_even_at_close_range() {
    use crate::board::vacuum_strength;
    let target = |distance: f32, degrees: f32| {
        let angle = degrees.to_radians();
        (angle.sin() * distance, angle.cos() * distance)
    };
    let centre = vacuum_strength((0.0, 0.0), 0.0, target(5.0, 0.0), 1.0).unwrap();
    let edge = vacuum_strength((0.0, 0.0), 0.0, target(5.0, 12.0), 1.0).unwrap();
    assert!(edge < centre && edge >= centre*0.6, "the visible edge still has usable strength while the centre pulls faster");
    assert!(vacuum_strength((0.0, 0.0), 0.0, target(5.0, 20.0), 1.0).is_none(), "a broad sweep misses");
    assert!(vacuum_strength((0.0, 0.0), 0.0, target(0.5, 180.0), 1.0).is_none(), "nearby targets behind you still need aiming at");
    assert!(vacuum_strength((0.0, 0.0), 0.0, target(0.5, 0.0), 1.0).is_some());
}

#[test]
fn the_vacuum_pulls_in_what_it_points_at_and_a_thrown_crate_stops_whoever_walks_into_it() {
    let (mut board, me, them) = duel(Item::Bat, 20);
    board.clear_crates_for_test();
    board.clear_air_for_test();
    board.place_for_test(them, 30, 0);
    board.loose_for_test(2, 5);
    board.crate_for_test(3);
    board.loose_for_test(40, 7);
    let before = token_of(&board, me).coins;
    board.suck(me).unwrap();
    let aim = |board: &Board, space: u8| {
        let (a, b) = (&crate::board::board()[token_of(board, me).space as usize], &crate::board::board()[space as usize]);
        (b.x - a.x).atan2(b.z - a.z)
    };
    // Pointed away, nothing comes; pointed at the pile and the crate, they do.
    board.aim_vacuum(me, aim(&board, 2) + std::f32::consts::PI, true).unwrap();
    for _ in 0..60 {
        board.vacuum_tick(1.0 / 60.0);
    }
    assert_eq!(token_of(&board, me).coins, before, "nothing behind you");
    board.aim_vacuum(me, aim(&board, 2), true).unwrap();
    for _ in 0..120 {
        board.vacuum_tick(1.0 / 60.0);
    }
    assert_eq!(token_of(&board, me).coins, before + 5, "the pile it points at, not the one across the board");
    assert!(token_of(&board, me).carrying, "and the crate behind it");
    for _ in 0..(60 * (crate::board::VACUUM_SECONDS as usize+1)) {
        board.vacuum_tick(1.0 / 60.0);
    }
    assert_eq!(*board.step(), TurnStep::Items, "the twenty-second battery is eventually put away");
    assert_eq!(board.suck(me), Err(BoardError::WrongStep), "once a turn");

    board.throw_crate(me).unwrap();
    board.calm_for_test();
    throw_at(&mut board, me, 4);
    assert_eq!(board.view().blockades, vec![4], "a barricade where it landed");
    board.end_turn();
    board.place_for_test(them, 2, 20);
    board.give_for_test(them, Item::GoldenDie);
    use_now(&mut board, them, Item::GoldenDie, Some(5));
    board.roll(them).unwrap();
    assert_eq!(token_of(&board, them).space, 4, "stopped at the barricade, short of a roll of five");
    assert!(board.view().blockades.is_empty(), "and smashed it");
}

#[test]
fn the_chest_is_won_by_vacuuming_it_in_and_rivals_are_drained() {
    let (mut board, me, them) = duel(Item::Bat, 20);
    board.clear_crates_for_test();
    board.clear_air_for_test();
    let prize = board.view().prize_at;
    // Stood right on the prize spot with the coins for it: it is not bought by landing.
    board.place_for_test(me, prize, 40);
    board.land_for_test(me);
    assert_eq!(token_of(&board, me).prizes, 0, "landing does not buy it");
    board.place_for_test(them, 60, 0);
    board.end_turn();
    board.end_turn();
    board.suck(me).unwrap();
    let at = crate::board::board()[prize as usize].clone();
    let mut won = false;
    for _ in 0..(60 * 5) {
        // Follow the chest as it drifts.
        let out = board.view().vacuum.map(|v| v.out).unwrap_or(0.0);
        let (ox, oz) = Board::chest_drift(out);
        let here = &crate::board::board()[token_of(&board, me).space as usize];
        board.aim_vacuum(me, (at.x + ox - here.x).atan2(at.z + oz - here.z), true).ok();
        board.vacuum_tick(1.0 / 60.0);
        if token_of(&board, me).prizes == 1 {
            won = true;
            break;
        }
    }
    assert!(won, "kept in the cone, the chest comes in");
    assert_eq!(token_of(&board, me).coins, 40 - board.prize_cost(me).min(40), "and is paid for");

    // A rival in the cone is drained a coin at a time.
    let (mut board, me, them) = duel(Item::Bat, 20);
    board.clear_crates_for_test();
    board.clear_air_for_test();
    board.place_for_test(them, 2, 10);
    board.suck(me).unwrap();
    let (a, b) = (&crate::board::board()[0], &crate::board::board()[2]);
    board.aim_vacuum(me, (b.x - a.x).atan2(b.z - a.z), true).unwrap();
    for _ in 0..60 {
        board.vacuum_tick(1.0 / 60.0);
    }
    assert!(token_of(&board, them).coins < 10, "coins drawn out of them");
    assert_eq!(token_of(&board, me).coins + token_of(&board, them).coins, 30, "and into you");
}

#[test]
fn a_bat_knocks_coins_out_across_the_board() {
    let (mut board, me, them) = duel(Item::Bat, 20);
    board.place_for_test(them, 0, 20);
    bat(&mut board, me, them);
    let scattered: i32 = board.view().loose.iter().map(|row| row.coins).sum();
    assert_eq!(scattered, 6, "a perfect swing knocks six loose");
    assert_eq!(token_of(&board, them).coins, 14);
}

#[test]
fn the_sandbox_vacuum_never_runs_out() {
    let mut room = Room::new("sandbox-vacuum", 3);
    room.connect("dev", "Dev", "").unwrap();
    room.dev_tools = true;
    room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
    let me = room.seat_of("dev").unwrap();
    let board = room.board.as_mut().unwrap();
    board.suck(me).unwrap();
    board.aim_vacuum(me, 0.0, true).unwrap();
    for _ in 0..(60 * 20) {
        board.vacuum_tick(1.0 / 60.0);
    }
    assert!(matches!(board.step(), TurnStep::Vacuum { .. }), "still out after twenty seconds of sucking");
    board.stow_vacuum(me).unwrap();
    board.suck(me).unwrap();
    assert!(matches!(board.step(), TurnStep::Vacuum { .. }), "and out again the same turn");
}

#[test]
fn cosmetic_facing_is_only_for_the_authenticated_member_and_changes_no_gameplay() {
    let mut room = Room::new("board-facing", 3);
    room.connect("dev", "Dev", "").unwrap();
    room.connect("friend", "Friend", "").unwrap();
    room.face("dev", 1.0).unwrap();
    room.aim_vacuum("dev", 0.0, false, false).unwrap();
    room.aim_walk_vacuum("dev", 0.0, 0.0, false).unwrap();
    assert!(room.board.is_none(), "late cosmetic/held input cannot create a board in the lobby");
    room.dev_tools = true;
    room.start_sandbox("dev").unwrap();room.finish_candy_offer_for_test();
    let mine = room.seat_of("dev").unwrap();
    let before = room.board.as_ref().unwrap().view();
    room.face("dev", -1.0).unwrap();
    let after = room.board.as_ref().unwrap().view();
    for (old, new) in before.tokens.iter().zip(&after.tokens) {
        assert_eq!((old.space,old.previous,old.coins,old.health), (new.space,new.previous,new.coins,new.health));
        if new.seat == mine {
            assert!((new.facing.unwrap() - (std::f32::consts::TAU - 1.0)).abs() < 0.001);
        } else { assert_eq!(new.facing, old.facing, "another player's facing is untouched"); }
    }
    assert!(room.face("dev", f32::NAN).is_err());
    assert!(room.face("dev", f32::INFINITY).is_err());
    assert!(room.face("unknown-user", 0.0).is_err());
}

#[test]
fn board_events_keep_distinct_monotonic_identity_after_the_log_rolls_over() {
    let mut board = Board::new(&[0,1], 6, 31);
    for _ in 0..60 { board.dev_coins(0, 0); board.dev_heal(0); board.dev_place(0, 0, false); }
    let view=board.view();
    assert_eq!(view.log.len(),40);
    let sequence:Vec<u64>=view.log.iter().map(|entry|entry["seq"].as_u64().expect("every event has an identity")).collect();
    assert!(sequence.windows(2).all(|pair|pair[1]>pair[0]));
    assert!(sequence[0]>1,"old log entries were evicted without reusing identities");
}

#[test]
fn pairs_share_a_result_and_never_hurt_each_other() {
    let mut room = Room::new("pairs", 5);
    room.connect("a", "A", "").unwrap();
    for _ in 0..3 {
        room.add_bot("a").unwrap();
    }
    room.choose_pairs("a", true).unwrap();
    room.start_party("a").unwrap();
    let pairs = room.view().pairs;
    assert_eq!(pairs.len(), 4, "four seats in two pairs");
    let me = room.seat_of("a").unwrap();
    let my_pair = pairs.iter().find(|p| p.seat == me).unwrap().team;
    let partner = pairs.iter().find(|p| p.seat != me && p.team == my_pair).unwrap().seat;

    // A bout's places: partners take the better of the two.
    let rivals: Vec<u8> = pairs.iter().filter(|p| p.team != my_pair).map(|p| p.seat).collect();
    let shared = room.share_places_for_test(vec![vec![rivals[0]], vec![me], vec![rivals[1]], vec![partner]]);
    assert_eq!(shared[0].len(), 2, "the leader's partner rises with them");
    assert!(shared[1].contains(&me) && shared[1].contains(&partner), "and mine with me");

    // On the board, a bat swung at your partner does nothing to them.
    let board = room.board.as_mut().unwrap();
    board.place_for_test(me, 0, 20);
    board.place_for_test(partner, 0, 20);
    for &rival in &rivals {
        board.place_for_test(rival, 60, 20);
    }
    board.turn_to_for_test(me);
    board.give_for_test(me, Item::Bat);
    board.use_item(me, Item::Bat, None).unwrap();
    board.activate_item(me).unwrap();
    board.swing(me, 0.0, 0.5).unwrap();
    let mate = board.view().tokens.into_iter().find(|t| t.seat == partner).unwrap();
    assert_eq!((mate.health, mate.coins, mate.space), (MAX_HEALTH, 20, 0), "your partner is never hit");
}

#[test]
fn a_practice_go_comes_before_each_bout_and_ends_when_everyone_is_ready() {
    let mut room = Room::new("practice", 5);
    room.connect("alice", "Alice", "").unwrap();
    room.connect("bob", "Bob", "").unwrap();
    room.start("alice").unwrap();
    assert!(matches!(room.phase, Phase::Practice { .. }));
    for _ in 0..60 {
        room.step();
    }
    assert!(matches!(room.phase, Phase::Practice { .. }), "it waits for everyone");
    room.ready("alice").unwrap();
    room.step();
    assert!(matches!(room.phase, Phase::Practice { .. }), "bob is not ready yet");
    room.ready("bob").unwrap();
    room.step();
    assert!(matches!(room.phase, Phase::Countdown { .. }), "and then it counts in");
}

#[test]
fn opening_minigame_funds_players_and_sets_order_without_using_up_round_one() {
    let mut board=Board::for_room(board::Map::Frozen,&seats(4),10,900);
    board.prepare_opening();
    let before=board.view();let spots:Vec<_>=before.tokens.iter().map(|t|t.space).collect();
    assert_eq!(spots.iter().copied().collect::<std::collections::HashSet<_>>().len(),4);
    assert!(before.tokens.iter().all(|t|t.coins==0&&t.items.is_empty()&&t.prizes==0));
    assert!(spots.iter().all(|id|{let s=&before.spaces[*id as usize];!s.gated&&!s.under&&matches!(s.kind,board::SpaceKind::Coin|board::SpaceKind::Start)}));
    board.finish_opening(&[vec![2],vec![0],vec![3],vec![1]]);
    let after=board.view();assert_eq!(after.round,1);assert_eq!(after.order,vec![2,0,3,1]);assert_eq!(after.turn_seat,2);
    assert_eq!(after.tokens.iter().map(|t|t.space).collect::<Vec<_>>(),spots);
    assert_eq!([2,0,3,1].map(|seat|token_of(&board,seat).coins),[25,20,15,10]);
    assert!(after.tokens.iter().all(|t|t.items.len()==1));
    assert!(board::item_tier_one(token_of(&board,2).items[0]));
    assert_eq!(after.crawler.unwrap().controller,Some(1));
}

#[test]
fn item_lottery_scales_with_player_count_and_keeps_last_place_at_twenty_percent() {
    for players in 2..=8 {
        assert_eq!(board::tier_one_chance(0,players),1.);
        assert!((board::tier_one_chance(players-1,players)-0.2).abs()<0.00001);
        for place in 1..players {assert!(board::tier_one_chance(place,players)<board::tier_one_chance(place-1,players));}
    }
    let mut premium=0;
    for seed in 0..2000 {
        let mut board=Board::new(&seats(4),10,seed);
        board.finish_opening(&[vec![0],vec![1],vec![2],vec![3]]);
        assert!(board::item_tier_one(token_of(&board,0).items[0]));
        premium+=usize::from(board::item_tier_one(token_of(&board,3).items[0]));
    }
    assert!((330..470).contains(&premium),"last-place lottery observed {premium}/2000 tier-one draws");
}

#[test]
fn later_minigames_pay_only_first_and_everyone_receives_an_item_even_with_a_full_bag() {
    let mut board=Board::new(&seats(4),3,901);
    for seat in 0..4 {board.place_for_test(seat,0,30);for _ in 0..MAX_ITEMS {board.give_for_test(seat,Item::Medkit);}}
    let before=board.view();board.finish_round(&[vec![2],vec![0,3],vec![1]]);
    assert_eq!([0,1,2,3].map(|seat|token_of(&board,seat).coins),[30,30,40,30]);
    let view=board.view();assert!(view.tokens.iter().all(|t|t.items.len()==MAX_ITEMS));
    assert_eq!(view.log.iter().filter(|e|e["kind"]=="award"&&e["item"].is_string()).count(),4);
    assert!(view.log.iter().filter(|e|e["kind"]=="award").all(|e|e["replaced"]=="medkit"));
    assert_eq!(view.order,vec![2,0,3,1]);assert_eq!(before.round+1,view.round);
}

#[test]
fn random_starts_are_seeded_and_only_chosen_once() {
    for map in board::Map::ALL {
        let mut first=Board::for_room(map,&seats(8),10,902);first.prepare_opening();
        let mut same=Board::for_room(map,&seats(8),10,902);same.prepare_opening();
        let mut other=Board::for_room(map,&seats(8),10,903);other.prepare_opening();
        let positions=|b:&Board|b.view().tokens.iter().map(|t|t.space).collect::<Vec<_>>();
        assert_eq!(positions(&first),positions(&same));assert_ne!(positions(&first),positions(&other));
        assert_eq!(positions(&first).iter().copied().collect::<std::collections::HashSet<_>>().len(),8);
    }
}

#[test]
fn real_party_opening_results_show_the_exact_rewards_then_enter_round_one_once() {
    let mut room=Room::new("opening-lifecycle",904);
    room.connect("host","Host","").unwrap();for _ in 0..3 {room.add_bot("host").unwrap();}
    room.start_party("host").unwrap();
    assert!(matches!(room.phase,Phase::Vote{..}));assert!(room.view().opening_minigame);
    let spots:Vec<_>=room.view().board.unwrap().tokens.iter().map(|t|t.space).collect();
    for member in &mut room.members {member.is_bot=true;}
    let mut rewards=None;let mut order=None;
    for _ in 0..20000 {
        room.step();
        if let Phase::Results{placements,..}=&room.phase {
            if rewards.is_none(){
                rewards=Some(room.view().minigame_rewards);order=Some(placements.iter().flatten().copied().collect::<Vec<_>>());
                assert!(room.view().board.unwrap().tokens.iter().all(|t|t.coins==0&&t.items.is_empty()));
            }
        }
        if room.phase==Phase::Board {break;}
    }
    assert_eq!(room.phase,Phase::Board);assert!(!room.view().opening_minigame);
    let board=room.view().board.unwrap();assert_eq!(board.round,1);assert_eq!(board.order,order.unwrap());
    assert_eq!(board.tokens.iter().map(|t|t.space).collect::<Vec<_>>(),spots);
    for reward in rewards.unwrap(){let token=board.tokens.iter().find(|t|t.seat==reward.seat).unwrap();assert_eq!(token.coins,reward.coins);assert_eq!(token.items.len(),1);}
    assert_eq!(board.log.iter().filter(|e|e["kind"]=="award").count(),4,"one draw each, once");
}
