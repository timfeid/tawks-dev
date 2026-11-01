//! The boards beyond the island, and the rules only they have.

use crate::board::{
    self, BotMove, Board, Item, LATE_ROUNDS, LOSS_SPACE, Map, RoundEvent, SpaceKind, TIDE_COINS, TIDE_FIND,
    TurnStep, Upgrade,
};
use crate::tests::{run_board_round, use_now};

fn seats(n: u8) -> Vec<u8> {
    (0..n).collect()
}

fn coins_of(board: &Board, seat: u8) -> i32 {
    board.view().tokens.iter().find(|t| t.seat == seat).unwrap().coins
}

fn first_of(map: Map, kind: SpaceKind) -> u8 {
    map.layout().iter().find(|s| s.kind == kind).expect("the board has one").id
}

#[test]
fn every_space_on_every_board_can_be_reached_from_the_start() {
    for map in Map::ALL {
        let layout = map.layout();
        for space in layout {
            assert!(
                board::distance_in(layout, 0, space.id) < u8::MAX,
                "{map:?} space {} can be reached from the start",
                space.id
            );
        }
        for spot in board::prize_spots_in(layout) {
            assert!((spot as usize) < layout.len(), "{map:?} prize spot {spot} is on the board");
        }
    }
}

#[test]
fn every_board_has_a_name_and_its_own_layout() {
    for (i, map) in Map::ALL.iter().enumerate() {
        assert!(!map.label().is_empty());
        for other in &Map::ALL[i + 1..] {
            assert_ne!(map.label(), other.label());
            assert!(!std::ptr::eq(map.layout(), other.layout()), "{map:?} and {other:?} share a layout");
        }
    }
}

#[test]
fn shipwreck_cove_has_tide_spaces_along_the_beach() {
    let layout = Map::Shipwreck.layout();
    let tides: Vec<_> = layout.iter().filter(|s| s.kind == SpaceKind::Tide).collect();
    assert!(tides.len() >= 5, "enough tide spaces to matter");
    // The beach is the near side of the cove; the hulls are the far side.
    assert!(tides.iter().all(|s| s.z > 0.0));
    assert!(Map::ALL.iter().filter(|map| **map != Map::Shipwreck).all(|map| map.layout().iter().all(|s| s.kind != SpaceKind::Tide)));
}

#[test]
fn the_tide_comes_in_on_even_rounds_and_costs_whoever_is_stood_in_it() {
    let mut board = Board::on(Map::Shipwreck, &seats(2), 6, 1);
    assert_eq!(board.view().tide, Some(false), "the tide starts out");
    let tide = first_of(Map::Shipwreck, SpaceKind::Tide);
    let dry = first_of(Map::Shipwreck, SpaceKind::Coin);
    board.place_for_test(0, tide, 20);
    board.place_for_test(1, dry, 20);
    board.finish_round(&[vec![0, 1]]);
    assert_eq!(board.view().round, 2);
    assert_eq!(board.view().tide, Some(true));
    assert_eq!(coins_of(&board, 1) - coins_of(&board, 0), TIDE_COINS, "only the one in the water pays");
    let log = board.view().log;
    let turned = log.iter().rev().find(|entry| entry["kind"] == "tide").expect("the tide turning is logged");
    assert_eq!(turned["flooded"], true);
    assert_eq!(turned["caught"], serde_json::json!([0]));
}

#[test]
fn the_tide_going_out_costs_nothing() {
    let mut board = Board::on(Map::Shipwreck, &seats(2), 6, 1);
    board.set_round_for_test(2);
    assert_eq!(board.view().tide, Some(true));
    let tide = first_of(Map::Shipwreck, SpaceKind::Tide);
    let dry = first_of(Map::Shipwreck, SpaceKind::Coin);
    board.place_for_test(0, tide, 20);
    board.place_for_test(1, dry, 20);
    board.finish_round(&[vec![0, 1]]);
    assert_eq!(board.view().round, 3);
    assert_eq!(board.view().tide, Some(false));
    assert_eq!(coins_of(&board, 0), coins_of(&board, 1));
}

#[test]
fn stopping_on_a_tide_space_pays_while_it_is_out_and_soaks_you_while_it_is_in() {
    let tide = first_of(Map::Shipwreck, SpaceKind::Tide);
    let mut board = Board::on(Map::Shipwreck, &seats(2), 6, 3);
    board.place_for_test(0, tide, 20);
    board.land_for_test(0);
    assert_eq!(coins_of(&board, 0), 20 + TIDE_FIND);

    board.set_round_for_test(2);
    board.place_for_test(0, tide, 20);
    board.land_for_test(0);
    assert_eq!(coins_of(&board, 0), 20 - LOSS_SPACE);
}

