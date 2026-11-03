//! Whole Tawkathons played by eight bots, lobby to podium, on every board.
//! Each party is driven only by ticking the room, the way the server does,
//! and checked as it goes: it has to finish, keep every purse, heart, bag and
//! token in bounds, and end with standings. The counts gathered along the way
//! show which parts of each board the bots actually get to.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;

use crate::board::{BotMove, Item, MAX_HEALTH, MAX_ITEMS, Map, TurnStep};
use crate::room::{MAX_SEATS, Phase, Room, TICK_HZ};

/// Generous: a five-round party of eight bots takes about half of this.
const PARTY_TICK_BUDGET: u64 = TICK_HZ as u64 * 60 * 120;
/// No one board turn, vacuum and all, should take anywhere near this long.
const TURN_TICK_BUDGET: u64 = TICK_HZ as u64 * 90;
/// Nor one bout.
const BOUT_TICK_BUDGET: u64 = TICK_HZ as u64 * 60 * 6;
/// A bot whose move is refused this many times in one turn is stuck.
const REFUSALS_PER_TURN: u32 = 12;

#[derive(Clone, Copy, Debug)]
struct Setup {
    map: Map,
    seed: u64,
    pairs: bool,
    mature: bool,
    /// Rounds in place of the room's, to keep the quick run quick.
    rounds: Option<u32>,
}

#[derive(Default)]
struct MapStats {
    games: u32,
    pairs_games: u32,
    mature_games: u32,
    ticks: u64,
    turns: u32,
    rounds: u32,
    bouts: BTreeMap<&'static str, u32>,
    item_uses: BTreeMap<String, u32>,
    /// Who used each item: distinct seats, and uses by that party's winner.
    item_users: BTreeMap<String, BTreeSet<(u64, u8)>>,
    item_by_winner: BTreeMap<String, u32>,
    /// Hit, miss, blocked and so on, from the board's own reports.
    item_outcomes: BTreeMap<String, u32>,
    vacuum_uses: u32,
    crate_throws: u32,
    snowball_throws: u32,
    prizes: u32,
    moves: BTreeMap<String, u32>,
    refused: BTreeMap<String, u32>,
    steps: BTreeMap<&'static str, u32>,
    events: BTreeMap<String, u32>,
    vacuumed: BTreeMap<String, u32>,
    space_effects: BTreeMap<String, u32>,
    round_events: BTreeMap<String, u32>,
    winning_prizes: Vec<u32>,
    /// Where each turn finished walking, by space.
    landed: BTreeMap<u8, u32>,
    /// The board's size, the lap's length, and the furthest round it any
    /// token got, in steps from the start.
    spaces: usize,
    lap: u8,
    furthest: u8,
}

fn step_name(step: &TurnStep) -> &'static str {
    match step {
        TurnStep::Items => "items",
        TurnStep::Roll => "roll",
        TurnStep::Branch { .. } => "branch",
        TurnStep::Shop { .. } => "shop",
        TurnStep::Cannon { .. } => "cannon",
        TurnStep::Lob { .. } => "lob",
        TurnStep::Swing { .. } => "swing",
        TurnStep::Check { .. } => "check",
        TurnStep::Vacuum { .. } => "vacuum",
        TurnStep::Done => "done",
        TurnStep::Raiding => "raiding",
        TurnStep::Position { .. } => "position",
        TurnStep::Walking => "walking",
        TurnStep::Driving => "driving",
    }
}

fn move_name(action: &BotMove) -> String {
    match action {
        BotMove::Item { item, .. } => format!("Item({item:?})"),
        BotMove::Shop { item: Some(_), .. } => "Shop(item)".into(),
        BotMove::Shop { upgrade: Some(_), .. } => "Shop(upgrade)".into(),
        BotMove::Shop { .. } => "Shop(walk on)".into(),
        BotMove::Vacuum { blow: true, .. } => "Vacuum(blow)".into(),
        BotMove::Vacuum { .. } => "Vacuum(suck)".into(),
        other => {
            let name = format!("{other:?}");
            name.split([' ', '{', '(']).next().unwrap_or_default().to_string()
        }
    }
}

fn item_key(item: &Value) -> String {
    item.as_str().map(str::to_string).unwrap_or_else(|| item.to_string())
}

