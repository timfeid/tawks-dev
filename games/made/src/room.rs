//! One lobby's room: who is connected, which seat each plays, the sandbox
//! round it is watching, and the last frame, which the next is diffed from.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait_free::Backend;
use made_protocol::protocol::{ClientMsg, EndReason, LogLevel, LogLine, RelayMsg, Welcome};
use made_protocol::simd::{SimSeat, SimdEvent, SimdRequest};
use made_protocol::{GameMeta, Input, MAX_INPUTS_PER_SEC, RosterSeat, SceneFrame, Skill};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::warn;

use crate::link::SimdLink;
use crate::token::Claims;

const IDLE_CLOSE: Duration = Duration::from_secs(120);
const AUTO_RESTART: Duration = Duration::from_secs(8);
/// A hub lobby's first round waits this long after the first person comes in,
/// so people who open the game together start it together.
const GATHER: Duration = Duration::from_secs(4);
const LOG_FLUSH: Duration = Duration::from_secs(2);
const MAX_KEPT_LOGS: usize = 200;

/// The API's answers, behind a trait so tests can stand in for it.
pub mod async_trait_free {
    use std::future::Future;
    use std::pin::Pin;

    use super::RoomConfig;

    pub type Fut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

    pub trait Backend: Send + Sync + 'static {
        fn room<'a>(&'a self, lobby_id: &'a str) -> Fut<'a, Option<RoomConfig>>;
        fn costume<'a>(&'a self, user_id: &'a str) -> Fut<'a, Option<serde_json::Value>>;
        fn log<'a>(
            &'a self,
            lobby_id: &'a str,
            activity_id: &'a str,
            version: i32,
            lines: Vec<(String, String)>,
        ) -> Fut<'a, ()>;
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct Seat {
    pub seat: u8,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub bot: bool,
    #[serde(default)]
    pub skill: Option<Skill>,
    #[serde(default)]
    pub name: String,
}

/// The API's `made_room` answer.
#[derive(Deserialize, Clone, Debug)]
pub struct RoomConfig {
    pub activity_id: String,
    pub version: i32,
    pub source: String,
    #[serde(default)]
    pub owner_user_id: Option<String>,
    #[serde(default)]
    pub seats: Vec<Seat>,
}

pub enum RoomCmd {
    Join {
        conn: u64,
        claims: Claims,
        out: mpsc::UnboundedSender<String>,
    },
    Leave {
        conn: u64,
    },
    Client {
        conn: u64,
        msg: ClientMsg,
    },
}

struct Conn {
    user_id: String,
    name: String,
    owner: bool,
    out: mpsc::UnboundedSender<String>,
    needs_full: bool,
    /// Input budget, refilled each second.
    inputs_left: u32,
    over_limit: u32,
}

