use serde_json::Value;

use crate::auth::{self, Claims};
use crate::board::{
    self, BAT_REACH, Board, GRAPPLE_COINS, KNOCKOUT_COINS, MAX_ITEMS, BoardError, BotMove, HAZARD_COINS, Item, MAX_HEALTH, PRIZE_COST,
    START_COINS, SpaceKind, TurnStep, board, distance, gap, prize_spots,
};
use crate::dodgeball::{
    BallState as DodgeBall, COURT_HALF_Z, Dodgeball, OUT_SECONDS, ROUND_SECONDS as BALL_SECONDS,
};
use crate::bout::{BOUTS, Bout, Contestant, Input, Stick, create};
use crate::paint_variant::{PaintPlaylist, PaintVariant};
use crate::paint_variant::{RETAKE_BOMB, RETAKE_DEFUSE, RETAKE_FUSE};
use crate::race::{BOOSTS, FINISH_Z, ROUND_SECONDS as RACE_SECONDS, RockerRally, START_Z};
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
    assert!(matches!(room.phase, Phase::Countdown { .. }));

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
    for _ in 0..3 {
        room.add_bot("host").unwrap();
    }
    assert_eq!(room.add_bot("host"), Err(RoomError::Full));
    room.connect("friend", "Friend", "").unwrap();
    assert_eq!(
        room.members.iter().filter(|m| m.is_bot).count(),
        2,
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
    assert_eq!(room.host_id.as_deref(), Some("b"));
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
fn run_board_round(board: &mut Board) {
    for _ in 0..400 {
        match board.bot_move() {
            BotMove::Item { item, target } => {
                let seat = board.turn_seat();
                let _ = board.use_item(seat, item, target);
            }
            BotMove::Pass => {
                let seat = board.turn_seat();
                board.pass_items(seat).unwrap();
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
    board.use_item(hitter, Item::Bat, Some(1)).unwrap();
    let after = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == 1)
        .cloned()
        .unwrap();
    assert!(after.space < target.space, "knocked back");
    assert_eq!(after.health, MAX_HEALTH - 1, "and it hurt");
}

#[test]
fn a_grapple_moves_coins_from_them_to_you() {
    let mut board = Board::new(&seats(2), 3, 13);
    board.give_for_test(0, Item::Grapple);
    board.use_item(0, Item::Grapple, Some(1)).unwrap();
    let tokens = board.view().tokens;
    assert_eq!(tokens[0].coins, START_COINS + START_COINS.min(GRAPPLE_COINS));
    assert_eq!(tokens[0].coins + tokens[1].coins, START_COINS * 2, "coins only move between players");
}

#[test]
fn losing_your_health_costs_coins_and_a_turn() {
    let mut board = Board::new(&seats(2), 3, 14);
    board.place_for_test(0, 12, 10);
    board.place_for_test(1, 12, 30);
    for _ in 0..MAX_HEALTH {
        board.give_for_test(0, Item::Bat);
        board.use_item(0, Item::Bat, Some(1)).unwrap();
    }
    let token = board.view().tokens.iter().find(|t| t.seat == 1).cloned().unwrap();
    assert_eq!(token.health, MAX_HEALTH, "healed after the knockout");
    assert_eq!(token.coins, 30 - KNOCKOUT_COINS);
    assert_eq!(token.resting, 1, "and sits out a turn");
    let hurts: Vec<_> = board.take_events().into_iter().filter(|e| e["kind"] == "hurt").collect();
    assert_eq!(hurts.len(), MAX_HEALTH as usize, "every hit is reported");
    assert_eq!(hurts.last().unwrap()["health"], 0);
}

#[test]
fn nobody_starts_with_items_and_minigame_winners_earn_one() {
    let mut board = Board::new(&seats(4), 3, 31);
    assert!(board.view().tokens.iter().all(|t| t.items.is_empty()));
    run_board_round(&mut board);
    let before: Vec<usize> = board.view().tokens.iter().map(|t| t.items.len()).collect();
    board.finish_round(&[vec![2], vec![0, 3], vec![1]]);
    let after: Vec<usize> = board.view().tokens.iter().map(|t| t.items.len()).collect();
    assert_eq!(after[2], (before[2] + 1).min(MAX_ITEMS), "the winner drew an item");
    assert_eq!((after[0], after[1], after[3]), (before[0], before[1], before[3]), "nobody else did");
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
    assert_eq!(room.phase, Phase::Board);
    assert!(room.view().board.is_some(), "the board is in the room view");

    // With Alice away every seat plays itself, so the party runs on its own.
    room.disconnect("alice");
    let mut saw_minigame = false;
    let mut saw_board_again = false;
    for _ in 0..(60 * 400) {
        room.step();
        saw_minigame |= room.phase == Phase::Playing;
        if saw_minigame && room.phase == Phase::Board {
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
    assert!(matches!(room.phase, Phase::Countdown { .. }), "everyone voted, so the bout starts");
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

#[test]
fn landing_on_the_prize_buys_it_and_moves_it_on() {
    let mut board = Board::new(&seats(2), 3, 21);
    let spot = board.view().prize_at;
    board.place_for_test(0, spot, 40);
    board.land_for_test(0);
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
fn an_item_only_reaches_someone_close_enough() {
    let mut board = Board::new(&seats(2), 3, 4);
    let hitter = board.turn_seat();
    let mark = if hitter == 0 { 1 } else { 0 };
    let far = board
        .view()
        .spaces
        .iter()
        .map(|space| space.id)
        .find(|id| gap(0, *id) > BAT_REACH)
        .expect("the map is bigger than a bat swing");
    board.place_for_test(hitter, 0, 20);
    board.place_for_test(mark, far, 20);
    board.give_for_test(hitter, Item::Bat);
    assert_eq!(
        board.use_item(hitter, Item::Bat, Some(mark)),
        Err(BoardError::OutOfReach),
        "a rival across the map is out of reach"
    );

    let near = board
        .view()
        .spaces
        .iter()
        .map(|space| space.id)
        .find(|id| *id != 0 && gap(0, *id) <= BAT_REACH)
        .expect("something sits within a swing of the start");
    board.place_for_test(mark, near, 20);
    board.use_item(hitter, Item::Bat, Some(mark)).unwrap();
    let hit = board
        .view()
        .tokens
        .iter()
        .find(|t| t.seat == mark)
        .cloned()
        .unwrap();
    assert_eq!(hit.health, MAX_HEALTH - 1, "the swing landed");
}

#[test]
fn the_board_view_says_how_far_away_everyone_is() {
    let mut board = Board::new(&seats(3), 3, 9);
    let seat = board.turn_seat();
    let view = board.view();
    assert_eq!(view.aim.len(), 2, "every rival is measured");
    for rival in &view.aim {
        let token = view.tokens.iter().find(|t| t.seat == rival.seat).unwrap();
        let me = view.tokens.iter().find(|t| t.seat == seat).unwrap();
        assert_eq!(rival.spaces, gap(me.space, token.space));
    }
    assert!(
        view.reach
            .iter()
            .any(|r| r.item == Item::Bat && r.spaces == BAT_REACH),
        "the bat's reach is on the view"
    );
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
            &[(0, Input::from_stick(Stick { x: side, z: 0.0 }))],
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
        &[(0, Input::from_stick(Stick { x: -1.0, z: 0.0 }))],
        TICK_DT,
        &mut events,
    );
    for _ in 0..9 {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(
        &[(0, Input::from_stick(Stick { x: 1.0, z: 0.0 }))],
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
        &[(0, Input::from_stick(Stick { x: -1.0, z: 0.0 }))],
        TICK_DT,
        &mut events,
    );
    for _ in 0..9 {
        game.step(&[], TICK_DT, &mut events);
    }
    game.step(
        &[(0, Input::from_stick(Stick { x: 1.0, z: 0.0 }))],
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
        15,
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

#[test]
fn the_ledge_horn_leaves_one_player_short_every_round() {
    use crate::ledge::{LedgeShuffle, Phase};
    let mut game = LedgeShuffle::new(&contestants(4, true), 3);
    assert_eq!(game.ledges.len(), 3, "always one ledge short");
    let mut events = Vec::new();
    let mut steps = 0;
    while game.round == 1 && steps < (30.0 / TICK_DT) as usize {
        let inputs: Vec<(u8, Input)> = (0..4).map(|s| (s, game.bot_input(s, TICK_DT))).collect();
        game.step(&inputs, TICK_DT, &mut events);
        steps += 1;
    }
    assert!(
        events.iter().any(|e| e["kind"] == "horn"),
        "the horn sounded"
    );
    assert!(
        events.iter().any(|e| e["kind"] == "over"),
        "somebody missed out"
    );
    assert_eq!(
        game.players.iter().filter(|p| p.playing()).count(),
        3,
        "one player went over"
    );
    assert_eq!(game.ledges.len(), 2, "and a ledge sank for the next round");
    assert_eq!(game.phase, Phase::Roam, "the music started again");
}

#[test]
fn ledge_shuffle_plays_down_to_a_winner() {
    use crate::ledge::LedgeShuffle;
    let mut game = LedgeShuffle::new(&contestants(4, true), 21);
    let mut events = Vec::new();
    run_bots(&mut game, 4, 150.0, &mut events);
    let placements = game.placements().expect("the shuffle finished");
    assert_eq!(
        placements.first().map(|g| g.len()),
        Some(1),
        "one winner is left standing"
    );
    assert_eq!(placements.iter().map(|g| g.len()).sum::<usize>(), 4);
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
    game.step(&[(0, swing)], TICK_DT, &mut events);
    assert!(game.health < TITAN_HEALTH, "the core took a hit");
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
    for i in 0..4 {
        room.connect(&format!("u{i}"), &format!("Player {i}"), "").unwrap();
    }
    assert_eq!(room.max_seats_for_test(), 4);
    assert_eq!(room.connect("u4", "Player 4", ""), Err(RoomError::Full));
    room.choose_game("u0", COVEN_KEY).unwrap();
    assert_eq!(room.max_seats_for_test(), crate::coven::MAX_PLAYERS);
    room.connect("u4", "Player 4", "").unwrap();
    room.connect("u5", "Player 5", "").unwrap();
    assert_eq!(room.choose_game("u0", "bumper"), Err(RoomError::TooManyPlayers));
    room.add_bot("u0").unwrap();
    room.start("u0").unwrap();
    assert_eq!(room.phase, Phase::Table);
    assert!(room.members.iter().all(|m| m.in_round));

    let (game, mine) = room.table_view(room.table_seat_of("u1")).unwrap();
    assert_eq!(game, "coven");
    assert_eq!(mine["players"].as_array().unwrap().len(), 7);
    assert_eq!(mine["hand"].as_array().unwrap().len(), crate::coven::HAND_AT_START);
    assert_eq!(mine["seats"].as_array().unwrap().len(), 7);
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