/// Plays one party to the podium, checking it on every tick it changes, and
/// adds what happened to `stats`. Panics, naming the party, on anything wrong.
fn play_party(setup: Setup, stats: &mut MapStats) {
    let Setup { map, seed, pairs, mature, rounds } = setup;
    let name = format!("{map:?} seed {seed}{}{}", if pairs { " pairs" } else { "" }, if mature { " mature" } else { "" });
    let mut room = Room::new(format!("sim-{seed}"), seed);
    room.connect("host", "Host", "").unwrap();
    room.choose_map("host", map).unwrap();
    room.choose_pairs("host", pairs).unwrap();
    room.choose_mature("host", mature).unwrap();
    while room.members.len() < MAX_SEATS {
        room.add_bot("host").unwrap();
    }
    room.start_party("host").unwrap();
    // The host's seat plays itself too: eight bots, nobody at the controls.
    for member in room.members.iter_mut() {
        member.is_bot = true;
    }
    assert!(room.members.iter().all(|m| m.is_bot) && room.members.len() == MAX_SEATS);
    if let (Some(rounds), Some(board)) = (rounds, room.board.as_mut()) {
        board.rounds = rounds;
    }
    let layout = map.layout();
    let seats: Vec<u8> = room.members.iter().map(|m| m.seat).collect();

    stats.games += 1;
    stats.pairs_games += pairs as u32;
    stats.mature_games += mature as u32;

    let mut last_phase = std::mem::discriminant(&room.phase);
    let mut last_step: Option<&'static str> = None;
    let mut last_turn: Option<(u32, u8)> = None;
    let mut turn_since = 0u64;
    let mut turn_refusals = 0u32;
    let mut bout_since: Option<u64> = None;
    let mut bout_key = "";
    let mut logged = 0usize;
    let mut items_by_seat: Vec<(String, u8)> = Vec::new();
    let mut standings: Option<Vec<Vec<u8>>> = None;
    let mut ticks = 0u64;

    while standings.is_none() {
        ticks += 1;
        assert!(ticks <= PARTY_TICK_BUDGET, "{name}: no podium after {ticks} ticks, stuck in {:?}", room.phase);
        room.step();

        // What the bots did, and whether the board took it.
        for (seat, action, result) in &room.bot_log[logged..] {
            let key = move_name(action);
            *stats.moves.entry(key.clone()).or_default() += 1;
            if let Err(error) = result {
                *stats.refused.entry(format!("{key}: {error:?}")).or_default() += 1;
                turn_refusals += 1;
                assert!(
                    turn_refusals < REFUSALS_PER_TURN,
                    "{name}: seat {seat} is stuck, its {key} refused {turn_refusals} times this turn ({error:?})"
                );
                continue;
            }
            match action {
                BotMove::Item { item, .. } => {
                    let item = format!("{item:?}");
                    *stats.item_uses.entry(item.clone()).or_default() += 1;
                    stats.item_users.entry(item.clone()).or_default().insert((seed, *seat));
                    items_by_seat.push((item, *seat));
                }
                BotMove::Suck => stats.vacuum_uses += 1,
                BotMove::ThrowCrate => stats.crate_throws += 1,
                BotMove::ThrowSnowball => stats.snowball_throws += 1,
                _ => {}
            }
        }
        logged = room.bot_log.len();

        // What the board said happened.
        if let Some(board) = room.board.as_mut() {
            for event in board.take_events() {
                let kind = event["kind"].as_str().unwrap_or("?").to_string();
                match kind.as_str() {
                    // Off-turn Slash Rush starts from the raid controller,
                    // not BotMove::Item, so it needs its own usage receipt.
                    "raid_start" if event["item"]=="slash_rush" => {
                        if let Some(seat)=event["seat"].as_u64().map(|s|s as u8){
                            let item="SlashRush".to_string();
                            *stats.item_uses.entry(item.clone()).or_default()+=1;
                            stats.item_users.entry(item.clone()).or_default().insert((seed,seat));
                            items_by_seat.push((item,seat));
                        }
                    }
                    "item" => {
                        let quality = event["quality"].as_str().unwrap_or("");
                        if !quality.is_empty() {
                            *stats.item_outcomes.entry(format!("{} {quality}", item_key(&event["item"]))).or_default() += 1;
                        }
                    }
                    "vacuumed" => *stats.vacuumed.entry(item_key(&event["what"])).or_default() += 1,
                    "space" => *stats.space_effects.entry(item_key(&event["effect"])).or_default() += 1,
                    "event" => *stats.round_events.entry(item_key(&event["event"])).or_default() += 1,
                    _ => {}
                }
                *stats.events.entry(kind).or_default() += 1;
            }
        }

        // Where the bouts are up to.
        let phase = std::mem::discriminant(&room.phase);
        if phase != last_phase {
            match &room.phase {
                Phase::Playing => {
                    bout_since = Some(ticks);
                    bout_key = room.minigame;
                    *stats.bouts.entry(room.minigame).or_default() += 1;
                }
                // The map's finale race is judged like a bout.
                Phase::MapFinale => bout_since = Some(ticks),
                Phase::Results { placements, .. } => {
                    assert!(bout_since.take().is_some(), "{name}: results without a bout");
                    let placed: Vec<u8> = placements.iter().flatten().copied().collect();
                    let unique: BTreeSet<u8> = placed.iter().copied().collect();
                    assert_eq!(placed.len(), MAX_SEATS, "{name}: {bout_key} placed {placements:?}");
                    assert_eq!(unique.len(), MAX_SEATS, "{name}: {bout_key} placed someone twice: {placements:?}");
                    assert!(placements.iter().all(|tier| !tier.is_empty()), "{name}: {bout_key} has an empty place");
                }
                _ => assert!(bout_since.is_none(), "{name}: {bout_key} ended without results, now {:?}", room.phase),
            }
            last_phase = phase;
        }
        if let Some(since) = bout_since {
            assert!(ticks - since < BOUT_TICK_BUDGET, "{name}: {bout_key} never finished");
        }

        if let Phase::GameOver { standings: podium, .. } = &room.phase {
            standings = Some(podium.clone());
        }

        // The board, checked whenever anything on it moved.
        if !room.take_dirty() && standings.is_none() {
            continue;
        }
        let Some(board) = room.board.as_ref() else {
            panic!("{name}: the board went away mid-party in {:?}", room.phase);
        };
        let view = board.view();
        for token in &view.tokens {
            let who = format!("{name}: seat {} in round {}", token.seat, view.round);
            assert!(token.coins >= 0, "{who} is {} coins in debt", token.coins);
            assert!(token.health <= MAX_HEALTH, "{who} has {} hearts", token.health);
            assert!((token.space as usize) < layout.len(), "{who} is on space {}, off the board", token.space);
            assert!(
                !layout[token.space as usize].gated || board.gates_open(),
                "{who} is on gated space {} before the gates open",
                token.space
            );
            assert!(token.items.len() <= MAX_ITEMS, "{who} holds {:?}", token.items);
            assert!(
                !token.items.iter().any(|i| matches!(i, Item::Crate | Item::Snowball)),
                "{who} has a crate or snowball in the bag"
            );
        }
        if room.phase == Phase::Board {
            let step = step_name(&view.turn);
            let turn = (view.round, view.turn_seat);
            if last_turn != Some(turn) {
                stats.turns += 1;
                last_turn = Some(turn);
                turn_since = ticks;
                turn_refusals = 0;
                last_step = None;
            }
            if last_step != Some(step) {
                if step == "done" {
                    if let Some(token) = view.tokens.iter().find(|t| t.seat == view.turn_seat) {
                        *stats.landed.entry(token.space).or_default() += 1;
                    }
                }
                *stats.steps.entry(step).or_default() += 1;
                last_step = Some(step);
            }
            assert!(ticks - turn_since < TURN_TICK_BUDGET, "{name}: seat {} stuck in {step}", view.turn_seat);
        } else {
            last_step = None;
            last_turn = None;
        }
    }

    // The podium.
    let standings = standings.unwrap();
    let board = room.board.as_ref().expect("the board stays up for the podium");
    let placed: Vec<u8> = standings.iter().flatten().copied().collect();
    let unique: BTreeSet<u8> = placed.iter().copied().collect();
    assert_eq!(placed.len(), seats.len(), "{name}: podium {standings:?}");
    assert_eq!(unique, seats.iter().copied().collect(), "{name}: podium {standings:?}");
    assert!(standings.first().is_some_and(|tier| !tier.is_empty()), "{name}: nobody won");
    let tokens = board.view().tokens;
    let score = |seat: u8| tokens.iter().find(|t| t.seat == seat).map(|t| (t.prizes, t.coins)).unwrap();
    let ranked = board.standings();
    assert!(ranked.windows(2).all(|w| w[0].1 >= w[1].1), "{name}: standings out of order {ranked:?}");
    if pairs {
        for tier in &standings {
            for seat in tier {
                let partner = board.partner_of(*seat).unwrap_or_else(|| panic!("{name}: seat {seat} has no partner"));
                assert!(tier.contains(&partner), "{name}: {seat} and partner {partner} finished apart: {standings:?}");
            }
        }
        // Partners finish at the better one's place: the top pair holds the best score.
        let best = seats.iter().map(|s| score(*s)).max().unwrap();
        assert!(standings[0].iter().any(|s| score(*s) == best), "{name}: the best score is not on top");
    } else {
        let best = seats.iter().map(|s| score(*s)).max().unwrap();
        assert!(standings[0].iter().all(|s| score(*s) == best), "{name}: the winner is not the best: {standings:?}");
    }
    assert!(stats.bouts.values().sum::<u32>() > 0, "{name}: no bouts at all");
    assert!(bout_since.is_none(), "{name}: a bout was left running");

    stats.spaces = layout.len();
    stats.lap = layout.iter().filter(|s| s.path == 0).count() as u8;
    for space in stats.landed.keys() {
        let away = crate::board::distance_in(layout, 0, *space);
        if away < u8::MAX {
            stats.furthest = stats.furthest.max(away);
        }
    }
    stats.ticks += ticks;
    stats.rounds += board.rounds;
    stats.prizes += tokens.iter().map(|t| t.prizes).sum::<u32>();
    stats.winning_prizes.push(standings[0].iter().map(|s| score(*s).0).max().unwrap());
    for (item, seat) in items_by_seat {
        if standings[0].contains(&seat) {
            *stats.item_by_winner.entry(item).or_default() += 1;
        }
    }
}

