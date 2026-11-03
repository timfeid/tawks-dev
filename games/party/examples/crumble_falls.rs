//! Diagnose why Crumble bots fall in production-form four-player bouts.
//! cargo run --release -p party --example crumble_falls -- [games]
use std::collections::VecDeque;

use party::{
    bout::{Bout, Contestant, TICK_DT, on_form},
    crumble::{CrumbleRun, TILE, Tile, TileState},
};

#[derive(Default)]
struct Falls {
    total: u32,
    expired_underfoot: u32,
    over_hole: u32,
    moving: u32,
    waiting_with_exit: u32,
    moving_with_exit: u32,
    no_near_exit: u32,
    no_clear_exit: u32,
    indirect_route: u32,
    component_sum: usize,
    component_one_second_sum: usize,
    component_one_second_count: u32,
    component_three_seconds_sum: usize,
    component_three_seconds_count: u32,
    radius_sum: f32,
    solid_sum: usize,
}

fn clear_exits(tiles: &[Tile], from: (f32, f32)) -> (usize, usize) {
    let current = tiles
        .iter()
        .position(|t| (from.0 - t.x).abs() <= TILE / 2.0 && (from.1 - t.z).abs() <= TILE / 2.0);
    let mut near = 0;
    let mut any = 0;
    for (index, tile) in tiles.iter().enumerate() {
        if Some(index) == current || tile.state != TileState::Solid {
            continue;
        }
        let distance = (tile.x - from.0).hypot(tile.z - from.1);
        if !(1..=12).all(|step| {
            let fraction = step as f32 / 12.0;
            let x = from.0 + (tile.x - from.0) * fraction;
            let z = from.1 + (tile.z - from.1) * fraction;
            tiles.iter().any(|tile| {
                (x - tile.x).abs() <= TILE / 2.0
                    && (z - tile.z).abs() <= TILE / 2.0
                    && matches!(tile.state, TileState::Solid | TileState::Cracking)
            })
        }) {
            continue;
        }
        any += 1;
        if distance <= TILE * 1.8 {
            near += 1;
        }
    }
    (near, any)
}

fn connected_solids(tiles: &[Tile], from: (f32, f32)) -> usize {
    let Some(start) = tiles
        .iter()
        .position(|t| (from.0 - t.x).abs() <= TILE / 2.0 && (from.1 - t.z).abs() <= TILE / 2.0)
    else {
        return 0;
    };
    if !tiles[start].standable() {
        return 0;
    }
    let mut visited = vec![false; tiles.len()];
    visited[start] = true;
    let mut queue = VecDeque::from([start]);
    let mut solids = 0;
    while let Some(index) = queue.pop_front() {
        let tile = &tiles[index];
        if index != start && tile.state == TileState::Solid {
            solids += 1;
        }
        for (next, other) in tiles.iter().enumerate() {
            if !visited[next]
                && other.standable()
                && (other.col - tile.col).abs() + (other.row - tile.row).abs() == 1
            {
                visited[next] = true;
                queue.push_back(next);
            }
        }
    }
    solids
}

