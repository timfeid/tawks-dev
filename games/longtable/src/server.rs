//! The table socket on the party port: one room per lobby, holding the table
//! in memory and writing it through to the store.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::pass::{self, Pass};
use crate::srd;
use crate::store::{self, Store};
use crate::table::{Action, Actor, Table, Turn};

const TICK: Duration = Duration::from_millis(250);
const SAVE_AFTER: Duration = Duration::from_secs(1);
const IDLE_CLOSE: Duration = Duration::from_secs(10 * 60);
const ACTIONS_PER_SECOND: u32 = 12;
const MAX_MESSAGE_BYTES: usize = 32 * 1024;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Hello { token: String },
    Ping { t: f64 },
    Act { action: Action },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg<'a> {
    Welcome {
        user_id: &'a str,
        conditions: Vec<serde_json::Value>,
        attribution: &'a str,
        compatibility: &'a str,
    },
    Table {
        table: crate::table::TableView<'a>,
        rolled: &'a [u64],
    },
    Pong {
        t: f64,
    },
    Error {
        message: &'a str,
    },
}

fn text(msg: &ServerMsg) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

/// Lights up the stage tile of whoever's turn it is.
pub trait TurnSink: Send + Sync {
    fn set_turn(&self, lobby_id: &str, turn: &Turn);
}

enum Cmd {
    Connect {
        conn: u64,
        pass: Pass,
        out: mpsc::UnboundedSender<Message>,
    },
    Disconnect {
        conn: u64,
    },
    Act {
        conn: u64,
        action: Action,
    },
}

struct Inner {
    store: Arc<dyn Store>,
    secret: [u8; 32],
    rooms: DashMap<String, mpsc::UnboundedSender<Cmd>>,
    next_conn: AtomicU64,
    ws_url: String,
    turns: RwLock<Option<Arc<dyn TurnSink>>>,
}

#[derive(Clone)]
pub struct LongtableServer {
    inner: Arc<Inner>,
}

impl LongtableServer {
    pub fn new(store: Arc<dyn Store>, secret: [u8; 32], ws_url: String) -> Self {
        LongtableServer {
            inner: Arc::new(Inner {
                store,
                secret,
                rooms: DashMap::new(),
                next_conn: AtomicU64::new(1),
                ws_url,
                turns: RwLock::new(None),
            }),
        }
    }

    /// Opens the `LONGTABLE_DATABASE_URL` store. Returns None when it is not
    /// set or can't be reached, so the table stays out of the lobby menu and
    /// the games around it carry on.
    pub async fn from_env(party_public_url: &str) -> Option<Self> {
        let Some(url) = std::env::var("LONGTABLE_DATABASE_URL").ok().filter(|u| !u.is_empty()) else {
            info!("LONGTABLE_DATABASE_URL is not set; Longtable is off");
            return None;
        };
        match store::PgStore::connect(&url).await {
            Ok(store) => Some(Self::new(
                Arc::new(store),
                pass::secret_from_env(),
                ws_url_for(party_public_url),
            )),
            Err(error) => {
                tracing::error!(%error, "the Longtable database did not open; Longtable is off");
                None
            }
        }
    }

    pub fn store(&self) -> Arc<dyn Store> {
        self.inner.store.clone()
    }

    pub fn ws_url(&self) -> &str {
        &self.inner.ws_url
    }