/// The parties, spread evenly over the boards, with pairs and mature games
/// mixed in and every seed different.
fn setups(games: usize, rounds: Option<u32>) -> Vec<Setup> {
    // PARTY_SIM_SEED shifts every seed, to go looking past the usual fifty.
    let shift: u64 = std::env::var("PARTY_SIM_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    (0..games)
        .map(|i| Setup {
            map: Map::ALL[i % Map::ALL.len()],
            seed: 7_919 * (i as u64 + 1) + 13 + shift,
            pairs: (i / Map::ALL.len()) % 3 == 1,
            mature: (i / Map::ALL.len()) % 2 == 1,
            rounds,
        })
        .collect()
}

/// The specials each board is built around, as the vacuum reports them.
fn specials(map: Map) -> &'static [&'static str] {
    match map {
        Map::Island => &["coconut"],
        Map::Volcano => &["lava"],
        Map::Shipwreck => &["flotsam"],
        Map::Frozen => &["snowball"],
    }
}

fn report(all: &BTreeMap<String, MapStats>) -> (String, Vec<String>) {
    let mut out = String::new();
    let mut never = Vec::new();
    let line = |out: &mut String, label: &str, f: &dyn Fn(&MapStats) -> String| {
        let _ = write!(out, "{label:<22}");
        for stats in all.values() {
            let _ = write!(out, "{:>14}", f(stats));
        }
        out.push('\n');
    };
    let _ = write!(out, "\n{:<22}", "");
    for map in all.keys() {
        let _ = write!(out, "{map:>14}");
    }
    out.push('\n');
    line(&mut out, "games (pairs/mature)", &|s| format!("{} ({}/{})", s.games, s.pairs_games, s.mature_games));
    line(&mut out, "rounds", &|s| s.rounds.to_string());
    line(&mut out, "minutes / game", &|s| format!("{:.1}", s.ticks as f32 / TICK_HZ as f32 / 60.0 / s.games as f32));
    line(&mut out, "turns", &|s| s.turns.to_string());
    line(&mut out, "spaces landed / all", &|s| format!("{}/{}", s.landed.len(), s.spaces));
    line(&mut out, "furthest from start", &|s| format!("{}/{}", s.furthest, s.lap));
    line(&mut out, "bouts played", &|s| s.bouts.values().sum::<u32>().to_string());
    line(&mut out, "prizes won", &|s| s.prizes.to_string());
    line(&mut out, "winner prizes (avg)", &|s| {
        format!("{:.1}", s.winning_prizes.iter().sum::<u32>() as f32 / s.games.max(1) as f32)
    });
    line(&mut out, "vacuum uses", &|s| s.vacuum_uses.to_string());
    line(&mut out, "crate throws", &|s| s.crate_throws.to_string());
    line(&mut out, "snowball throws", &|s| s.snowball_throws.to_string());

    let section = |out: &mut String, title: &str, keys: BTreeSet<String>, get: &dyn Fn(&MapStats, &str) -> String| {
        let _ = writeln!(out, "-- {title}");
        for key in keys {
            let _ = write!(out, "{:<22}", format!("  {key}"));
            for stats in all.values() {
                let _ = write!(out, "{:>14}", get(stats, &key));
            }
            out.push('\n');
        }
    };
    let keys = |f: &dyn Fn(&MapStats) -> Vec<String>| all.values().flat_map(f).collect::<BTreeSet<String>>();

    let map_of = |name: &str| Map::ALL.iter().copied().find(|m| format!("{m:?}") == name).unwrap();
    let kinds_of = |map: Map| {
        let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
        for space in map.layout() {
            *kinds.entry(format!("{:?}", space.kind)).or_default() += 1;
            if space.gated {
                *kinds.entry("(gated)".into()).or_default() += 1;
            }
        }
        kinds
    };
    let names: Vec<String> = all.keys().cloned().collect();
    let kinds: Vec<BTreeMap<String, u32>> = names.iter().map(|n| kinds_of(map_of(n))).collect();
    let _ = writeln!(out, "-- spaces on the board");
    for kind in kinds.iter().flat_map(|k| k.keys().cloned()).collect::<BTreeSet<String>>() {
        let _ = write!(out, "{:<22}", format!("  {kind}"));
        for k in &kinds {
            let _ = write!(out, "{:>14}", k.get(&kind).copied().unwrap_or(0));
        }
        out.push('\n');
    }

    let items: BTreeSet<String> = Item::ALL.iter().map(|i| format!("{i:?}")).collect();
    section(&mut out, "item uses (seats using it / uses by the winner)", items.clone(), &|s, k| {
        format!(
            "{} ({}/{})",
            s.item_uses.get(k).copied().unwrap_or(0),
            s.item_users.get(k).map_or(0, |u| u.len()),
            s.item_by_winner.get(k).copied().unwrap_or(0)
        )
    });
    let count = |m: &BTreeMap<String, u32>, k: &str| m.get(k).copied().unwrap_or(0).to_string();
    section(&mut out, "item outcomes", keys(&|s| s.item_outcomes.keys().cloned().collect()), &|s, k| count(&s.item_outcomes, k));
    section(&mut out, "vacuumed up", keys(&|s| s.vacuumed.keys().cloned().collect()), &|s, k| count(&s.vacuumed, k));
    section(&mut out, "space effects", keys(&|s| s.space_effects.keys().cloned().collect()), &|s, k| count(&s.space_effects, k));
    section(&mut out, "round events", keys(&|s| s.round_events.keys().cloned().collect()), &|s, k| count(&s.round_events, k));
    section(&mut out, "turn steps entered", keys(&|s| s.steps.keys().map(|k| k.to_string()).collect()), &|s, k| {
        s.steps.get(k).copied().unwrap_or(0).to_string()
    });
    section(&mut out, "bot moves", keys(&|s| s.moves.keys().cloned().collect()), &|s, k| count(&s.moves, k));
    section(&mut out, "bot moves refused", keys(&|s| s.refused.keys().cloned().collect()), &|s, k| count(&s.refused, k));
    section(&mut out, "board reports", keys(&|s| s.events.keys().cloned().collect()), &|s, k| count(&s.events, k));
    section(&mut out, "bouts", keys(&|s| s.bouts.keys().map(|k| k.to_string()).collect()), &|s, k| {
        s.bouts.get(k).copied().unwrap_or(0).to_string()
    });

    // Whatever never happened on a board.
    for (map, stats) in all {
        let map_enum = map_of(map);
        for item in &items {
            if item=="DriveCar"&&map_enum!=Map::Frozen{continue;}
            if !stats.item_uses.contains_key(item) {
                never.push(format!("{map}: no bot ever used {item}"));
            }
        }
        for special in specials(map_enum) {
            if !stats.vacuumed.contains_key(*special) {
                never.push(format!("{map}: the {special} special never triggered"));
            }
        }
        for (what, n) in [("vacuum", stats.vacuum_uses), ("crate throw", stats.crate_throws)] {
            if n == 0 {
                never.push(format!("{map}: no {what}"));
            }
        }
        if map_enum == Map::Frozen && stats.snowball_throws == 0 {
            never.push(format!("{map}: no snowball was ever thrown"));
        }
        for step in ["items", "roll", "branch", "shop", "cannon", "lob", "swing", "check", "vacuum", "done"] {
            if !stats.steps.contains_key(step) {
                never.push(format!("{map}: no turn ever reached the {step} step"));
            }
        }
        let layout = map_enum.layout();
        let reachable = |id: u8| crate::board::distance_in(layout, 0, id) < u8::MAX;
        let mut unvisited: BTreeMap<String, Vec<u8>> = BTreeMap::new();
        for space in layout {
            if !stats.landed.contains_key(&space.id) {
                unvisited.entry(format!("{:?}", space.kind)).or_default().push(space.id);
            }
        }
        for (kind, ids) in unvisited {
            let total = layout.iter().filter(|s| format!("{:?}", s.kind) == kind).count();
            let cut_off: Vec<u8> = ids.iter().copied().filter(|id| !reachable(*id)).collect();
            // A cannon, a slide or a teleport moves the token on, so a turn never ends there.
            if ids.len() == total && !matches!(kind.as_str(), "Start" | "Coin" | "Cannon" | "Ice" | "Teleport") {
                never.push(format!("{map}: no turn ever ended on any of its {total} {kind} spaces {ids:?}"));
            }
            if !cut_off.is_empty() {
                never.push(format!("{map}: {kind} spaces {cut_off:?} cannot be reached from the start"));
            }
        }
        if stats.prizes == 0 {
            never.push(format!("{map}: nobody ever bought a prize"));
        }
    }
    (out, never)
}