#[test]
fn boards_without_tide_spaces_have_no_tide() {
    let mut board = Board::on(Map::Island, &seats(2), 4, 1);
    assert_eq!(board.view().tide, None);
    board.finish_round(&[vec![0, 1]]);
    assert_eq!(board.view().tide, None);
    assert!(board.view().log.iter().all(|entry| entry["kind"] != "tide"));
}

#[test]
fn a_party_on_shipwreck_cove_plays_through() {
    let mut board = Board::on(Map::Shipwreck, &seats(4), 4, 11);
    assert_eq!(board.view().map, Map::Shipwreck);
    for round in 0..4 {
        run_board_round(&mut board);
        let more = board.finish_round(&[vec![0], vec![1], vec![2], vec![3]]);
        assert_eq!(more, round < 3);
    }
    assert!(board.view().tokens.iter().all(|token| token.stats.spaces > 0), "everyone got round");
}

/// An ice space with a plain lap space before it and a coin space after it.
fn lone_ice() -> (u8, u8, u8) {
    let layout = Map::Frozen.layout();
    layout
        .iter()
        .filter(|s| s.kind == SpaceKind::Ice && s.next.len() == 1 && !s.gated
            && layout.iter().filter(|from| from.link.is_none() && from.next.contains(&s.id)).count() == 1)
        .find_map(|ice| {
            let before = layout.iter().find(|s| s.next == vec![ice.id])?;
            let after = ice.next[0];
            (layout[after as usize].kind == SpaceKind::Coin).then_some((before.id, ice.id, after))
        })
        .expect("Frozen Peak has an ice space between two plain ones")
}

fn roll_exactly(board: &mut Board, seat: u8, face: u8) {
    board.give_for_test(seat, Item::GoldenDie);
    use_now(board, seat, Item::GoldenDie, Some(face));
    board.roll(seat).unwrap();
}

/// The lap space the summit leaves from, and the first space up it.
fn summit_gate() -> (u8, u8) {
    let layout = Map::Frozen.layout();
    layout
        .iter()
        .filter(|s| !s.gated)
        .find_map(|s| s.next.iter().find(|way| layout[**way as usize].gated).map(|way| (s.id, *way)))
        .expect("Frozen Peak has a gated summit")
}

#[test]
fn frozen_peak_has_ice_and_a_gated_summit_only_it_has() {
    let layout = Map::Frozen.layout();
    assert!(layout.iter().filter(|s| s.kind == SpaceKind::Ice).count() >= 8);
    for ice in layout.iter().filter(|s| s.kind == SpaceKind::Ice) {
        assert_ne!(layout[ice.next[0] as usize].kind, SpaceKind::Ice, "ice at {} slides straight onto more ice", ice.id);
    }
    let gated: Vec<_> = layout.iter().filter(|s| s.gated).collect();
    assert!(gated.len() >= 6, "the summit is a real climb");
    assert!(gated.iter().any(|s| s.kind == SpaceKind::PrizeSpot), "something worth climbing for");
    let (from, first) = summit_gate();
    assert_eq!(layout[from as usize].next[0], from + 1, "the lap stays the first way out");
    assert!(layout[from as usize].next.contains(&first));
    // The summit saves more than any other way round.
    let lap_way = board::distance_in(layout, layout[from as usize].next[0], *gated.last().unwrap().next.first().unwrap());
    assert!(lap_way > gated.len() as u8 + 6, "the summit is a real shortcut");
    for map in Map::ALL.iter().filter(|map| **map != Map::Frozen) {
        assert!(map.layout().iter().all(|s| !s.gated && s.kind != SpaceKind::Ice), "{map:?} has no ice or gates");
    }
}

#[test]
fn stopping_on_ice_carries_you_one_space_further() {
    let (before, ice, after) = lone_ice();
    let mut board = Board::on(Map::Frozen, &seats(2), 6, 5);
    let seat = board.turn_seat();
    board.place_for_test(seat, before, 20);
    roll_exactly(&mut board, seat, 1);
    let token = board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap();
    assert_eq!(token.space, after, "slid off the ice onto the next space");
    assert_eq!(token.coins, 20, "sliding beside floating coins does not collect them");
    let slide = board.view().log.into_iter().find(|entry| entry["kind"] == "slide").expect("the slide is logged");
    assert_eq!(slide["from"], ice);
    assert_eq!(slide["to"], after);
}