    pub fn set_turns(&self, sink: Arc<dyn TurnSink>) {
        *self.inner.turns.write().unwrap() = Some(sink);
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

    pub fn router(&self) -> Router {
        Router::new()
            .route("/party/longtable/ws", get(ws_handler))
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

    fn send_turn(&self, lobby_id: &str, turn: &Turn) {
        if let Some(sink) = self.inner.turns.read().unwrap().clone() {
            sink.set_turn(lobby_id, turn);
        }
    }
}

/// `wss://host/party/ws` becomes `wss://host/party/longtable/ws`.
pub fn ws_url_for(party_public_url: &str) -> String {
    let base = party_public_url.trim_end_matches('/').trim_end_matches("/party/ws");
    format!("{base}/party/longtable/ws")
}

async fn ws_handler(State(server): State<LongtableServer>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| serve_socket(server, socket))
}

async fn serve_socket(server: LongtableServer, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let pass = loop {
        match stream.next().await {
            Some(Ok(Message::Text(raw))) => match serde_json::from_str::<ClientMsg>(&raw) {
                Ok(ClientMsg::Hello { token }) => match pass::verify(&token, &server.inner.secret, pass::now_unix()) {
                    Some(pass) => break pass,
                    None => {
                        let _ = sink
                            .send(text(&ServerMsg::Error {
                                message: "pass_expired",
                            }))
                            .await;
                        return;
                    }
                },
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
    if let Err(mpsc::error::SendError(Cmd::Connect { pass, out, .. })) = room.send(Cmd::Connect {
        conn,
        pass,
        out: out.clone(),
    }) {
        server.inner.rooms.remove_if(&lobby_id, |_, tx| tx.is_closed());
        room = server.room(&lobby_id);
        let _ = room.send(Cmd::Connect { conn, pass, out });
    }

    let writer = tokio::spawn(async move {
        while let Some(message) = outbox.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    let mut window = Instant::now();
    let mut in_window = 0u32;
    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(raw) = message else { continue };
        let msg = match serde_json::from_str::<ClientMsg>(&raw) {
            Ok(msg) => msg,
            Err(_) => {
                let _ = out.send(text(&ServerMsg::Error {
                    message: "The table did not understand that",
                }));
                continue;
            }
        };
        match msg {
            ClientMsg::Ping { t } => {
                let _ = out.send(text(&ServerMsg::Pong { t }));
            }
            ClientMsg::Hello { .. } => {}
            ClientMsg::Act { action } => {
                if window.elapsed() >= Duration::from_secs(1) {
                    window = Instant::now();
                    in_window = 0;
                }
                in_window += 1;
                if in_window > ACTIONS_PER_SECOND {
                    let _ = out.send(text(&ServerMsg::Error {
                        message: "Slow down a little",
                    }));
                    continue;
                }
                if room.send(Cmd::Act { conn, action }).is_err() {
                    break;
                }
            }
        }
    }
    let _ = room.send(Cmd::Disconnect { conn });
    writer.abort();
}

struct Seat {
    pass: Pass,
    out: mpsc::UnboundedSender<Message>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

async fn load(server: &LongtableServer, key: &str) -> Option<Table> {
    for attempt in 0..3 {
        match server.inner.store.load(key).await {
            Ok(Some(state)) => match serde_json::from_value(state) {
                Ok(table) => return Some(table),
                Err(error) => {
                    warn!(%error, key, "a saved table did not read; starting it over");
                    return Some(Table::default());
                }
            },
            Ok(None) => return Some(Table::default()),
            Err(error) => {
                warn!(%error, key, attempt, "the table did not load");
                tokio::time::sleep(Duration::from_millis(300 * (attempt + 1))).await;
            }
        }
    }
    None
}

async fn save(server: &LongtableServer, key: &str, table: &Table) -> bool {
    match serde_json::to_value(table) {
        Ok(state) => match server.inner.store.save(key, &state).await {
            Ok(()) => true,
            Err(error) => {
                warn!(%error, key, "the table did not save");
                false
            }
        },
        Err(error) => {
            warn!(%error, key, "the table did not serialize");
            false
        }
    }
}

fn welcome(user_id: &str) -> Message {
    let conditions = srd::CONDITIONS
        .iter()
        .map(|c| json!({ "name": c.name, "summary": c.summary }))
        .collect();
    text(&ServerMsg::Welcome {
        user_id,
        conditions,
        attribution: srd::ATTRIBUTION,
        compatibility: srd::COMPATIBILITY,
    })
}

fn broadcast(table: &Table, seats: &mut HashMap<u64, Seat>, rolled: &[u64]) {
    let online: Vec<String> = seats.values().map(|s| s.pass.user_id.clone()).collect();
    let is_online = |id: &str| online.iter().any(|o| o == id);
    let mut views: HashMap<String, Message> = HashMap::new();
    seats.retain(|_, seat| {
        let user = seat.pass.user_id.clone();
        let message = views
            .entry(user.clone())
            .or_insert_with(|| {
                text(&ServerMsg::Table {
                    table: table.view_for(&user, &is_online),
                    rolled,
                })
            })
            .clone();
        seat.out.send(message).is_ok()
    });
}

async fn run_room(server: LongtableServer, lobby_id: String, mut commands: mpsc::UnboundedReceiver<Cmd>) {
    let key = store::lobby_key(&lobby_id);
    let Some(mut table) = load(&server, &key).await else {
        server.inner.rooms.remove(&lobby_id);
        commands.close();
        while let Some(cmd) = commands.recv().await {
            if let Cmd::Connect { out, .. } = cmd {
                let _ = out.send(text(&ServerMsg::Error {
                    message: "The table could not load. Try again in a moment.",
                }));
                let _ = out.send(Message::Close(None));
            }
        }
        return;
    };
    let mut seats: HashMap<u64, Seat> = HashMap::new();
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::from_os_rng();
    let mut dirty_at: Option<Instant> = None;
    let mut empty_since = Some(Instant::now());
    let mut last_turn = table.turn();
    if !last_turn.user_ids.is_empty() {
        server.send_turn(&lobby_id, &last_turn);
    }
    let mut ticker = tokio::time::interval(TICK);
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                let mut rolled = Vec::new();
                match command {
                    Cmd::Connect { conn, pass, out } => {
                        let _ = out.send(welcome(&pass.user_id));
                        table.arrive(&pass.user_id, &pass.name, &pass.avatar_url);
                        seats.insert(conn, Seat { pass, out });
                        empty_since = None;
                        dirty_at.get_or_insert_with(Instant::now);
                    }
                    Cmd::Disconnect { conn } => {
                        seats.remove(&conn);
                        if seats.is_empty() {
                            empty_since = Some(Instant::now());
                        }
                    }
                    Cmd::Act { conn, action } => {
                        let Some(seat) = seats.get(&conn) else { continue };
                        let actor = Actor { user_id: &seat.pass.user_id, name: &seat.pass.name, lobby_owner: seat.pass.owner };
                        match table.apply(&actor, action, &mut rng, now_ms()) {
                            Ok(outcome) => {
                                rolled = outcome.rolls;
                                dirty_at.get_or_insert_with(Instant::now);
                            }
                            Err(message) => {
                                let _ = seat.out.send(text(&ServerMsg::Error { message: &message }));
                                continue;
                            }
                        }
                    }
                }
                broadcast(&table, &mut seats, &rolled);
                let turn = table.turn();
                if turn != last_turn {
                    server.send_turn(&lobby_id, &turn);
                    last_turn = turn;
                }
            }
            _ = ticker.tick() => {
                if dirty_at.is_some_and(|at| at.elapsed() >= SAVE_AFTER) && save(&server, &key, &table).await {
                    dirty_at = None;
                }
                if dirty_at.is_none() && empty_since.is_some_and(|at| at.elapsed() >= IDLE_CLOSE) {
                    break;
                }
            }
        }
    }
    server.inner.rooms.remove(&lobby_id);
    commands.close();
    while let Some(cmd) = commands.recv().await {
        if let Cmd::Connect { out, .. } = cmd {
            let _ = out.send(Message::Close(None));
        }
    }
    if dirty_at.is_some() {
        save(&server, &key, &table).await;
    }
    if !last_turn.user_ids.is_empty() {
        server.send_turn(&lobby_id, &Turn::default());
    }
}
