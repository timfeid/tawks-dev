//! The drive socket on the party port, `/party/outroads/ws`: one room per
//! lobby in memory, car poses relayed ten times a second, and place search
//! answered through the shared geocoder.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::geocode::{Found, Geocoder};
use crate::heist::{self, game::Heist, view};
use crate::pass::{self, Pass};
use crate::room::{Action, Changed, Place, Pose, Room, RoomView};
use crate::terrain::TerrainProxy;

const TICK: Duration = Duration::from_millis(100);
const IDLE_CLOSE: Duration = Duration::from_secs(10 * 60);
const POSES_PER_SECOND: u32 = 40;
const ACTIONS_PER_SECOND: u32 = 8;
const HONKS_PER_SECOND: u32 = 4;
const HEIST_ACTS_PER_SECOND: u32 = 30;
const SEARCH_EVERY: Duration = Duration::from_millis(700);
const MAX_MESSAGE_BYTES: usize = 8 * 1024;

pub const ATTRIBUTION: &str = "Map data (c) OpenStreetMap contributors, ODbL. Tiles by OpenFreeMap, schema (c) OpenMapTiles. Search by Photon from komoot.";

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Hello {
        token: String,
        #[serde(default)]
        place: Option<Place>,
    },
    Ping {
        t: f64,
    },
    Pose {
        pose: Pose,
    },
    Honk,
    Act {
        action: Action,
    },
    Search {
        q: String,
        #[serde(default)]
        near: Option<(f64, f64)>,
    },
    /// The host turns the lobby's drive into a run of Port Run, or back.
    HeistOpen,
    HeistClose,
    Heist {
        msg: serde_json::Value,
    },
}