#[test]
fn passing_over_ice_does_nothing() {
    let (before, _, after) = lone_ice();
    let mut board = Board::on(Map::Frozen, &seats(2), 6, 5);
    let seat = board.turn_seat();
    board.place_for_test(seat, before, 20);
    roll_exactly(&mut board, seat, 2);
    let token = board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap();
    assert_eq!(token.space, after);
    assert!(board.view().log.iter().all(|entry| entry["kind"] != "slide"));
}

#[test]
fn the_summit_stays_shut_until_the_last_rounds() {
    let rounds = 6;
    let (from, first) = summit_gate();
    let mut board = Board::on(Map::Frozen, &seats(2), rounds, 5);
    assert_eq!(board.view().gates_open_from, Some(rounds + 1 - LATE_ROUNDS));
    assert!(!board.view().gates_open);
    let seat = board.turn_seat();
    board.place_for_test(seat, from, 20);
    roll_exactly(&mut board, seat, 1);
    let token = board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap();
    assert!(!matches!(board.step(), TurnStep::Branch { .. }), "no choice while the gate is shut");
    assert_eq!(token.space, from + 1, "walked on round the lap");

    let mut board = Board::on(Map::Frozen, &seats(2), rounds, 5);
    board.set_round_for_test(rounds - 1);
    assert!(board.view().gates_open);
    let seat = board.turn_seat();
    board.place_for_test(seat, from, 20);
    roll_exactly(&mut board, seat, 1);
    let TurnStep::Branch { options, .. } = board.step().clone() else {
        panic!("the summit is offered once it opens");
    };
    assert!(options.iter().any(|option| option.node == first && option.shortcut));
    board.choose_branch(seat, first).unwrap();
    let token = board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap();
    assert_eq!(token.space, first);
}

#[test]
fn the_prize_never_waits_behind_a_shut_gate() {
    for seed in 0..40 {
        let mut board = Board::on(Map::Frozen, &seats(2), 6, seed);
        for _ in 0..12 {
            let at = board.view().prize_at;
            assert!(!Map::Frozen.layout()[at as usize].gated, "seed {seed} put the prize at {at} behind the gate");
            board.set_event_for_test_full(RoundEvent::PrizeMoves);
        }
    }
    let mut board = Board::on(Map::Frozen, &seats(2), 6, 3);
    board.set_round_for_test(5);
    let mut summit = false;
    for _ in 0..60 {
        board.set_event_for_test_full(RoundEvent::PrizeMoves);
        summit |= Map::Frozen.layout()[board.view().prize_at as usize].gated;
    }
    assert!(summit, "once the gate opens the prize can sit at the summit");
}

#[test]
fn bots_head_for_the_summit_once_it_opens() {
    let (from, first) = summit_gate();
    let mut board = Board::on(Map::Frozen, &seats(2), 6, 5);
    board.set_round_for_test(5);
    let seat = board.turn_seat();
    board.place_for_test(seat, from, 20);
    roll_exactly(&mut board, seat, 3);
    assert_eq!(board.bot_move(), BotMove::Branch { node: first });
}

#[test]
fn bots_play_a_party_on_frozen_peak_without_slipping_through_the_gate() {
    let rounds = 6;
    let mut board = Board::on(Map::Frozen, &seats(4), rounds, 21);
    let layout = Map::Frozen.layout();
    for round in 1..=rounds {
        run_board_round(&mut board);
        if !board.view().gates_open {
            for token in board.view().tokens {
                assert!(!layout[token.space as usize].gated, "seat {} is past the gate in round {round}", token.seat);
            }
        }
        let more = board.finish_round(&[vec![0], vec![1], vec![2], vec![3]]);
        assert_eq!(more, round < rounds);
    }
    assert!(board.view().tokens.iter().all(|token| token.stats.spaces > 0), "everyone got round");
}

#[test]
fn the_boards_are_big_enough_for_a_full_room() {
    // Each board is drawn at WORLD_SCALE, so a lap is long enough that eight
    // tokens spread out instead of piling onto the same few spaces.
    for map in crate::board::Map::ALL {
        let layout = map.layout();
        assert!(layout.len() >= 100, "{map:?} has {} spaces", layout.len());
        assert!(layout.len() < u8::MAX as usize, "{map:?} still fits space ids in a byte");
        let width = layout.iter().map(|s| s.x).fold(f32::MIN, f32::max) - layout.iter().map(|s| s.x).fold(f32::MAX, f32::min);
        let minimum=if map==Map::Frozen {50.0}else{70.0};
        assert!(width > minimum, "{map:?} is {width:.0} across");
    }
}