fn run(games: u64, top: f32, field: f32, base: u64) -> [Falls; 2] {
    let mut result = [Falls::default(), Falls::default()];
    for seed in 0..games {
        let star = (seed % 4) as u8;
        let seats: Vec<_> = (0..4)
            .map(|seat| Contestant {
                seat,
                is_bot: true,
                skill: if seat == star { top } else { field },
                team: seat % 2,
                ally: None,
            })
            .collect();
        let seed = base + seed;
        let mut game = CrumbleRun::new(&on_form(&seats, seed), seed);
        let mut events = Vec::new();
        let mut history: Vec<VecDeque<(f32, usize)>> = vec![VecDeque::new(); 4];
        for tick in 0..(75 * 60) {
            if tick % 60 == 0 {
                for player in game.players.iter().filter(|player| player.up()) {
                    history[player.seat as usize].push_back((
                        game.time,
                        connected_solids(&game.tiles, (player.run.x, player.run.z)),
                    ));
                }
            }
            let inputs: Vec<_> = seats
                .iter()
                .map(|seat| (seat.seat, game.bot_input(seat.seat, TICK_DT)))
                .collect();
            let tiles_before = game.tiles.clone();
            let before: Vec<_> = game
                .players
                .iter()
                .map(|player| {
                    let tile = player.on.and_then(|index| game.tiles.get(index));
                    let stick = inputs
                        .iter()
                        .find(|(seat, _)| *seat == player.seat)
                        .unwrap()
                        .1
                        .stick;
                    (
                        player.up(),
                        tile.map(|t| (t.state, t.timer)),
                        (player.run.x, player.run.z),
                        stick.x.hypot(stick.z) > 0.1,
                        player.run.x.hypot(player.run.z),
                    )
                })
                .collect();
            game.step(&inputs, TICK_DT, &mut events);
            for event in &events {
                if event["kind"] != "drop" {
                    continue;
                }
                let seat = event["seat"].as_u64().unwrap() as usize;
                let (was_up, tile, position, moving, radius) = before[seat];
                if !was_up {
                    continue;
                }
                let (near, any) = clear_exits(&tiles_before, position);
                let connected = connected_solids(&tiles_before, position);
                let row = &mut result[usize::from(seat as u8 != star)];
                row.total += 1;
                row.radius_sum += radius;
                row.solid_sum += tiles_before
                    .iter()
                    .filter(|tile| tile.state == TileState::Solid)
                    .count();
                if tile
                    .is_some_and(|(state, timer)| state == TileState::Cracking && timer <= TICK_DT)
                {
                    row.expired_underfoot += 1;
                } else {
                    row.over_hole += 1;
                }
                if moving {
                    row.moving += 1;
                    if near > 0 {
                        row.moving_with_exit += 1;
                    }
                } else if near > 0 {
                    row.waiting_with_exit += 1;
                }
                if near == 0 {
                    row.no_near_exit += 1;
                }
                if any == 0 {
                    row.no_clear_exit += 1;
                    if connected > 0 {
                        row.indirect_route += 1;
                    }
                }
                row.component_sum += connected;
                for (seconds, sum, count) in [
                    (
                        1.0,
                        &mut row.component_one_second_sum,
                        &mut row.component_one_second_count,
                    ),
                    (
                        3.0,
                        &mut row.component_three_seconds_sum,
                        &mut row.component_three_seconds_count,
                    ),
                ] {
                    if let Some((_, earlier)) = history[seat]
                        .iter()
                        .rev()
                        .find(|(at, _)| *at <= game.time - seconds)
                    {
                        *sum += earlier;
                        *count += 1;
                    }
                }
            }
            events.clear();
            if game.placements().is_some() {
                break;
            }
        }
    }
    result
}

fn main() {
    let games = std::env::args()
        .nth(1)
        .and_then(|n| n.parse().ok())
        .unwrap_or(200);
    let offset = std::env::var("FORM_BALANCE_SEED")
        .ok()
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0);
    for (label, top, field) in [("top", 1.0, 0.7), ("mid", 0.7, 0.4)] {
        let [favorite, field] = run(games, top, field, 10_000 + offset);
        for (side, row) in [("favorite", favorite), ("field", field)] {
            println!(
                "{label} {side}: drops={} expired={} over_hole={} moving={} waiting_with_exit={} moving_with_exit={} no_near_exit={} no_clear_exit={} indirect_route={} mean_connected_solids={:.0} mean_component_1s={:.0} mean_component_3s={:.0} mean_radius={:.1} mean_solid_remaining={:.0}",
                row.total,
                row.expired_underfoot,
                row.over_hole,
                row.moving,
                row.waiting_with_exit,
                row.moving_with_exit,
                row.no_near_exit,
                row.no_clear_exit,
                row.indirect_route,
                row.component_sum as f32 / row.total.max(1) as f32,
                row.component_one_second_sum as f32 / row.component_one_second_count.max(1) as f32,
                row.component_three_seconds_sum as f32
                    / row.component_three_seconds_count.max(1) as f32,
                row.radius_sum / row.total.max(1) as f32,
                row.solid_sum as f32 / row.total.max(1) as f32,
            );
        }
    }
}