#[derive(Serialize)]
struct Car<'a> {
    id: &'a str,
    #[serde(flatten)]
    pose: Pose,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg<'a> {
    Welcome { user_id: &'a str, attribution: &'a str },
    Room { room: RoomView<'a> },
    Poses { cars: Vec<Car<'a>> },
    Honk { id: &'a str },
    Results { q: &'a str, places: &'a [Found] },
    Pong { t: f64 },
    Error { message: &'a str },
    Heist { view: serde_json::Value },
    HeistWorld { world: serde_json::Value },
    HeistPoses { poses: serde_json::Value },
    HeistEvent { event: serde_json::Value },
}

fn text(msg: &ServerMsg) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

enum Cmd {
    Connect {
        conn: u64,
        pass: Pass,
        resume: Option<Place>,
        out: mpsc::UnboundedSender<Message>,
    },
    Disconnect {
        conn: u64,
    },
    Act {
        conn: u64,
        action: Action,
    },
    Pose {
        conn: u64,
        pose: Pose,
    },
    Honk {
        conn: u64,
    },
    HeistOpen {
        conn: u64,
        open: bool,
    },
    Heist {
        conn: u64,
        msg: serde_json::Value,
    },
}

/// Where finished runs are paid and lobby predictions settled.
pub trait Reports: Send + Sync {
    fn started(&self, lobby_id: &str, players: Vec<String>);
    fn finished(&self, lobby_id: &str, reference: &str, results: &crate::heist::game::Results);
}

struct Inner {
    secret: [u8; 32],
    rooms: DashMap<String, mpsc::UnboundedSender<Cmd>>,
    next_conn: AtomicU64,
    ws_url: String,
    geocoder: Geocoder,
    terrain: TerrainProxy,
    reports: std::sync::RwLock<Option<Arc<dyn Reports>>>,
    dev: bool,
}

#[derive(Clone)]
pub struct OutroadsServer {
    inner: Arc<Inner>,
}

impl OutroadsServer {
    pub fn new(secret: [u8; 32], ws_url: String, geocoder: Geocoder) -> Self {
        OutroadsServer {
            inner: Arc::new(Inner {
                secret,
                rooms: DashMap::new(),
                next_conn: AtomicU64::new(1),
                ws_url,
                geocoder,
                terrain: TerrainProxy::from_env(),
                reports: std::sync::RwLock::new(None),
                dev: ["OUTROADS_DEV_TOOLS", "PARTY_DEV_TOOLS"]
                    .iter()
                    .any(|k| std::env::var(k).is_ok_and(|v| v == "1" || v == "true")),
            }),
        }
    }

    pub fn from_env(party_public_url: &str) -> Self {
        Self::new(
            pass::secret_from_env(),
            ws_url_for(party_public_url),
            Geocoder::from_env(),
        )
    }

    pub fn ws_url(&self) -> &str {
        &self.inner.ws_url
    }

    pub fn issue_pass(&self, lobby_id: &str, user_id: &str, name: &str, avatar_url: &str, owner: bool) -> String {
        pass::sign(
            &Pass {
                lobby_id: lobby_id.to_string(),
                user_id: user_id.to_string(),
                name: name.to_string(),
                avatar_url: avatar_url.to_string(),
                owner,
                exp: pass::now_unix() + pass::PASS_TTL_SECS,
            },
            &self.inner.secret,
        )
    }

    pub fn set_reports(&self, reports: Arc<dyn Reports>) {
        *self.inner.reports.write().unwrap() = Some(reports);
    }

    pub fn with_dev_tools(self, on: bool) -> Self {
        let mut inner =
            Arc::try_unwrap(self.inner).unwrap_or_else(|_| panic!("set dev tools before sharing the server"));
        inner.dev = on;
        OutroadsServer { inner: Arc::new(inner) }
    }

    fn reports(&self) -> Option<Arc<dyn Reports>> {
        self.inner.reports.read().unwrap().clone()
    }

    pub fn terrain(&self) -> &TerrainProxy {
        &self.inner.terrain
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/party/outroads/ws", get(ws_handler))
            .route("/party/outroads/terrain/:z/:x/:file", get(crate::terrain::handler))
            .with_state(self.clone())
    }

    fn room(&self, lobby_id: &str) -> mpsc::UnboundedSender<Cmd> {
        self.inner
            .rooms
            .entry(lobby_id.to_string())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(run_room(self.clone(), lobby_id.to_string(), rx));
                tx
            })
            .clone()
    }
}

/// `wss://host/party/ws` becomes `wss://host/party/outroads/ws`.
pub fn ws_url_for(party_public_url: &str) -> String {
    let base = party_public_url.trim_end_matches('/').trim_end_matches("/party/ws");
    format!("{base}/party/outroads/ws")
}

async fn ws_handler(State(server): State<OutroadsServer>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| serve_socket(server, socket))
}

/// Poses and the take's presses spend separate budgets, so walking never
/// eats the release that stops a start.
fn heist_budget(msg: &serde_json::Value, poses: &mut Budget, acts: &mut Budget) -> bool {
    match msg.get("op").and_then(|op| op.as_str()) {
        Some("vehicle" | "foot") => poses.take(),
        _ => acts.take(),
    }
}

struct Budget {
    window: Instant,
    used: u32,
    limit: u32,
}

impl Budget {
    fn new(limit: u32) -> Budget {
        Budget {
            window: Instant::now(),
            used: 0,
            limit,
        }
    }

    fn take(&mut self) -> bool {
        if self.window.elapsed() >= Duration::from_secs(1) {
            self.window = Instant::now();
            self.used = 0;
        }
        self.used += 1;
        self.used <= self.limit
    }
}