#[test]
fn tunnels_run_underground_between_two_mouths_above_it() {
    for map in Map::ALL {
        let layout = map.layout();
        let under: Vec<_> = layout.iter().filter(|s| s.under).collect();
        assert!(under.len() >= 6, "{map:?} has a tunnel {} spaces long", under.len());
        for space in layout {
            // A shop or a prize spot is never down in the dark, and nothing
            // forks underground.
            if space.under {
                assert_eq!(space.next.len(), 1, "{map:?} space {} forks underground", space.id);
                assert!(!matches!(space.kind, SpaceKind::Shop | SpaceKind::PrizeSpot | SpaceKind::Start));
            }
            // Every way into or out of a tunnel steps from a space above ground.
            for &next in &space.next {
                let other = &layout[next as usize];
                if space.under != other.under {
                    let outside = if space.under { other } else { space };
                    assert!(!outside.under && outside.next.len() == 1, "{map:?} tunnel mouth at {}", outside.id);
                }
            }
        }
    }
}

/// Takes out the vacuum on `seat`'s turn and holds it on a point until the
/// vacuum is put away. The board's layout is the map's.
fn vacuum_at(board: &mut Board, map: Map, seat: u8, (tx, tz): (f32, f32)) {
    board.turn_to_for_test(seat);
    board.suck(seat).unwrap();
    for _ in 0..(60 * 6) {
        let here = &map.layout()[board.view().tokens.iter().find(|t| t.seat == seat).unwrap().space as usize];
        if board.aim_vacuum(seat, (tx - here.x).atan2(tz - here.z), true).is_err() {
            break;
        }
        board.vacuum_tick(1.0 / 60.0);
    }
}

/// The space nearest a point.
fn space_near(map: Map, (x, z): (f32, f32)) -> u8 {
    map.layout()
        .iter()
        .filter(|s| !s.under)
        .min_by(|a, b| ((a.x - x).powi(2) + (a.z - z).powi(2)).total_cmp(&((b.x - x).powi(2) + (b.z - z).powi(2))))
        .unwrap()
        .id
}

#[test]
fn palm_island_coconuts_are_shaken_down_with_the_vacuum() {
    let mut board = Board::on(Map::Island, &seats(2), 4, 3);
    board.clear_crates_for_test();
    let palm = Map::Island.specials().palms[0];
    let seat = board.turn_seat();
    board.place_for_test(seat, space_near(Map::Island, palm), 0);
    board.place_for_test(if seat == 0 { 1 } else { 0 }, 60, 0);
    vacuum_at(&mut board, Map::Island, seat, palm);
    assert!(board.view().coconuts[0] < crate::board::COCONUTS, "coconuts came down");
    let got: Vec<_> = board.view().log.iter().filter(|e| e["what"] == "coconut").cloned().collect();
    assert!(!got.is_empty());
}

#[test]
fn the_volcano_spits_lava_rocks_that_burn_and_can_be_vacuumed_up() {
    let mut board = Board::on(Map::Volcano, &seats(2), 4, 7);
    let rocks = board.view().lava_rocks;
    assert!(!rocks.is_empty(), "rocks on the board from the start");
    let seat = board.turn_seat();
    board.place_for_test(seat, rocks[0], 20);
    board.land_for_test(seat);
    assert!(board.view().log.iter().any(|e| e["kind"] == "lava_burn"), "stopping on one burns");
    let at = &Map::Volcano.layout()[rocks[0] as usize];
    board.clear_crates_for_test();
    board.place_for_test(if seat == 0 { 1 } else { 0 }, 0, 0);
    vacuum_at(&mut board, Map::Volcano, seat, (at.x, at.z));
    assert!(!board.view().lava_rocks.contains(&rocks[0]), "and vacuumed up");
}

#[test]
fn shipwreck_flotsam_is_vacuumed_in_off_the_water() {
    let mut board = Board::on(Map::Shipwreck, &seats(2), 4, 9);
    board.clear_crates_for_test();
    let point = board.view().flotsam[0];
    let at = Map::Shipwreck.specials().flotsam[point as usize];
    let seat = board.turn_seat();
    board.place_for_test(seat, space_near(Map::Shipwreck, at), 0);
    board.place_for_test(if seat == 0 { 1 } else { 0 }, 60, 0);
    vacuum_at(&mut board, Map::Shipwreck, seat, at);
    assert!(!board.view().flotsam.contains(&point), "pulled in");
}

