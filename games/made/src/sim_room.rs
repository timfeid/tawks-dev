//! One lobby's sim: everyone connected sees the same run, and one person at a
//! time drives it. The first person in is the controller; anyone may take
//! control, and when the controller leaves it passes to whoever has been
//! there longest. The relay keeps the last frame (for deltas) and the plots'
//! history (for whoever joins mid-run).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use made_protocol::protocol::{ClientMsg, LogLevel, LogLine, RelayMsg};
use made_protocol::sim::{PlotHistory, SimFrame, SimMeta, SimViewer, SimWelcome};
use made_protocol::simd::{SimdEvent, SimdRequest};
use tokio::sync::mpsc;
use tracing::warn;

use crate::link::SimdLink;
use crate::room::async_trait_free::Backend;
use crate::room::{RoomCmd, RoomConfig};

const IDLE_CLOSE: Duration = Duration::from_secs(120);
const LOG_FLUSH: Duration = Duration::from_secs(2);
const MAX_CONTROLS_PER_SEC: u32 = 90;
const MAX_KEPT_LOGS: usize = 200;

struct Conn {
    user_id: String,
    name: String,
    owner: bool,
    out: mpsc::UnboundedSender<String>,
    needs_full: bool,
    controls_left: u32,
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

pub struct SimRoom {
    lobby_id: String,
    link: SimdLink,
    backend: Arc<dyn Backend>,
    conns: HashMap<u64, Conn>,
    config: Option<RoomConfig>,
    meta: Option<SimMeta>,
    key: Option<String>,
    events: Option<mpsc::UnboundedReceiver<SimdEvent>>,
    opened: u32,
    last_frame: Option<SimFrame>,
    history: PlotHistory,
    controller: Option<u64>,
    crashed: bool,
    empty_since: Option<Instant>,
    pending_logs: Vec<(String, String)>,
    logs: Vec<LogLine>,
}

impl SimRoom {
    pub fn new(lobby_id: String, link: SimdLink, backend: Arc<dyn Backend>) -> Self {
        SimRoom {
            lobby_id,
            link,
            backend,
            conns: HashMap::new(),
            config: None,
            meta: None,
            key: None,
            events: None,
            opened: 0,
            last_frame: None,
            history: PlotHistory::default(),
            controller: None,
            crashed: false,
            empty_since: Some(Instant::now()),
            pending_logs: Vec::new(),
            logs: Vec::new(),
        }
    }

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
                    match self.events.as_mut() {
                        Some(rx) => rx.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    match event {
                        Some(event) => self.sim_event(event),
                        None => self.events = None,
                    }
                }
                _ = tick.tick() => {
                    if last_refill.elapsed() >= Duration::from_secs(1) {
                        last_refill = Instant::now();
                        for conn in self.conns.values_mut() {
                            conn.controls_left = MAX_CONTROLS_PER_SEC;
                        }
                    }
                    if last_flush.elapsed() >= LOG_FLUSH {
                        last_flush = Instant::now();
                        self.flush_logs();
                    }
                    if self.conns.is_empty() {
                        if self.key.is_some() && self.empty_since.is_some_and(|at| at.elapsed() >= Duration::from_secs(20)) {
                            // Nobody is looking, so the run stops; the next person in starts it fresh.
                            self.stop();
                        }
                        if self.empty_since.is_some_and(|at| at.elapsed() >= IDLE_CLOSE) {
                            break;
                        }
                    }
                }
            }
        }
        self.stop();
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

    fn viewer(&self, conn: u64) -> Option<SimViewer> {
        self.conns.get(&conn).map(|c| SimViewer {
            user_id: c.user_id.clone(),
            name: c.name.clone(),
        })
    }

    fn viewers(&self) -> Vec<SimViewer> {
        let mut ids: Vec<&u64> = self.conns.keys().collect();
        ids.sort();
        let mut out: Vec<SimViewer> = Vec::new();
        for id in ids {
            let c = &self.conns[id];
            if !out.iter().any(|v| v.user_id == c.user_id) {
                out.push(SimViewer {
                    user_id: c.user_id.clone(),
                    name: c.name.clone(),
                });
            }
        }
        out
    }

    fn announce_controller(&self) {
        self.broadcast(&RelayMsg::Controller {
            controller: self.controller.and_then(|c| self.viewer(c)),
            viewers: self.viewers(),
        });
    }

    fn welcome(&self, conn: u64) -> Option<String> {
        let config = self.config.as_ref()?;
        let meta = self.meta.clone()?;
        let c = self.conns.get(&conn)?;
        Some(text(&RelayMsg::SimWelcome {
            welcome: SimWelcome {
                user_id: c.user_id.clone(),
                owner: c.owner,
                lobby_id: self.lobby_id.clone(),
                activity_id: config.activity_id.clone(),
                version: config.version,
                meta,
                controller: self.controller.and_then(|c| self.viewer(c)),
                viewers: self.viewers(),
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
                        message: "This lobby is not running a sim.".into(),
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
                        controls_left: MAX_CONTROLS_PER_SEC,
                        over_limit: 0,
                    },
                );
                if self.controller.is_none() || owner {
                    self.controller = Some(conn);
                }
                if self.key.is_none() && !self.crashed {
                    self.open();
                } else if let Some(body) = self.welcome(conn) {
                    let _ = self.conns[&conn].out.send(body);
                    for line in self.logs.iter().rev().take(20).rev() {
                        let _ = self.conns[&conn]
                            .out
                            .send(text(&RelayMsg::Log { line: line.clone() }));
                    }
                }
                self.announce_controller();
                self.log(LogLevel::Info, format!("{name} joined"));
            }
            RoomCmd::Leave { conn } => {
                if self.conns.remove(&conn).is_some() {
                    if self.controller == Some(conn) {
                        self.controller = self.conns.keys().min().copied();
                    }
                    self.announce_controller();
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
            ClientMsg::Sim { control } => {
                if conn.controls_left == 0 {
                    conn.over_limit += 1;
                    if conn.over_limit > 600 {
                        let _ = conn.out.send(text(&RelayMsg::Error {
                            message: "Too many changes at once.".into(),
                            fatal: true,
                        }));
                    }
                    return;
                }
                conn.controls_left -= 1;
                if self.controller != Some(conn_id) {
                    let _ = conn.out.send(text(&RelayMsg::Error {
                        message: "Take control to change the sim.".into(),
                        fatal: false,
                    }));
                    return;
                }
                let Some(control) = control.sanitized() else {
                    return;
                };
                if let Some(key) = &self.key {
                    self.link.send(SimdRequest::SimControl {
                        room: key.clone(),
                        control,
                    });
                }
            }
            ClientMsg::TakeControl => {
                if self.controller != Some(conn_id) {
                    self.controller = Some(conn_id);
                    let name = self.conns[&conn_id].name.clone();
                    self.announce_controller();
                    self.log(LogLevel::Info, format!("{name} took control"));
                }
            }
            ClientMsg::Ping { t } => {
                let _ = conn.out.send(text(&RelayMsg::Pong { t, now: now_ms() }));
            }
            ClientMsg::Resync => conn.needs_full = true,
            ClientMsg::Restart => {
                if !conn.owner {
                    let _ = conn.out.send(text(&RelayMsg::Error {
                        message: "Only whoever started this test lobby can reload it.".into(),
                        fatal: false,
                    }));
                    return;
                }
                self.load_config().await;
                self.crashed = false;
                self.open();
            }
            ClientMsg::Input { .. } | ClientMsg::Hello { .. } => {}
        }
    }

    async fn load_config(&mut self) {
        match self.backend.room(&self.lobby_id).await {
            Some(config) => self.config = Some(config),
            None => warn!(lobby = %self.lobby_id, "no sim for this lobby"),
        }
    }

    fn stop(&mut self) {
        if let Some(key) = self.key.take() {
            self.link.unsubscribe(&key);
        }
        self.events = None;
    }

    fn open(&mut self) {
        self.stop();
        let Some(config) = self.config.clone() else {
            return;
        };
        self.opened += 1;
        let key = format!("{}#sim{}", self.lobby_id, self.opened);
        self.events = Some(self.link.subscribe(&key));
        self.key = Some(key.clone());
        self.last_frame = None;
        self.history.clear();
        self.crashed = false;
        for conn in self.conns.values_mut() {
            conn.needs_full = true;
        }
        self.link.send(SimdRequest::SimOpen {
            room: key,
            source: config.source.clone(),
            seed: rand::random(),
        });
    }

    fn sim_event(&mut self, event: SimdEvent) {
        match event {
            SimdEvent::SimOpened { meta, .. } => {
                self.meta = Some(*meta);
                let ids: Vec<u64> = self.conns.keys().copied().collect();
                for id in ids {
                    if let Some(body) = self.welcome(id) {
                        let _ = self.conns[&id].out.send(body);
                    }
                }
            }
            SimdEvent::SimFrame { frame, .. } => {
                let frame = *frame;
                if self.last_frame.as_ref().is_some_and(|last| {
                    last.plots
                        .iter()
                        .zip(frame.plots.iter())
                        .any(|(a, b)| a.epoch != b.epoch)
                }) {
                    self.history.clear();
                }
                self.history.absorb(&frame.plots);
                let delta = frame.diff(self.last_frame.as_ref());
                let mut full_body: Option<String> = None;
                let delta_body = text(&RelayMsg::SimFrame { frame: delta });
                for conn in self.conns.values_mut() {
                    if conn.needs_full || self.last_frame.is_none() {
                        let body = full_body.get_or_insert_with(|| {
                            let mut full = frame.diff(None);
                            full.plots = self.history.full();
                            text(&RelayMsg::SimFrame { frame: full })
                        });
                        let _ = conn.out.send(body.clone());
                        conn.needs_full = false;
                    } else {
                        let _ = conn.out.send(delta_body.clone());
                    }
                }
                self.last_frame = Some(frame);
            }
            SimdEvent::Log { line, .. } => self.log(line.level, line.text),
            SimdEvent::Failed { message, .. } => {
                self.crashed = true;
                self.log(LogLevel::Error, format!("the sim stopped: {message}"));
                self.broadcast(&RelayMsg::Error {
                    message: "The sim stopped working. The Builder agent can read what went wrong."
                        .into(),
                    fatal: false,
                });
                if let Some(key) = self.key.take() {
                    self.link.unsubscribe(&key);
                }
                self.events = None;
            }
            _ => {}
        }
    }
}