fn run(games: usize, rounds: Option<u32>) -> (BTreeMap<String, MapStats>, Vec<String>) {
    let started = std::time::Instant::now();
    let mut all: BTreeMap<String, MapStats> = BTreeMap::new();
    for (index,setup) in setups(games, rounds).into_iter().enumerate() {
        play_party(setup, all.entry(format!("{:?}", setup.map)).or_default());
        eprintln!("party soak {}/{} complete ({:?}, {:.1}s elapsed)",index+1,games,setup.map,started.elapsed().as_secs_f32());
    }
    let (table, never) = report(&all);
    println!("{games} parties of eight bots in {:.1}s{table}", started.elapsed().as_secs_f32());
    for line in &never {
        println!("never: {line}");
    }
    (all, never)
}

#[test]
fn eight_bots_finish_a_short_party_on_every_board() {
    // One quick party per board, one in pairs and one mature, on every run.
    let mut all: BTreeMap<String, MapStats> = BTreeMap::new();
    for (i, map) in Map::ALL.into_iter().enumerate() {
        let setup = Setup { map, seed: 101 + i as u64, pairs: i == 1, mature: i == 2, rounds: Some(2) };
        play_party(setup, all.entry(format!("{map:?}")).or_default());
    }
    for (map, stats) in &all {
        assert_eq!(stats.games, 1);
        // Knocked-out players spend a board turn resting, so count those
        // receipts alongside turns the bots could actively play.
        let resting = stats.events.get("resting").copied().unwrap_or(0);
        assert!(stats.turns >= MAX_SEATS as u32, "{map}: only {} active turns", stats.turns);
        assert!(stats.turns + resting >= 2 * MAX_SEATS as u32 - 2, "{map}: only {} turns plus {resting} resting", stats.turns);
        assert_eq!(stats.bouts.values().sum::<u32>(), 2, "{map}: a bout a round");
        assert!(stats.refused.is_empty(), "{map}: the board refused bot moves {:?}", stats.refused);
    }
}

#[test]
#[ignore = "fifty full parties; run with --ignored --nocapture"]
fn fifty_parties_of_eight_bots_play_out_on_every_board() {
    // PARTY_SIM_GAMES plays more (or fewer) than fifty.
    let games = std::env::var("PARTY_SIM_GAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(50);
    let (all, _) = run(games, None);
    assert_eq!(all.len(), Map::ALL.len(), "every board played");
    let games: Vec<u32> = all.values().map(|s| s.games).collect();
    assert!(games.iter().max().unwrap() - games.iter().min().unwrap() <= 1, "spread evenly: {games:?}");
    for (map, stats) in &all {
        assert!(stats.pairs_games > 0 && stats.mature_games > 0, "{map}: pairs and mature both played");
        assert_eq!(stats.bouts.values().sum::<u32>(), stats.rounds, "{map}: a bout a round");
        // A bot only tries what the board will take; anything refused is a
        // bot and board that disagree, and a refused move is tried again.
        assert!(stats.refused.is_empty(), "{map}: the board refused bot moves {:?}", stats.refused);
    }
}