#[test]
fn frozen_peaks_snowman_makes_a_snowball_that_freezes_whoever_it_hits() {
    let mut board = Board::on(Map::Frozen, &seats(2), 4, 9);
    board.clear_crates_for_test();
    let snowman = Map::Frozen.specials().snowmen[0];
    let seat = board.turn_seat();
    let other = if seat == 0 { 1 } else { 0 };
    let here = space_near(Map::Frozen, snowman);
    board.place_for_test(seat, here, 0);
    board.place_for_test(other, 0, 0);
    vacuum_at(&mut board, Map::Frozen, seat, snowman);
    let me = board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap();
    assert!(me.snowball, "packed a snowball");
    assert_eq!(board.view().snowmen, vec![false], "and the snowman is gone this round");
    if matches!(board.step(),TurnStep::Vacuum{..}) {board.stow_vacuum(seat).unwrap();}
    board.throw_snowball(seat).unwrap();
    board.calm_for_test();
    // Thrown straight at the other one, with the power solved for the distance.
    let (from, to) = (&Map::Frozen.layout()[here as usize], &Map::Frozen.layout()[0]);
    let (dx, dz) = (to.x - from.x, to.z - from.z);
    let reach = (dx * dx + dz * dz).sqrt();
    let pitch = std::f32::consts::FRAC_PI_4;
    let power = (0..=2000)
        .map(|step| step as f32 / 2000.0)
        .min_by(|a, b| {
            let off = |p: f32| {
                let ((x, z), _) = Board::bomb_landing((from.x, from.z), dx.atan2(dz), pitch, p, (0.0, 0.0));
                (((x - from.x).powi(2) + (z - from.z).powi(2)).sqrt() - reach).abs()
            };
            off(*a).total_cmp(&off(*b))
        })
        .unwrap();
    board.lob(seat, dx.atan2(dz), pitch, power).unwrap();
    let them = board.view().tokens.into_iter().find(|t| t.seat == other).unwrap();
    assert_eq!(them.resting, 1, "frozen for a turn");
}

#[test]
fn the_storm_can_be_dragged_and_item_boxes_vacuumed() {
    let mut board = Board::on(Map::Island, &seats(2), 6, 4);
    board.clear_crates_for_test();
    board.clear_air_for_test();
    let seat = board.turn_seat();
    let other = if seat == 0 { 1 } else { 0 };
    board.place_for_test(other, 60, 0);
    // An item space's box, from a space or two away.
    let item = Map::Island.layout().iter().find(|s| s.kind == SpaceKind::Item && !s.under).unwrap();
    let beside = Map::Island.layout()[item.id as usize].next[0];
    board.place_for_test(seat, beside, 0);
    let target=board.box_spot_for_test(item.id);
    vacuum_at(&mut board, Map::Island, seat, target);
    assert!(board.view().boxes_taken.contains(&item.id), "the box came in");
    assert_eq!(board.view().tokens.into_iter().find(|t| t.seat == seat).unwrap().items.len(), 1, "with an item in it");

    board.storm_for_test(20);
    board.place_for_test(seat, 23, 0);
    let storm = &Map::Island.layout()[20];
    vacuum_at(&mut board, Map::Island, seat, (storm.x, storm.z));
    assert_ne!(board.view().storm, Some(20), "the storm was dragged nearer");
}

#[test]
fn no_two_spaces_overlap_on_any_board() {
    for map in Map::ALL {
        let layout = map.layout();
        for a in layout {
            for b in layout.iter().filter(|b| b.id > a.id) {
                // A tunnel runs under the land, so it may pass under a space.
                if a.under != b.under {
                    continue;
                }
                let apart = ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt();
                assert!(apart >= 1.5, "{map:?} spaces {} and {} are {apart:.2} apart", a.id, b.id);
            }
        }
    }
}

