//! What each player sees: the whole board, everyone's public standing, and
//! their own hand, tickets and legal claims.

use serde::Serialize;

use crate::game::{Card, Stage, TurnPhase};
use crate::logic::TracklinesState;
use crate::map::{self, Ticket, TrackColor, CITIES, ROUTES};

#[derive(Clone, Debug, Serialize)]
pub struct CityView {
    pub id: &'static str,
    pub name: &'static str,
    pub longitude: f32,
    pub latitude: f32,
}

#[derive(Clone, Debug, Serialize)]
pub struct RouteView {
    pub id: u16,
    pub from: &'static str,
    pub to: &'static str,
    pub length: u8,
    pub color: TrackColor,
    pub twin: Option<u16>,
    pub owner: Option<String>,
    pub points: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct PlayerView {
    pub id: String,
    pub name: String,
    pub avatar_url: String,
    pub is_ai: bool,
    pub is_host: bool,
    pub is_active: bool,
    pub seat: usize,
    pub trains: u8,
    pub hand_size: usize,
    pub ticket_count: usize,
    pub score: i32,
    pub claimed: Vec<u16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HandCard {
    pub card: Card,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClaimOption {
    pub card: Card,
    pub locomotives: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClaimableRoute {
    pub route: u16,
    pub options: Vec<ClaimOption>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TicketView {
    pub id: u16,
    pub from: String,
    pub from_name: &'static str,
    pub to: String,
    pub to_name: &'static str,
    pub points: u16,
    pub completed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct ViewerView {
    pub id: String,
    pub seated: bool,
    pub is_host: bool,
    pub is_active: bool,
    pub hand: Vec<HandCard>,
    pub tickets: Vec<TicketView>,
    pub ticket_offer: Vec<TicketView>,
    pub must_keep: usize,
    pub claimable: Vec<ClaimableRoute>,
    pub can_draw_deck: bool,
    pub can_draw_tickets: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct StandingView {
    pub id: String,
    pub name: String,
    pub score: i32,
    pub tickets_done: usize,
    pub tickets_missed: usize,
    pub longest_path: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TracklinesView {
    pub stage: Stage,
    pub phase: TurnPhase,
    pub round: u32,
    pub host_id: Option<String>,
    pub active_player_id: Option<String>,
    pub final_round: bool,
    pub players: Vec<PlayerView>,
    pub viewer: Option<ViewerView>,
    pub face_up: Vec<Card>,
    pub deck_count: usize,
    pub ticket_deck_count: usize,
    pub cities: Vec<CityView>,
    pub routes: Vec<RouteView>,
    pub winner_id: Option<String>,
    pub standings: Vec<StandingView>,
    pub log: Vec<crate::game::LogEntry>,
}

fn ticket_view(state: &TracklinesState, player_id: &str, t: &Ticket) -> TicketView {
    let connected = state.game.connected_cities(player_id);
    TicketView {
        id: t.id,
        from: t.from.clone(),
        from_name: map::city(&t.from).map(|c| c.name).unwrap_or("?"),
        to: t.to.clone(),
        to_name: map::city(&t.to).map(|c| c.name).unwrap_or("?"),
        points: t.points,
        completed: connected.get(t.from.as_str()).map(|s| s.contains(t.to.as_str())).unwrap_or(false),
    }
}

pub fn build(state: &TracklinesState, player_id: &str) -> TracklinesView {
    let g = &state.game;
    let players = g
        .players
        .iter()
        .enumerate()
        .map(|(seat, p)| PlayerView {
            id: p.id.clone(),
            name: p.name.clone(),
            avatar_url: p.avatar_url.clone(),
            is_ai: p.is_ai,
            is_host: g.host_id.as_deref() == Some(&p.id),
            is_active: g.is_active(&p.id),
            seat,
            trains: p.trains,
            hand_size: p.hand.len(),
            ticket_count: p.tickets.len(),
            score: p.score,
            claimed: p.claimed.clone(),
        })
        .collect();
    let viewer = g.player(player_id).map(|p| {
        let mut hand: Vec<HandCard> = Card::ALL
            .iter()
            .map(|c| HandCard { card: *c, count: p.hand_count(*c) })
            .filter(|h| h.count > 0)
            .collect();
        hand.sort_by_key(|h| std::cmp::Reverse(h.count));
        let is_active = g.is_active(&p.id);
        let claimable = if is_active && g.phase == TurnPhase::Start {
            ROUTES
                .iter()
                .filter(|r| g.route_is_claimable(&p.id, r))
                .map(|r| ClaimableRoute {
                    route: r.id,
                    options: g
                        .claim_options(&p.id, r)
                        .into_iter()
                        .map(|(card, locomotives)| ClaimOption { card, locomotives })
                        .collect(),
                })
                .collect()
        } else {
            Vec::new()
        };
        ViewerView {
            id: p.id.clone(),
            seated: true,
            is_host: g.host_id.as_deref() == Some(&p.id),
            is_active,
            hand,
            tickets: p.tickets.iter().map(|t| ticket_view(state, &p.id, t)).collect(),
            ticket_offer: p.ticket_offer.iter().map(|t| ticket_view(state, &p.id, t)).collect(),
            must_keep: if p.first_ticket_pick_done { 1 } else { 2.min(p.ticket_offer.len()) },
            claimable,
            can_draw_deck: is_active && g.phase != TurnPhase::ChoosingTickets && (!g.deck.is_empty() || !g.discard.is_empty()),
            can_draw_tickets: is_active && g.phase == TurnPhase::Start && !g.ticket_deck.is_empty(),
        }
    });
    let standings = if g.stage == Stage::Finished {
        let mut rows: Vec<StandingView> = g
            .players
            .iter()
            .map(|p| {
                let connected = g.connected_cities(&p.id);
                let done = p
                    .tickets
                    .iter()
                    .filter(|t| connected.get(t.from.as_str()).map(|s| s.contains(t.to.as_str())).unwrap_or(false))
                    .count();
                StandingView {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    score: p.score,
                    tickets_done: done,
                    tickets_missed: p.tickets.len() - done,
                    longest_path: g.longest_path.as_ref().map(|(id, _)| id == &p.id).unwrap_or(false),
                }
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.score));
        rows
    } else {
        Vec::new()
    };
    TracklinesView {
        stage: g.stage,
        phase: g.phase,
        round: g.round,
        host_id: g.host_id.clone(),
        active_player_id: g.active_player().map(|p| p.id.clone()),
        final_round: g.final_round_by.is_some(),
        players,
        viewer: viewer.or_else(|| {
            Some(ViewerView {
                id: player_id.to_string(),
                seated: false,
                is_host: false,
                is_active: false,
                hand: Vec::new(),
                tickets: Vec::new(),
                ticket_offer: Vec::new(),
                must_keep: 0,
                claimable: Vec::new(),
                can_draw_deck: false,
                can_draw_tickets: false,
            })
        }),
        face_up: g.face_up.clone(),
        deck_count: g.deck.len() + g.discard.len(),
        ticket_deck_count: g.ticket_deck.len(),
        cities: CITIES.iter().map(|c| CityView { id: c.id, name: c.name, longitude: c.longitude, latitude: c.latitude }).collect(),
        routes: ROUTES
            .iter()
            .map(|r| RouteView {
                id: r.id,
                from: r.from,
                to: r.to,
                length: r.length,
                color: r.color,
                twin: r.twin,
                owner: g.owners.get(&r.id).cloned(),
                points: map::route_points(r.length),
            })
            .collect(),
        winner_id: g.winner_id.clone(),
        standings,
        log: g.log.iter().rev().take(12).cloned().collect(),
    }
}