async fn serve_socket(server: OutroadsServer, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (pass, resume) = loop {
        match stream.next().await {
            Some(Ok(Message::Text(raw))) => match serde_json::from_str::<ClientMsg>(&raw) {
                Ok(ClientMsg::Hello { token, place }) => {
                    match pass::verify(&token, &server.inner.secret, pass::now_unix()) {
                        Some(pass) => break (pass, place),
                        None => {
                            let _ = sink
                                .send(text(&ServerMsg::Error {
                                    message: "pass_expired",
                                }))
                                .await;
                            return;
                        }
                    }
                }
                _ => continue,
            },
            Some(Ok(_)) => continue,
            _ => return,
        }
    };

    let conn = server.inner.next_conn.fetch_add(1, Ordering::Relaxed);
    let (out, mut outbox) = mpsc::unbounded_channel::<Message>();
    let lobby_id = pass.lobby_id.clone();
    let mut room = server.room(&lobby_id);
    if let Err(mpsc::error::SendError(Cmd::Connect { pass, resume, out, .. })) = room.send(Cmd::Connect {
        conn,
        pass,
        resume,
        out: out.clone(),
    }) {
        server.inner.rooms.remove_if(&lobby_id, |_, tx| tx.is_closed());
        room = server.room(&lobby_id);
        let _ = room.send(Cmd::Connect {
            conn,
            pass,
            resume,
            out,
        });
    }

    let writer = tokio::spawn(async move {
        while let Some(message) = outbox.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut poses = Budget::new(POSES_PER_SECOND);
    let mut actions = Budget::new(ACTIONS_PER_SECOND);
    let mut honks = Budget::new(HONKS_PER_SECOND);
    let mut heist_acts = Budget::new(HEIST_ACTS_PER_SECOND);
    let mut last_search = Instant::now() - SEARCH_EVERY;
    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(raw) = message else { continue };
        let Ok(msg) = serde_json::from_str::<ClientMsg>(&raw) else {
            let _ = out.send(text(&ServerMsg::Error {
                message: "The drive did not understand that",
            }));
            continue;
        };
        let sent = match msg {
            ClientMsg::Ping { t } => {
                let _ = out.send(text(&ServerMsg::Pong { t }));
                Ok(())
            }
            ClientMsg::Hello { .. } => Ok(()),
            ClientMsg::Pose { pose } if poses.take() => match pose.checked() {
                Some(pose) => room.send(Cmd::Pose { conn, pose }).map_err(drop),
                None => Ok(()),
            },
            ClientMsg::Pose { .. } => Ok(()),
            ClientMsg::Honk if honks.take() => room.send(Cmd::Honk { conn }).map_err(drop),
            ClientMsg::Honk => Ok(()),
            ClientMsg::HeistOpen => room.send(Cmd::HeistOpen { conn, open: true }).map_err(drop),
            ClientMsg::HeistClose => room.send(Cmd::HeistOpen { conn, open: false }).map_err(drop),
            ClientMsg::Heist { msg } if heist_budget(&msg, &mut poses, &mut heist_acts) => {
                room.send(Cmd::Heist { conn, msg }).map_err(drop)
            }
            ClientMsg::Heist { .. } => Ok(()),
            ClientMsg::Act { action } if actions.take() => room.send(Cmd::Act { conn, action }).map_err(drop),
            ClientMsg::Act { .. } => {
                let _ = out.send(text(&ServerMsg::Error {
                    message: "Slow down a little",
                }));
                Ok(())
            }
            ClientMsg::Search { q, near } => {
                if last_search.elapsed() < SEARCH_EVERY {
                    tokio::time::sleep(SEARCH_EVERY - last_search.elapsed()).await;
                }
                last_search = Instant::now();
                let near = near.filter(|(lat, lon)| lat.is_finite() && lon.is_finite());
                let (server, out) = (server.clone(), out.clone());
                tokio::spawn(async move {
                    let message = match server.inner.geocoder.search(&q, near).await {
                        Ok(places) => text(&ServerMsg::Results { q: &q, places: &places }),
                        Err(message) => text(&ServerMsg::Error { message }),
                    };
                    let _ = out.send(message);
                });
                Ok(())
            }
        };
        if sent.is_err() {
            break;
        }
    }
    let _ = room.send(Cmd::Disconnect { conn });
    writer.abort();
}

struct Seat {
    user_id: String,
    out: mpsc::UnboundedSender<Message>,
}

fn broadcast(seats: &mut HashMap<u64, Seat>, message: Message, except: Option<u64>) {
    seats.retain(|conn, seat| Some(*conn) == except || seat.out.send(message.clone()).is_ok());
}

async fn run_room(server: OutroadsServer, lobby_id: String, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let mut room = Room::new();
    let mut seats: HashMap<u64, Seat> = HashMap::new();
    let mut poses: HashMap<String, Pose> = HashMap::new();
    let mut moved: Vec<String> = Vec::new();
    let mut empty_since = Some(Instant::now());
    let born = Instant::now();
    let mut heist: Option<Heist> = None;
    let mut heist_ticks: u64 = 0;
    let mut sent_world: u32 = u32::MAX;
    let mut heist_ref = String::new();
    let mut ticker = tokio::time::interval(TICK);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            cmd = commands.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    Cmd::Connect { conn, pass, resume, out } => {
                        let user_id = pass.user_id.clone();
                        room.join(&pass);
                        if let Some(place) = resume {
                            room.resume(place);
                        }
                        let _ = out.send(text(&ServerMsg::Welcome { user_id: &user_id, attribution: ATTRIBUTION }));
                        let cars: Vec<Car> = poses.iter().map(|(id, pose)| Car { id, pose: *pose }).collect();
                        let _ = out.send(text(&ServerMsg::Poses { cars }));
                        if let Some(h) = heist.as_mut() {
                            let name = room.driver(&user_id).map(|d| d.name.clone()).unwrap_or_default();
                            h.join(&user_id, &name);
                            let now = born.elapsed().as_secs_f64();
                            let _ = out.send(text(&ServerMsg::HeistWorld { world: view::world(h) }));
                            let _ = out.send(text(&ServerMsg::Heist { view: view::view(h, &user_id, now) }));
                        }
                        seats.insert(conn, Seat { user_id, out });
                        empty_since = None;
                        broadcast(&mut seats, text(&ServerMsg::Room { room: room.view() }), None);
                    }
                    Cmd::Disconnect { conn } => {
                        if let Some(seat) = seats.remove(&conn)
                            && !seats.values().any(|s| s.user_id == seat.user_id)
                        {
                            room.leave(&seat.user_id);
                            poses.remove(&seat.user_id);
                            if let Some(h) = heist.as_mut() {
                                h.leave(&seat.user_id);
                            }
                            broadcast(&mut seats, text(&ServerMsg::Room { room: room.view() }), None);
                        }
                        if seats.is_empty() {
                            empty_since = Some(Instant::now());
                        }
                    }
                    Cmd::Act { conn, action } => {
                        let Some(user_id) = seats.get(&conn).map(|s| s.user_id.clone()) else { continue };
                        match room.act(&user_id, action) {
                            Ok(changed) => {
                                if changed == Changed::Moved {
                                    poses.clear();
                                    moved.clear();
                                }
                                broadcast(&mut seats, text(&ServerMsg::Room { room: room.view() }), None);
                            }
                            Err(message) => {
                                if let Some(seat) = seats.get(&conn) {
                                    let _ = seat.out.send(text(&ServerMsg::Error { message }));
                                }
                            }
                        }
                    }
                    Cmd::Pose { conn, pose } => {
                        let Some(user_id) = seats.get(&conn).map(|s| s.user_id.clone()) else { continue };
                        if room.place.is_some() && poses.get(&user_id) != Some(&pose) {
                            poses.insert(user_id.clone(), pose);
                            if !moved.contains(&user_id) {
                                moved.push(user_id);
                            }
                        }
                    }
                    Cmd::Honk { conn } => {
                        let Some(user_id) = seats.get(&conn).map(|s| s.user_id.clone()) else { continue };
                        broadcast(&mut seats, text(&ServerMsg::Honk { id: &user_id }), Some(conn));
                    }
                    Cmd::HeistOpen { conn, open } => {
                        let Some(user_id) = seats.get(&conn).map(|s| s.user_id.clone()) else { continue };
                        if room.host() != Some(user_id.as_str()) {
                            if let Some(seat) = seats.get(&conn) {
                                let _ = seat.out.send(text(&ServerMsg::Error { message: "Only the host can start Port Run" }));
                            }
                            continue;
                        }
                        if open && heist.is_none() {
                            let port = heist::rules::PORT;
                            let _ = room.act(&user_id, Action::SetPlace { place: Place { name: "Port Newark".into(), lat: port.lat, lon: port.lon } });
                            poses.clear();
                            moved.clear();
                            let mut h = Heist::new(server.inner.dev, rand::random());
                            for d in room.view().drivers {
                                h.join(&d.user_id, &d.name);
                            }
                            heist = Some(h);
                            sent_world = u32::MAX;
                        } else if !open {
                            heist = None;
                            broadcast(&mut seats, text(&ServerMsg::Heist { view: serde_json::Value::Null }), None);
                        }
                        broadcast(&mut seats, text(&ServerMsg::Room { room: room.view() }), None);
                    }
                    Cmd::Heist { conn, msg } => {
                        let Some(user_id) = seats.get(&conn).map(|s| s.user_id.clone()) else { continue };
                        let Some(h) = heist.as_mut() else { continue };
                        let now = born.elapsed().as_secs_f64();
                        let host = room.host() == Some(user_id.as_str());
                        let result = serde_json::from_value::<heist::game::Msg>(msg)
                            .map_err(|_| "Port Run did not understand that")
                            .and_then(|m| h.handle(&user_id, host, m, now));
                        if let Err(message) = result
                            && let Some(seat) = seats.get(&conn)
                        {
                            let _ = seat.out.send(text(&ServerMsg::Error { message }));
                        }
                    }
                }
            }
            _ = ticker.tick() => {
                if let Some(h) = heist.as_mut() {
                    let now = born.elapsed().as_secs_f64();
                    h.tick(now, TICK.as_secs_f64());
                    heist_ticks += 1;
                    for event in std::mem::take(&mut h.events) {
                        if let heist::game::What::Started = event.what {
                            heist_ref = format!("heist-{lobby_id}-{}", (now * 1000.0) as u64);
                            if let Some(r) = server.reports() {
                                r.started(&lobby_id, h.players.keys().cloned().collect());
                            }
                        }
                        if let heist::game::What::Ended { .. } = event.what
                            && let heist::game::Phase::Ended { results } = &h.phase
                            && let Some(r) = server.reports()
                        {
                            r.finished(&lobby_id, &heist_ref, results);
                        }
                        let value = serde_json::to_value(&event).unwrap_or_default();
                        for seat in seats.values() {
                            if view::hears(h, &seat.user_id, event.to) {
                                let _ = seat.out.send(text(&ServerMsg::HeistEvent { event: value.clone() }));
                            }
                        }
                    }
                    if h.world_version != sent_world {
                        sent_world = h.world_version;
                        broadcast(&mut seats, text(&ServerMsg::HeistWorld { world: view::world(h) }), None);
                    }
                    let mut cache: HashMap<String, (Message, Option<Message>)> = HashMap::new();
                    let views = heist_ticks % 3 == 0;
                    seats.retain(|_, seat| {
                        let (p, v) = cache
                            .entry(seat.user_id.clone())
                            .or_insert_with(|| {
                                let p = text(&ServerMsg::HeistPoses { poses: view::poses(h, &seat.user_id) });
                                let v = views.then(|| text(&ServerMsg::Heist { view: view::view(h, &seat.user_id, now) }));
                                (p, v)
                            })
                            .clone();
                        let ok = seat.out.send(p).is_ok();
                        if let Some(v) = v {
                            let _ = seat.out.send(v);
                        }
                        ok
                    });
                }
                if !moved.is_empty() {
                    let cars: Vec<Car> = moved
                        .iter()
                        .filter_map(|id| poses.get(id).map(|pose| Car { id, pose: *pose }))
                        .collect();
                    broadcast(&mut seats, text(&ServerMsg::Poses { cars }), None);
                    moved.clear();
                }
                if let Some(since) = empty_since
                    && since.elapsed() >= IDLE_CLOSE
                {
                    break;
                }
            }
        }
    }
    drop(commands);
    server.inner.rooms.remove_if(&lobby_id, |_, tx| tx.is_closed());
}