#[test]
fn the_vacuum_pulls_hardest_dead_centre_and_close() {
    let centre = board::vacuum_strength((0.0, 0.0), 0.0, (0.0, 4.0), 1.0).unwrap();
    let angle = board::VACUUM_CONE * 0.9;
    let edge = board::vacuum_strength((0.0, 0.0), 0.0, (4.0 * angle.sin(), 4.0 * angle.cos()), 1.0).unwrap();
    let far = board::vacuum_strength((0.0, 0.0), 0.0, (0.0, 13.0), 1.0).unwrap();
    assert!(centre > edge && centre > far, "centre {centre}, edge {edge}, far {far}");
    assert!(board::vacuum_strength((0.0, 0.0), 0.0, (0.0, -4.0), 1.0).is_none(), "nothing behind it");
    assert!(board::vacuum_strength((0.0, 0.0), 0.0, (0.0, 15.0), 1.0).is_none(), "nothing out of reach");
}

/// Coins a rival standing a few spaces in front loses to three seconds of
/// sucking, bracing or not.
fn drained(brace: bool) -> i32 {
    let mut board = Board::on(Map::Island, &seats(2), 4, 3);
    board.clear_crates_for_test();
    let layout = Map::Island.layout();
    let (from, to) = layout
        .iter()
        .filter(|a| !a.under)
        .flat_map(|a| layout.iter().filter(|b| !b.under && b.id != a.id).map(move |b| (a, b)))
        .find(|(a, b)| ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt() > 5.0 && ((a.x - b.x).powi(2) + (a.z - b.z).powi(2)).sqrt() < 7.0)
        .unwrap();
    board.place_for_test(0, from.id, 0);
    board.place_for_test(1, to.id, 50);
    board.turn_to_for_test(0);
    board.suck(0).unwrap();
    if brace {
        board.brace(1, true).unwrap();
    }
    for _ in 0..(60 * 3) {
        board.aim_vacuum(0, (to.x - from.x).atan2(to.z - from.z), true).unwrap();
        board.vacuum_tick(1.0 / 60.0);
    }
    50 - coins_of(&board, 1)
}

#[test]
fn a_rival_bracing_against_the_vacuum_keeps_more_coins() {
    let open = drained(false);
    let braced = drained(true);
    assert!(open > 0, "the vacuum drains a rival in front of it");
    assert!(braced < open, "bracing keeps coins: {braced} lost against {open}");
}

#[test]
fn the_vacuum_spins_up_and_the_turn_cannot_brace() {
    let mut board = Board::on(Map::Island, &seats(2), 4, 3);
    board.turn_to_for_test(0);
    board.suck(0).unwrap();
    assert!(board.brace(0, true).is_err(), "the one holding the vacuum has nothing to brace against");
    board.aim_vacuum(0, 0.0, true).unwrap();
    board.vacuum_tick(0.1);
    let early = board.view().vacuum.unwrap().power;
    for _ in 0..30 {
        board.vacuum_tick(1.0 / 60.0);
    }
    let later = board.view().vacuum.unwrap().power;
    assert!(early < 0.5 && later >= 0.99, "spins up: {early} then {later}");
}

/// A space on the island lap a few spaces ahead of `from`, with what lies
/// between: the space walked `steps` ahead along the first way on.
fn ahead_of(from: u8, steps: u8) -> u8 {
    let layout = Map::Island.layout();
    (0..steps).fold(from, |at, _| layout[at as usize].next[0])
}

fn space_of(board: &Board, seat: u8) -> u8 {
    board.view().tokens.iter().find(|t| t.seat == seat).unwrap().space
}

/// Blows at a space from wherever `seat` stands for `seconds`.
fn blow_at(board: &mut Board, seat: u8, target: u8, seconds: f32) {
    let layout = Map::Island.layout();
    for _ in 0..(60.0 * seconds) as usize {
        let (here, there) = (&layout[space_of(board, seat) as usize], &layout[target as usize]);
        if board.aim_vacuum_with(seat, (there.x - here.x).atan2(there.z - here.z), true, true).is_err() {
            break;
        }
        board.vacuum_tick(1.0 / 60.0);
    }
}

/// Two on the island, the first at a quiet stretch of the lap with the second
/// far off, and the first to play.
fn blowing_board() -> (Board, u8) {
    let mut board = Board::on(Map::Island, &seats(2), 4, 3);
    board.clear_crates_for_test();
    let me = 10;
    board.place_for_test(0, me, 20);
    board.place_for_test(1, ahead_of(me, 12), 20);
    board.turn_to_for_test(0);
    (board, me)
}

#[test]
fn blowing_pushes_the_storm_a_space_away() {
    let (mut board, me) = blowing_board();
    let layout = Map::Island.layout();
    let storm = ahead_of(me, 2);
    board.storm_for_test(storm);
    board.suck(0).unwrap();
    blow_at(&mut board, 0, storm, 3.0);
    let now = board.view().storm.unwrap();
    assert_ne!(now, storm, "the storm moved");
    assert!(
        board::gap_in(layout, now, me) > board::gap_in(layout, storm, me),
        "away from you: {storm} to {now}, you at {me}"
    );
}

