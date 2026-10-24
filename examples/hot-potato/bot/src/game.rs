use std::collections::HashMap;

use anyhow::{Result, bail};
use game_framework::{GameEffect, GameLogic};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const MAX_PLAYERS: usize = 6;
const FUSE_SECONDS: std::ops::RangeInclusive<u32> = 8..=20;
const SIZZLE_UNDER: u32 = 3;
const LOG_LINES: usize = 6;

#[derive(Deserialize)]
pub enum Action {
    View { name: String },
    Input { name: String, input: Input },
    Tick,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Input {
    Join,
    Start,
    Pass { to: usize },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Waiting,
    Playing,
    Over,
}

struct Player {
    id: String,
    name: String,
    alive: bool,
}

pub struct HotPotato {
    players: Vec<Player>,
    instances: HashMap<String, String>,
    names: HashMap<String, String>,
    phase: Phase,
    holder: usize,
    passed_from: Option<usize>,
    fuse_left: u32,
    held_for: u32,
    round: u32,
    log: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct PlayerView {
    name: String,
    alive: bool,
    holding: bool,
}

#[derive(Serialize, Clone)]
pub struct View {
    me: Option<usize>,
    phase: &'static str,
    players: Vec<PlayerView>,
    holder: Option<usize>,
    held_for: u32,
    can_pass_to: Vec<usize>,
    sizzling: bool,
    round: u32,
    winner: Option<usize>,
    can_join: bool,
    can_start: bool,
    max_players: usize,
    log: Vec<String>,
}

impl HotPotato {
    fn seat_of(&self, player_id: &str) -> Option<usize> {
        self.players.iter().position(|p| p.id == player_id)
    }

    fn alive(&self) -> Vec<usize> {
        (0..self.players.len()).filter(|&i| self.players[i].alive).collect()
    }

    fn note(&mut self, line: String) {
        self.log.push(line);
        let excess = self.log.len().saturating_sub(LOG_LINES);
        self.log.drain(..excess);
    }

    fn new_round(&mut self) {
        let alive = self.alive();
        let mut rng = rand::rng();
        self.holder = alive[rng.random_range(0..alive.len())];
        self.fuse_left = rng.random_range(FUSE_SECONDS);
        self.held_for = 0;
        self.passed_from = None;
        self.round += 1;
        let line = format!("Round {}: {} starts with the potato.", self.round, self.players[self.holder].name);
        self.note(line);
    }

    fn start(&mut self, player_id: &str) -> Result<()> {
        if self.seat_of(player_id).is_none() {
            bail!("Join the game first.");
        }
        if self.phase == Phase::Playing {
            bail!("The game is already running.");
        }
        if self.players.len() < 2 {
            bail!("You need at least two players.");
        }
        for p in &mut self.players {
            p.alive = true;
        }
        self.round = 0;
        self.log.clear();
        self.phase = Phase::Playing;
        self.new_round();
        Ok(())
    }

    fn passable_from(&self, seat: usize) -> Vec<usize> {
        let alive = self.alive();
        alive
            .iter()
            .copied()
            .filter(|&s| s != seat && (alive.len() == 2 || Some(s) != self.passed_from))
            .collect()
    }

    fn pass(&mut self, player_id: &str, to: usize) -> Result<()> {
        let Some(seat) = self.seat_of(player_id) else { bail!("You are watching this game.") };
        if self.phase != Phase::Playing {
            bail!("Nobody is holding the potato.");
        }
        if seat != self.holder {
            bail!("You are not holding the potato.");
        }
        if !self.passable_from(seat).contains(&to) {
            bail!("You can't pass to that player.");
        }
        self.passed_from = Some(seat);
        self.holder = to;
        self.held_for = 0;
        Ok(())
    }

    fn burn(&mut self) -> Vec<GameEffect> {
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        self.held_for += 1;
        self.fuse_left = self.fuse_left.saturating_sub(1);
        if self.fuse_left > 0 {
            return Vec::new();
        }
        let loser = self.holder;
        self.players[loser].alive = false;
        let mut said = format!("Boom! {} is out.", self.players[loser].name);
        self.note(said.clone());
        let alive = self.alive();
        if let [winner] = alive[..] {
            self.phase = Phase::Over;
            let line = format!("{} survives and wins!", self.players[winner].name);
            said = format!("{said} {line}");
            self.note(line);
        } else {
            self.new_round();
        }
        vec![GameEffect::Speak { text: said }]
    }
}

impl GameLogic for HotPotato {
    type Action = Action;
    type View = View;
    type Effect = GameEffect;
    type InitArgs = ();

