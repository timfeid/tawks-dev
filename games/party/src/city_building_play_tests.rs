use super::*;
use crate::city_buildings::room_at;
const DT: f32 = 1. / 60.;
fn game(bots: [bool; 3]) -> CityGame {
    let mut g = CityGame::new(
        &[(0, bots[0]), (1, bots[1]), (2, bots[2])],
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
    a.path.clear();
}
fn me(g: &CityGame, seat: u8) -> &Actor {
    g.actors.iter().find(|a| a.seat == seat).unwrap()
}
/// Holds the stick toward `to` for a while.
fn walk(g: &mut CityGame, seat: u8, to: (f32, f32), seconds: f32) {
    for _ in 0..(seconds * 60.) as usize {
        let from = me(g, seat).at;
        let d = distance(from, to);
        let stick = if d < 0.2 {
            Stick::default()
        } else {
            Stick {
                x: (to.0 - from.0) / d,
                z: (to.1 - from.1) / d,
            }
        };
        g.tick(DT, &[(seat, stick, 0.)], &[0, 1, 2], |_, _| false);
        assert!(
            walkable(me(g, seat).at),
            "never inside a wall: {:?}",
            me(g, seat).at
        );
    }
}

#[test]
fn crew_bots_enter_both_power_and_bank_doors_from_the_plaza() {
    for bot_count in [2u8, 5] {
        let seats: Vec<_> = (0..=bot_count).map(|seat| (seat, seat != 0)).collect();
        let ids: Vec<_> = seats.iter().map(|(seat, _)| *seat).collect();
        let mut g = CityGame::new(&seats, CityConfig::default(), false);
        g.briefing_left = 0.;
        g.time = 3.;
        g.actors[0].side = Side::Crew;
        g.sandbox = true;
        g.tuning.second_post = true;
        g.qa_power_job();
        put(&mut g, 0, (-11.5, 20.5));
        for seat in 1..=bot_count {
            put(&mut g, seat, (-2. + f32::from(seat - 1) * 1.5, 12.));
        }

        let mut power = false;
        let mut bank = false;
        for _ in 0..60 * 90 {
            g.tick(DT, &[], &ids, |_, _| false);
            power |= room_at(me(&g, 1).at) == Some("power");
            bank |= matches!(room_at(me(&g, 2).at), Some("bank" | "vault"));
            if power && bank {
                break;
            }
        }
        assert!(power, "{bot_count} bots: power bot ended at {:?}", me(&g, 1).at);
        assert!(bank, "{bot_count} bots: bank bot ended at {:?}", me(&g, 2).at);
    }
}

#[test]
fn crew_bots_cross_doors_from_headless_review_positions() {
    let mut g = game([false, true, true]);
    g.actors[0].side = Side::Crew;
    g.sandbox = true;
    g.tuning.second_post = true;
    g.qa_power_job();
    put(&mut g, 0, (-11.5, 20.5));
    put(&mut g, 1, (-9., 25.5));
    put(&mut g, 2, (1., 37.));

    let mut power = false;
    let mut bank = false;
    for _ in 0..60 {
        g.tick(0.05, &[], &[0, 1, 2], |_, _| false);
        power |= room_at(me(&g, 1).at) == Some("power");
        bank |= matches!(room_at(me(&g, 2).at), Some("bank" | "vault"));
        if power && bank {
            break;
        }
    }
    assert!(power, "power bot ended at {:?}", me(&g, 1).at);
    assert!(bank, "bank bot ended at {:?}", me(&g, 2).at);
}

#[test]
fn you_walk_into_the_bank_through_its_door_and_not_its_walls() {
    let mut g = game([false; 3]);
    put(&mut g, 1, (-1.5, 40.));
    walk(&mut g, 1, (-1.5, 30.), 3.);
    assert_eq!(room_at(me(&g, 1).at), None, "the front wall stops you");
    assert!(me(&g, 1).at.1 > 35.);
    put(&mut g, 1, (1., 40.));
    walk(&mut g, 1, (1., 30.), 3.);
    assert_eq!(room_at(me(&g, 1).at), Some("bank"), "the door lets you in");
    walk(&mut g, 1, (2., 27.), 1.);
    walk(&mut g, 1, (9., 27.), 2.);
    assert_eq!(
        room_at(me(&g, 1).at),
        Some("bank"),
        "the vault wall stops you"
    );
    walk(&mut g, 1, (4., 30.), 1.5);
    walk(&mut g, 1, (8., 30.), 2.);
    assert_eq!(
        room_at(me(&g, 1).at),
        Some("vault"),
        "through the vault door"
    );
    let view = g.view_for(Some(1));
    assert_eq!(
        view.actors.iter().find(|a| a.seat == 1).unwrap().room,
        Some("vault")
    );
}

#[test]
fn the_vault_only_works_from_inside_the_vault_room() {
    let mut g = game([false; 3]);
    g.job = Job::Vault;
    g.power = 30.;
    put(&mut g, 1, (BANK.0 - 4., BANK.1 + 1.5));
    assert!(distance(me(&g, 1).at, BANK) < 4.5);
    g.control(1, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    g.control(1, false, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(g.qa_check_kind(1), None, "no cracking through the wall");
    put(&mut g, 1, (BANK.0 - 1., BANK.1));
    g.control(1, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(g.qa_check_kind(1), Some(CheckKind::Dial));
}

#[test]
fn cars_stop_outside_buildings() {
    let mut g = game([false; 3]);
    g.place(0, (1., 41.), true, std::f32::consts::PI);
    for _ in 0..180 {
        g.tick(
            DT,
            &[(0, Stick { x: 0., z: -1. }, 0.)],
            &[0, 1, 2],
            |_, _| false,
        );
    }
    let car = me(&g, 0).at;
    assert!(
        car.1 >= 35. + city_buildings::CAR - 0.01,
        "the car stays out: {car:?}"
    );
    assert_eq!(room_at(car), None);
}

#[test]
fn routes_on_foot_go_through_doors() {
    let path = route(CREW_SPAWN, BANK, false);
    assert!(!path.is_empty());
    let mut rooms = vec![room_at(CREW_SPAWN)];
    let mut here = CREW_SPAWN;
    for &next in &path {
        assert!(
            city_buildings::clear_walk(here, next) || distance(here, next) < 1.5,
            "{here:?} to {next:?}"
        );
        here = next;
        let room = room_at(next);
        if rooms.last() != Some(&room) {
            rooms.push(room);
        }
    }
    assert_eq!(rooms, vec![None, Some("bank"), Some("vault")]);
    for (from, to, room) in [
        (JAIL_EXIT, JAIL, "jail"),
        ((-28., -20.), ESCAPE, "garage"),
        ((-20., 30.), POWER, "power"),
    ] {
        let path = route(from, to, false);
        let mut here = from;
        for &next in &path {
            assert!(city_buildings::clear_walk(here, next) || distance(here, next) < 1.5);
            here = next;
        }
        assert_eq!(room_at(here), Some(room));
    }
}

#[test]
fn a_bot_carrier_runs_out_of_the_bank_and_into_the_garage() {
    let mut g = game([false, true, false]);
    put(&mut g, 0, (60., -60.));
    put(&mut g, 2, (60., 60.));
    put(&mut g, 1, BANK);
    g.job = Job::Carry;
    g.bags = vec![bags::Bag {
        at: BANK,
        carrier: Some(1),
    }];
    let mut rooms = vec![room_at(BANK)];
    for _ in 0..60 * 40 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        let at = me(&g, 1).at;
        assert!(walkable(at), "the bot is never in a wall: {at:?}");
        let room = room_at(at);
        if rooms.last() != Some(&room) {
            rooms.push(room);
        }
        if g.night_score[1] > 0 {
            break;
        }
    }
    assert_eq!(
        rooms,
        vec![Some("vault"), Some("bank"), None, Some("garage")]
    );
    assert!(g.night_score[1] > 0, "the bag is delivered at the van");
}

#[test]
fn an_officer_bot_follows_a_robber_inside_and_grabs_them() {
    let mut g = game([true, false, false]);
    put(&mut g, 0, (1., 41.));
    put(&mut g, 1, (1., 31.));
    put(&mut g, 2, (60., 60.));
    for _ in 0..60 * 10 {
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
        assert!(walkable(me(&g, 0).at));
        if !g.arrests.is_empty() {
            break;
        }
    }
    assert_eq!(g.arrests.len(), 1, "the officer reached them");
    assert_eq!(room_at(me(&g, 0).at), Some("bank"));
}

#[test]
fn an_officer_drags_a_cuffed_robber_out_through_the_door() {
    let mut g = game([false; 3]);
    put(&mut g, 0, (1., 31.));
    put(&mut g, 1, (1.8, 31.));
    g.actors[1].stun = 2.;
    for _ in 0..30 {
        g.control(0, true, false);
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.arrests.len(), 1);
    for _ in 0..(2.7 * 60.) as usize {
        g.control(0, true, false);
        if let Some(t) = g.arrests.first().map(|x| x.t) {
            let p = arrest::figure_eight(t + std::f32::consts::TAU / 2.5 / 60.);
            g.trace(0, p.0, p.1);
        }
        g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    }
    assert_eq!(g.arrests[0].stage, Stage::Escort);
    // Letting go of the use button: holding it tightens the cuffs in place.
    g.control(0, false, false);
    for to in [(1., 33.), (1., 38.), (1., 41.)] {
        walk(&mut g, 0, to, 1.5);
        assert!(
            walkable(me(&g, 1).at),
            "the robber is never pulled into a wall"
        );
    }
    assert_eq!(g.arrests.len(), 1, "still in custody");
    assert_eq!(room_at(me(&g, 0).at), None);
    assert_eq!(room_at(me(&g, 1).at), None, "dragged out the door");
    assert!(distance(me(&g, 0).at, me(&g, 1).at) < 2.5);
}

#[test]
fn prisoners_sit_in_the_cell_and_are_rescued_from_the_front_room() {
    let mut g = game([false; 3]);
    put(&mut g, 1, (60., 60.));
    g.actors[1].jailed = true;
    g.actors[1].jail_left = 100.;
    g.actors[1].at = JAIL_CELL;
    assert_eq!(room_at(JAIL_CELL), Some("cell"));
    put(&mut g, 2, (28., -22.));
    walk(&mut g, 2, JAIL, 3.);
    assert_eq!(room_at(me(&g, 2).at), Some("jail"));
    g.control(2, true, false);
    g.tick(DT, &[], &[0, 1, 2], |_, _| false);
    assert_eq!(
        g.qa_check_kind(2),
        Some(CheckKind::Lockpick),
        "the cell lock, from the front room"
    );
    g.qa_pass_check(2);
    assert!(!g.actors[1].jailed);
    assert_eq!(room_at(g.actors[1].at), None, "freed out the front door");
}