#[test]
fn blowing_knocks_a_rival_back_once_unless_shielded() {
    let (mut board, me) = blowing_board();
    let them = ahead_of(me, 2);
    board.place_for_test(1, them, 20);
    board.suck(0).unwrap();
    blow_at(&mut board, 0, them, 0.3);
    assert_eq!(space_of(&board, 1), them, "it takes a moment of blowing");
    blow_at(&mut board, 0, them, 4.0);
    assert_eq!(space_of(&board, 1), ahead_of(me, 1), "knocked back a single space along the path, once");
    assert_eq!(coins_of(&board, 1), 20, "blowing takes no coins");

    let (mut board, me) = blowing_board();
    let them = ahead_of(me, 2);
    board.place_for_test(1, them, 20);
    board.raise_shield_for_test(1);
    board.suck(0).unwrap();
    blow_at(&mut board, 0, them, 2.0);
    assert_eq!(space_of(&board, 1), them, "a shield holds them in place");
    assert!(!board.view().tokens.iter().find(|t| t.seat == 1).unwrap().shield, "and is spent");
}

#[test]
fn blowing_scatters_loose_coins_and_sends_the_chest_further_off_once() {
    let (mut board, me) = blowing_board();
    let pile = ahead_of(me, 2);
    board.loose_for_test(pile, 6);
    board.suck(0).unwrap();
    blow_at(&mut board, 0, pile, 1.5);
    let loose = board.view().loose;
    assert!(loose.iter().all(|row| row.space != pile), "the pile is gone from its space");
    assert_eq!(loose.iter().map(|row| row.coins).sum::<i32>(), 6, "but the coins are all still on the board");
    assert_eq!(coins_of(&board, 0), 20, "none of them blown to you");

    let mut board = Board::on(Map::Island, &seats(2), 4, 3);
    board.clear_crates_for_test();
    let prize = board.view().prize_at;
    let layout = Map::Island.layout();
    let mine = layout.iter().find(|s| !s.under && board::distance_in(layout, s.id, prize) == 2).unwrap().id;
    board.place_for_test(0, mine, 0);
    board.place_for_test(1, ahead_of(prize, 10), 0);
    board.turn_to_for_test(0);
    board.suck(0).unwrap();
    let at = layout[prize as usize].clone();
    let here = layout[mine as usize].clone();
    for _ in 0..(60 * 5) {
        let out = board.view().vacuum.map(|v| v.out).unwrap_or(0.0);
        let (ox, oz) = Board::chest_drift(out);
        if board.aim_vacuum_with(0, (at.x + ox - here.x).atan2(at.z + oz - here.z), true, true).is_err() {
            break;
        }
        board.vacuum_tick(1.0 / 60.0);
    }
    let moved = board.view().prize_at;
    assert_ne!(moved, prize, "the chest is blown to another spot");
    assert!(board::distance_in(layout, mine, moved) > 2, "further on from you");
    let blown = board.view().log.iter().filter(|entry| entry["kind"] == "chest_blown").count();
    assert_eq!(blown, 1, "once a turn");
}

#[test]
fn practice_bots_blow_the_storm_away_when_it_bears_down() {
    let (mut board, me) = blowing_board();
    board.storm_for_test(ahead_of(me, 1));
    assert_eq!(board.bot_move(), BotMove::Suck, "the storm is reason enough to take it out");
    board.suck(0).unwrap();
    assert!(matches!(board.bot_move(), BotMove::Vacuum { blow: true, .. }), "and it blows");
}