    fn new(_: ()) -> Self {
        Self {
            players: Vec::new(),
            instances: HashMap::new(),
            names: HashMap::new(),
            phase: Phase::Waiting,
            holder: 0,
            passed_from: None,
            fuse_left: 0,
            held_for: 0,
            round: 0,
            log: Vec::new(),
        }
    }

    fn record_instance_id(&mut self, player_id: &str, instance_id: &str) {
        if player_id != "system" {
            self.instances.insert(player_id.to_string(), instance_id.to_string());
        }
    }

    fn get_instance_id_for_player_id(&self, player_id: &str) -> Option<String> {
        self.instances.get(player_id).cloned()
    }

    fn get_player_ids(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }

    fn should_fanout(&self, action: &Action) -> bool {
        !matches!(action, Action::View { .. })
    }

    fn update(&mut self, player_id: &str, action: Action) -> Result<Vec<GameEffect>> {
        match action {
            Action::View { name } => {
                self.names.insert(player_id.to_string(), name);
            }
            Action::Tick => return Ok(self.burn()),
            Action::Input { name, input } => {
                self.names.insert(player_id.to_string(), name.clone());
                match input {
                    Input::Join => {
                        if self.seat_of(player_id).is_some() {
                            return Ok(Vec::new());
                        }
                        if self.phase == Phase::Playing {
                            bail!("Wait for this game to finish.");
                        }
                        if self.players.len() >= MAX_PLAYERS {
                            bail!("The circle is full.");
                        }
                        self.note(format!("{name} joined."));
                        self.players.push(Player { id: player_id.to_string(), name, alive: true });
                    }
                    Input::Start => self.start(player_id)?,
                    Input::Pass { to } => self.pass(player_id, to)?,
                }
            }
        }
        Ok(Vec::new())
    }

    fn view_for(&self, player_id: &str) -> View {
        let me = self.seat_of(player_id);
        let playing = self.phase == Phase::Playing;
        let can_pass_to = match me {
            Some(seat) if playing && seat == self.holder => self.passable_from(seat),
            _ => Vec::new(),
        };
        View {
            me,
            phase: match self.phase {
                Phase::Waiting => "waiting",
                Phase::Playing => "playing",
                Phase::Over => "over",
            },
            players: self
                .players
                .iter()
                .enumerate()
                .map(|(i, p)| PlayerView { name: p.name.clone(), alive: p.alive, holding: playing && i == self.holder })
                .collect(),
            holder: playing.then_some(self.holder),
            held_for: self.held_for,
            can_pass_to,
            sizzling: playing && self.fuse_left <= SIZZLE_UNDER,
            round: self.round,
            winner: if self.phase == Phase::Over { self.alive().first().copied() } else { None },
            can_join: me.is_none() && !playing && self.players.len() < MAX_PLAYERS,
            can_start: me.is_some() && !playing && self.players.len() >= 2,
            max_players: MAX_PLAYERS,
            log: self.log.clone(),
        }
    }

    fn tick(&mut self) -> Vec<Action> {
        if self.phase == Phase::Playing { vec![Action::Tick] } else { Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seated(n: usize) -> HotPotato {
        let mut g = HotPotato::new(());
        for i in 0..n {
            let id = format!("p{i}");
            g.record_instance_id(&id, &format!("inst{i}"));
            g.update(&id, Action::Input { name: id.clone(), input: Input::Join }).unwrap();
        }
        g
    }

    #[test]
    fn last_one_standing_wins_and_fuse_stays_hidden() {
        let mut g = seated(3);
        g.update("p0", Action::Input { name: "p0".into(), input: Input::Start }).unwrap();
        let json = serde_json::to_value(g.view_for("p1")).unwrap();
        assert!(json.get("fuse_left").is_none());
        for _ in 0..100 {
            g.update("system", Action::Tick).unwrap();
        }
        assert_eq!(g.alive().len(), 1);
        assert!(g.view_for("p0").winner.is_some());
    }

    #[test]
    fn cannot_pass_straight_back_with_three_alive() {
        let mut g = seated(3);
        g.update("p0", Action::Input { name: "p0".into(), input: Input::Start }).unwrap();
        g.fuse_left = 99;
        let a = g.holder;
        let b = (a + 1) % 3;
        g.pass(&format!("p{a}"), b).unwrap();
        assert!(g.pass(&format!("p{b}"), a).is_err());
        assert!(g.pass(&format!("p{a}"), b).is_err());
    }
}
