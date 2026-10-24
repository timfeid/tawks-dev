//! A practice opponent: chase the cheapest open path for its tickets, take
//! the route it can afford, otherwise draw the cards that path needs.

use std::collections::{BinaryHeap, HashMap};

use crate::game::{Action, Card, Game, Stage, TurnPhase};
use crate::map::{self, Route, TrackColor, ROUTES};

pub fn next_action(game: &Game, player_id: &str) -> Option<Action> {
    if game.stage != Stage::Playing {
        return None;
    }
    let player = game.player(player_id)?;
    if !player.ticket_offer.is_empty() {
        // Keep the two cheapest opening tickets, or the single cheapest later.
        let mut offer = player.ticket_offer.clone();
        offer.sort_by_key(|t| t.points);
        let n = if player.first_ticket_pick_done { 1 } else { 2.min(offer.len()) };
        return Some(Action::KeepTickets { keep: offer.iter().take(n).map(|t| t.id).collect() });
    }
    if !game.is_active(player_id) {
        return None;
    }
    match game.phase {
        TurnPhase::ChoosingTickets => None,
        TurnPhase::DrewOne => Some(draw_for(game, player_id, &wanted_routes(game, player_id), true)),
        TurnPhase::Start => {
            let wanted = wanted_routes(game, player_id);
            for route in &wanted {
                if game.route_is_claimable(player_id, route) {
                    let (color, _) = game.claim_options(player_id, route)[0];
                    return Some(Action::ClaimRoute { route: route.id, color: Some(color) });
                }
            }
            if wanted.is_empty() && player.trains > 12 && !game.ticket_deck.is_empty() {
                return Some(Action::DrawTickets);
            }
            if wanted.is_empty() {
                // Nothing left to chase: grab any affordable long route.
                let mut open: Vec<&Route> = ROUTES.iter().filter(|r| game.route_is_claimable(player_id, r)).collect();
                open.sort_by_key(|r| std::cmp::Reverse(r.length));
                if let Some(r) = open.first() {
                    let (color, _) = game.claim_options(player_id, r)[0];
                    return Some(Action::ClaimRoute { route: r.id, color: Some(color) });
                }
            }
            Some(draw_for(game, player_id, &wanted, false))
        }
    }
}

/// Routes on the cheapest still-open paths for each unfinished ticket.
fn wanted_routes(game: &Game, player_id: &str) -> Vec<&'static Route> {
    let Some(player) = game.player(player_id) else { return Vec::new() };
    let connected = game.connected_cities(player_id);
    let mut out: Vec<&'static Route> = Vec::new();
    for t in &player.tickets {
        if connected.get(t.from.as_str()).map(|s| s.contains(t.to.as_str())).unwrap_or(false) {
            continue;
        }
        for r in cheapest_path(game, player_id, &t.from, &t.to) {
            if !out.iter().any(|o| o.id == r.id) {
                out.push(r);
            }
        }
    }
    out
}

/// Dijkstra over routes the player could still use: own routes are free,
/// open routes cost their length, other players' routes are walls.
fn cheapest_path(game: &Game, player_id: &str, from: &str, to: &str) -> Vec<&'static Route> {
    let mut dist: HashMap<&str, u32> = HashMap::new();
    let mut prev: HashMap<&str, &'static Route> = HashMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(from, 0);
    heap.push(std::cmp::Reverse((0u32, from)));
    while let Some(std::cmp::Reverse((d, at))) = heap.pop() {
        if at == to {
            break;
        }
        if dist.get(at).copied().unwrap_or(u32::MAX) < d {
            continue;
        }
        for r in ROUTES {
            let next = if r.from == at { r.to } else if r.to == at { r.from } else { continue };
            let cost = match game.owners.get(&r.id) {
                Some(o) if o == player_id => 0,
                Some(_) => continue,
                None => {
                    if r.twin.map(|t| game.owners.get(&t).map(|o| o == player_id || game.players.len() < 4).unwrap_or(false)).unwrap_or(false) {
                        continue;
                    }
                    r.length as u32
                }
            };
            let nd = d + cost;
            if nd < dist.get(next).copied().unwrap_or(u32::MAX) {
                dist.insert(next, nd);
                prev.insert(next, r);
                heap.push(std::cmp::Reverse((nd, next)));
            }
        }
    }
    let mut path = Vec::new();
    let mut at = to;
    while at != from {
        let Some(r) = prev.get(at) else { return Vec::new() };
        if game.owners.get(&r.id).is_none() {
            path.push(*r);
        }
        at = if r.from == at { r.to } else { r.from };
    }
    path.reverse();
    path
}

fn draw_for(game: &Game, player_id: &str, wanted: &[&Route], second: bool) -> Action {
    let player = game.player(player_id).expect("player");
    // Prefer a face-up card in a color a wanted route needs.
    let needed: Vec<TrackColor> = wanted.iter().map(|r| r.color).collect();
    let mut best: Option<(usize, i32)> = None;
    for (slot, card) in game.face_up.iter().enumerate() {
        if *card == Card::Locomotive {
            if second {
                continue;
            }
            let score = 5;
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((slot, score));
            }
            continue;
        }
        let useful = needed.iter().any(|c| card.matches(*c) || *c == TrackColor::Any);
        let held = player.hand_count(*card) as i32;
        let score = if useful { 10 + held } else { held };
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((slot, score));
        }
    }
    match best {
        Some((slot, score)) if score >= 10 || (score >= 5 && !second) => Action::DrawCard { face_up: Some(slot) },
        _ if !game.deck.is_empty() || !game.discard.is_empty() => Action::DrawCard { face_up: None },
        _ => best
            .map(|(slot, _)| Action::DrawCard { face_up: Some(slot) })
            .unwrap_or(Action::DrawCard { face_up: None }),
    }
}

pub fn card_for(route: &Route) -> Option<Card> {
    match route.color {
        TrackColor::Any => None,
        c => Card::ALL.iter().copied().find(|card| card.matches(c) && *card != Card::Locomotive),
    }
}

pub fn city_name(id: &str) -> &'static str {
    map::city(id).map(|c| c.name).unwrap_or("?")
}