#[test]
fn upgrades_widen_the_cone_quicken_the_motor_and_last_longer() {
    let (mut plain, _) = blowing_board();
    plain.suck(0).unwrap();
    let (mut kitted, _) = blowing_board();
    for upgrade in [Upgrade::WideNozzle, Upgrade::Turbo, Upgrade::BigBattery] {
        kitted.upgrade_for_test(0, upgrade);
    }
    kitted.suck(0).unwrap();
    assert_eq!(*plain.step(), TurnStep::Vacuum { seconds: board::VACUUM_SECONDS });
    assert_eq!(*kitted.step(), TurnStep::Vacuum { seconds: board::BIG_BATTERY_SECONDS }, "the big battery");
    let (plain_cone, kitted_cone) = (plain.view().vacuum.unwrap().cone, kitted.view().vacuum.unwrap().cone);
    assert!((kitted_cone - plain_cone * board::WIDE_NOZZLE_CONE).abs() < 1e-5, "the wide nozzle: {plain_cone} to {kitted_cone}");
    for board in [&mut plain, &mut kitted] {
        board.aim_vacuum(0, 0.0, true).unwrap();
        for _ in 0..15 {
            board.vacuum_tick(1.0 / 60.0);
        }
    }
    let (slow, fast) = (plain.view().vacuum.unwrap().power, kitted.view().vacuum.unwrap().power);
    assert!(slow < 0.7 && fast >= 0.99, "turbo spins up in half the time: {slow} against {fast}");
    for board in [&mut plain, &mut kitted] {
        for _ in 0..(60 * (board::VACUUM_SECONDS as usize+1)) {
            board.vacuum_tick(1.0 / 60.0);
        }
    }
    assert_eq!(*plain.step(), TurnStep::Items, "twenty seconds and it is done");
    assert!(matches!(kitted.step(), TurnStep::Vacuum { .. }), "thirty-two with the big battery");
}

#[test]
fn the_wide_nozzle_reaches_rivals_off_to_the_side() {
    let drained = |wide: bool| {
        let (mut board, me) = blowing_board();
        let layout = Map::Island.layout();
        let them = ahead_of(me, 2);
        board.place_for_test(1, them, 20);
        if wide {
            board.upgrade_for_test(0, Upgrade::WideNozzle);
        }
        board.suck(0).unwrap();
        let (a, b) = (&layout[me as usize], &layout[them as usize]);
        let off = board::VACUUM_CONE * 1.2;
        for _ in 0..(60 * 3) {
            board.aim_vacuum(0, (b.x - a.x).atan2(b.z - a.z) + off, true).unwrap();
            board.vacuum_tick(1.0 / 60.0);
        }
        20 - coins_of(&board, 1)
    };
    assert_eq!(drained(false), 0, "just outside the plain cone");
    assert!(drained(true) > 0, "inside the wide one");
}

#[test]
fn a_filter_stops_rivals_draining_you_but_not_blowing_you_back() {
    let (mut board, me) = blowing_board();
    let layout = Map::Island.layout();
    let them = ahead_of(me, 2);
    board.place_for_test(1, them, 20);
    board.upgrade_for_test(1, Upgrade::Filter);
    board.suck(0).unwrap();
    let (a, b) = (&layout[me as usize], &layout[them as usize]);
    for _ in 0..(60 * 3) {
        board.aim_vacuum(0, (b.x - a.x).atan2(b.z - a.z), true).unwrap();
        board.vacuum_tick(1.0 / 60.0);
    }
    assert_eq!(coins_of(&board, 1), 20, "nothing drawn out through the filter");
    blow_at(&mut board, 0, them, 1.5);
    assert_ne!(space_of(&board, 1), them, "blowing still knocks them back");
}

#[test]
fn upgrades_are_bought_at_the_shop_and_kept() {
    let (mut board, _) = blowing_board();
    let shop = first_of(Map::Island, SpaceKind::Shop);
    board.place_for_test(0, shop, 60);
    board.land_for_test(0);
    let TurnStep::Shop { upgrades, .. } = board.step().clone() else {
        panic!("the shop opened, not {:?}", board.step());
    };
    assert_eq!(upgrades.len(), Upgrade::ALL.len(), "every upgrade on offer");
    let filter = upgrades.iter().find(|offer| offer.upgrade == Upgrade::Filter).unwrap();
    assert!(upgrades.iter().all(|offer| offer.price <= filter.price), "the filter is the dearest");
    board.shop_with(0, None, Some(Upgrade::Turbo)).unwrap();
    let token = board.view().tokens.into_iter().find(|t| t.seat == 0).unwrap();
    assert_eq!(token.upgrades, vec![Upgrade::Turbo]);
    assert!(token.items.is_empty(), "not in the bag");
    assert_eq!(token.coins, 60 - Upgrade::Turbo.price());
    assert_eq!(*board.step(), TurnStep::Done, "one thing a visit");

    board.place_for_test(0, shop, 60);
    board.land_for_test(0);
    let TurnStep::Shop { upgrades, .. } = board.step().clone() else {
        panic!("the shop opened again");
    };
    assert!(upgrades.iter().all(|offer| offer.upgrade != Upgrade::Turbo), "what you own is not sold again");
}