fn text(msg: &RelayMsg) -> String {
    serde_json::to_string(msg).unwrap_or_default()
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

pub struct Room {
    lobby_id: String,
    link: SimdLink,
    backend: Arc<dyn Backend>,
    conns: HashMap<u64, Conn>,
    config: Option<RoomConfig>,
    roster: Vec<RosterSeat>,
    meta: Option<GameMeta>,
    sim_key: Option<String>,
    sim_events: Option<mpsc::UnboundedReceiver<SimdEvent>>,
    round: u32,
    last_frame: Option<SceneFrame>,
    running: bool,
    ended_at: Option<Instant>,
    crashed: bool,
    starts_at: Option<Instant>,
    empty_since: Option<Instant>,
    pending_logs: Vec<(String, String)>,
    logs: Vec<LogLine>,
}

impl Room {
    pub fn new(lobby_id: String, link: SimdLink, backend: Arc<dyn Backend>) -> Self {
        Room {
            lobby_id,
            link,
            backend,
            conns: HashMap::new(),
            config: None,
            roster: Vec::new(),
            meta: None,
            sim_key: None,
            sim_events: None,
            round: 0,
            last_frame: None,
            running: false,
            ended_at: None,
            crashed: false,
            starts_at: None,
            empty_since: Some(Instant::now()),
            pending_logs: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Runs until the room has been empty for a while. Returns so the server forgets it.
    pub async fn run(mut self, mut commands: mpsc::UnboundedReceiver<RoomCmd>) {
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        let mut last_flush = Instant::now();
        let mut last_refill = Instant::now();
        loop {
            tokio::select! {
                cmd = commands.recv() => {
                    let Some(cmd) = cmd else { break };
                    self.command(cmd).await;
                }
                event = async {
                    match self.sim_events.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match event {
                        Some(event) => self.sim_event(event),
                        None => self.sim_events = None,
                    }
                }
                _ = tick.tick() => {
                    if last_refill.elapsed() >= Duration::from_secs(1) {
                        last_refill = Instant::now();
                        for conn in self.conns.values_mut() {
                            conn.inputs_left = MAX_INPUTS_PER_SEC;
                        }
                    }
                    if last_flush.elapsed() >= LOG_FLUSH {
                        last_flush = Instant::now();
                        self.flush_logs();
                    }
                    if let Some(at) = self.ended_at {
                        if !self.crashed && at.elapsed() >= AUTO_RESTART && !self.conns.is_empty() {
                            if self.hub_lobby() {
                                self.load_config().await;
                            }
                            self.start_round().await;
                        }
                    }
                    if self.starts_at.is_some_and(|at| Instant::now() >= at) {
                        self.starts_at = None;
                        if !self.running && !self.conns.is_empty() {
                            self.start_round().await;
                        }
                    }
                    if self.conns.is_empty() && self.empty_since.is_some_and(|at| at.elapsed() >= IDLE_CLOSE) {
                        break;
                    }
                }
            }
        }
        self.stop_sim();
        self.flush_logs();
    }

    fn log(&mut self, level: LogLevel, text: String) {
        let line = LogLine {
            at: now_ms(),
            level,
            text,
        };
        let name = match level {
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        self.pending_logs.push((name.into(), line.text.clone()));
        self.logs.push(line.clone());
        if self.logs.len() > MAX_KEPT_LOGS {
            self.logs.remove(0);
        }
        self.broadcast(&RelayMsg::Log { line });
    }

    fn flush_logs(&mut self) {
        if self.pending_logs.is_empty() {
            return;
        }
        let Some(config) = &self.config else { return };
        let lines = std::mem::take(&mut self.pending_logs);
        let backend = self.backend.clone();
        let (lobby, activity, version) = (
            self.lobby_id.clone(),
            config.activity_id.clone(),
            config.version,
        );
        tokio::spawn(async move { backend.log(&lobby, &activity, version, lines).await });
    }

    fn broadcast(&self, msg: &RelayMsg) {
        let body = text(msg);
        for conn in self.conns.values() {
            let _ = conn.out.send(body.clone());
        }
    }

    /// A hub lobby seats whoever is connected; a test lobby has its seats from the API.
    fn hub_lobby(&self) -> bool {
        self.config.as_ref().is_some_and(|c| c.seats.is_empty())
    }

    fn seat_of(&self, user_id: &str) -> Option<u8> {
        self.roster
            .iter()
            .find(|s| !s.bot && s.user_id.as_deref() == Some(user_id))
            .map(|s| s.seat)
    }

    fn mark_connected(&mut self) {
        let connected: Vec<String> = self.conns.values().map(|c| c.user_id.clone()).collect();
        for seat in &mut self.roster {
            seat.connected = seat.user_id.as_ref().is_some_and(|u| connected.contains(u));
        }
    }

    fn welcome(&self, conn: &Conn) -> Option<String> {
        let config = self.config.as_ref()?;
        let meta = self.meta.clone()?;
        Some(text(&RelayMsg::Welcome {
            welcome: Welcome {
                seat: self.seat_of(&conn.user_id),
                user_id: conn.user_id.clone(),
                owner: conn.owner,
                lobby_id: self.lobby_id.clone(),
                activity_id: config.activity_id.clone(),
                version: config.version,
                meta,
                roster: self.roster.clone(),
            },
        }))
    }

    async fn command(&mut self, cmd: RoomCmd) {
        match cmd {
            RoomCmd::Join { conn, claims, out } => {
                if self.config.is_none()
                    || claims.version > self.config.as_ref().map(|c| c.version).unwrap_or(0)
                {
                    self.load_config().await;
                }
                let Some(config) = &self.config else {
                    let _ = out.send(text(&RelayMsg::Error {
                        message: "This lobby is not playing a made game.".into(),
                        fatal: true,
                    }));
                    return;
                };
                if claims.activity_id != config.activity_id || claims.version != config.version {
                    let _ = out.send(text(&RelayMsg::Error {
                        message: "stale_pass".into(),
                        fatal: true,
                    }));
                    return;
                }
                let owner = claims.owner
                    || config.owner_user_id.as_deref() == Some(claims.user_id.as_str());
                self.empty_since = None;
                let name = claims.name.clone();
                self.conns.insert(
                    conn,
                    Conn {
                        user_id: claims.user_id,
                        name: name.clone(),
                        owner,
                        out,
                        needs_full: true,
                        inputs_left: MAX_INPUTS_PER_SEC,
                        over_limit: 0,
                    },
                );
                if !self.running && self.ended_at.is_none() && !self.crashed {
                    if !self.hub_lobby() {
                        self.start_round().await;
                    } else if self.starts_at.is_none() {
                        self.starts_at = Some(Instant::now() + GATHER);
                    }
                } else {
                    self.mark_connected();
                    if let Some(body) = self.conns.get(&conn).and_then(|c| self.welcome(c)) {
                        let _ = self.conns[&conn].out.send(body);
                    }
                    for line in self.logs.iter().rev().take(20).rev() {
                        let _ = self.conns[&conn]
                            .out
                            .send(text(&RelayMsg::Log { line: line.clone() }));
                    }
                    self.broadcast(&RelayMsg::Roster {
                        roster: self.roster.clone(),
                    });
                }
                self.log(LogLevel::Info, format!("{name} joined"));
            }
            RoomCmd::Leave { conn } => {
                if self.conns.remove(&conn).is_some() {
                    self.mark_connected();
                    self.broadcast(&RelayMsg::Roster {
                        roster: self.roster.clone(),
                    });
                }
                if self.conns.is_empty() {
                    self.empty_since = Some(Instant::now());
                }
            }
            RoomCmd::Client { conn, msg } => self.client(conn, msg).await,
        }
    }

    async fn client(&mut self, conn_id: u64, msg: ClientMsg) {
        let Some(conn) = self.conns.get_mut(&conn_id) else {
            return;
        };
        match msg {
            ClientMsg::Input { input } => {
                if conn.inputs_left == 0 {
                    conn.over_limit += 1;
                    if conn.over_limit > 600 {
                        let _ = conn.out.send(text(&RelayMsg::Error {
                            message: "Too many inputs.".into(),
                            fatal: true,
                        }));
                    }
                    return;
                }
                conn.inputs_left -= 1;
                let user = conn.user_id.clone();
                if let (Some(seat), Some(key)) = (self.seat_of(&user), self.sim_key.clone()) {
                    if self.running {
                        self.link.send(SimdRequest::Input {
                            room: key,
                            seat,
                            input: Input::sanitized(input),
                        });
                    }
                }
            }
            ClientMsg::Ping { t } => {
                let _ = conn.out.send(text(&RelayMsg::Pong { t, now: now_ms() }));
            }
            ClientMsg::Resync => conn.needs_full = true,
            ClientMsg::Restart => {
                if !conn.owner {
                    let _ = conn.out.send(text(&RelayMsg::Error {
                        message: "Only whoever started this test lobby can restart it.".into(),
                        fatal: false,
                    }));
                    return;
                }
                self.load_config().await;
                self.start_round().await;
            }
            ClientMsg::Hello { .. } => {}
        }
    }

    async fn load_config(&mut self) {
        match self.backend.room(&self.lobby_id).await {
            Some(config) => self.config = Some(config),
            None => warn!(lobby = %self.lobby_id, "no made game for this lobby"),
        }
    }

    async fn build_roster(&mut self) -> Vec<SimSeat> {
        let Some(config) = self.config.clone() else {
            return Vec::new();
        };
        let mut seats = config.seats.clone();
        if seats.is_empty() {
            // A hub lobby: everyone connected plays, in the order they came.
            let mut conns: Vec<(&u64, &Conn)> = self.conns.iter().collect();
            conns.sort_by_key(|(id, _)| **id);
            let mut users: Vec<(String, String)> = Vec::new();
            for (_, conn) in conns {
                if !users.iter().any(|(user, _)| *user == conn.user_id) {
                    users.push((conn.user_id.clone(), conn.name.clone()));
                }
            }
            seats = users
                .into_iter()
                .take(8)
                .enumerate()
                .map(|(i, (user, name))| Seat {
                    seat: i as u8 + 1,
                    user_id: Some(user),
                    bot: false,
                    skill: None,
                    name,
                })
                .collect();
            while seats.len() < 2 {
                seats.push(Seat {
                    seat: seats.len() as u8 + 1,
                    user_id: None,
                    bot: true,
                    skill: Some(Skill::Steady),
                    name: format!("Bot {}", seats.len() + 1),
                });
            }
        }
        let mut roster = Vec::new();
        for seat in &seats {
            let costume = match (&seat.user_id, seat.bot) {
                (Some(user), false) => self.backend.costume(user).await,
                _ => None,
            };
            roster.push(RosterSeat {
                seat: seat.seat,
                name: seat.name.clone(),
                bot: seat.bot,
                skill: if seat.bot {
                    Some(seat.skill.unwrap_or_default())
                } else {
                    None
                },
                user_id: if seat.bot { None } else { seat.user_id.clone() },
                avatar_url: None,
                costume,
                connected: false,
            });
        }
        self.roster = roster;
        self.mark_connected();
        self.roster
            .iter()
            .map(|s| SimSeat {
                seat: s.seat,
                name: s.name.clone(),
                bot: s.bot,
                skill: s.skill.unwrap_or_default().level(),
            })
            .collect()
    }

    fn stop_sim(&mut self) {
        if let Some(key) = self.sim_key.take() {
            self.link.unsubscribe(&key);
        }
        self.sim_events = None;
        self.running = false;
    }

    async fn start_round(&mut self) {
        self.stop_sim();
        let Some(config) = self.config.clone() else {
            return;
        };
        let seats = self.build_roster().await;
        self.round += 1;
        let key = format!("{}#{}", self.lobby_id, self.round);
        self.sim_events = Some(self.link.subscribe(&key));
        self.sim_key = Some(key.clone());
        self.last_frame = None;
        self.ended_at = None;
        self.crashed = false;
        self.running = true;
        for conn in self.conns.values_mut() {
            conn.needs_full = true;
        }
        self.link.send(SimdRequest::Open {
            room: key,
            source: config.source.clone(),
            seats,
            seed: rand::random(),
        });
    }

    fn sim_event(&mut self, event: SimdEvent) {
        match event {
            SimdEvent::Opened { meta, .. } => {
                let first = self.meta.is_none();
                self.meta = Some(meta);
                if first || self.round > 0 {
                    for conn in self.conns.values() {
                        if let Some(body) = self.welcome(conn) {
                            let _ = conn.out.send(body);
                        }
                    }
                }
                if self.round > 1 {
                    let version = self.config.as_ref().map(|c| c.version).unwrap_or(0);
                    self.broadcast(&RelayMsg::Restarting { version });
                }
            }
            SimdEvent::Frame { frame, .. } => {
                let delta = frame.diff(self.last_frame.as_ref());
                let mut full_body: Option<String> = None;
                let delta_body = text(&RelayMsg::Frame { frame: delta });
                for conn in self.conns.values_mut() {
                    if conn.needs_full || self.last_frame.is_none() {
                        let body = full_body.get_or_insert_with(|| {
                            text(&RelayMsg::Frame {
                                frame: frame.diff(None),
                            })
                        });
                        let _ = conn.out.send(body.clone());
                        conn.needs_full = false;
                    } else {
                        let _ = conn.out.send(delta_body.clone());
                    }
                }
                self.last_frame = Some(frame);
            }
            SimdEvent::End {
                placements, reason, ..
            } => {
                self.running = false;
                self.ended_at = Some(Instant::now());
                let names: Vec<String> = placements
                    .first()
                    .map(|g| {
                        g.iter()
                            .filter_map(|s| {
                                self.roster
                                    .iter()
                                    .find(|r| r.seat == *s)
                                    .map(|r| r.name.clone())
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.broadcast(&RelayMsg::End {
                    placements,
                    reason: reason.clone(),
                });
                let how = match reason {
                    EndReason::Timeout => "ran out of time",
                    _ => "ended",
                };
                self.log(
                    LogLevel::Info,
                    format!(
                        "round {} {how}; won by {}",
                        self.round,
                        if names.is_empty() {
                            "nobody".into()
                        } else {
                            names.join(" and ")
                        }
                    ),
                );
                if let Some(key) = self.sim_key.take() {
                    self.link.unsubscribe(&key);
                }
                self.sim_events = None;
            }
            SimdEvent::Log { line, .. } => self.log(line.level, line.text),
            SimdEvent::Failed { message, .. } => {
                self.running = false;
                self.crashed = true;
                self.ended_at = Some(Instant::now());
                self.log(LogLevel::Error, format!("the game stopped: {message}"));
                self.broadcast(&RelayMsg::End {
                    placements: Vec::new(),
                    reason: EndReason::Crashed,
                });
                if let Some(key) = self.sim_key.take() {
                    self.link.unsubscribe(&key);
                }
                self.sim_events = None;
            }
            SimdEvent::PlayoutDone { .. } | SimdEvent::Pong => {}
        }
    }
}
