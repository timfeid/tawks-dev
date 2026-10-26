//! The WebSocket side of party rooms. One task per room ticks the simulation
//! at 60 Hz and streams snapshots at 20 Hz to everyone connected.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use rand::Rng;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::{error, info, warn};

use crate::auth::{self, Claims};
use crate::protocol::{ClientMsg, ServerMsg};
use crate::room::{Phase, Recording, Room, SNAPSHOT_EVERY_TICKS, TICK_DT, TICK_HZ};
use crate::roster::ROSTER;

const IDLE_ROOM_TICKS: u64 = TICK_HZ as u64 * 120;

/// Films a room from somewhere else: opens it as a spectator with the pass in
/// `watch` and records what it sees into the lobby's channel files.
pub trait SessionRecorder: Send + Sync {
    fn start(&self, match_id: &str, title: &str, recorded_by: &str, watch: serde_json::Value) -> Result<(), String>;
    fn stop(&self, match_id: &str);
}

#[derive(Clone, Debug)]
pub enum RecorderStatus {
    Live,
    Ended(Option<String>),
}

#[derive(Clone, Debug)]
pub struct PartyConfig {
    pub bind: SocketAddr,
    pub public_url: String,
}

impl PartyConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let bind = std::env::var("PARTY_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8095".into())
            .parse()?;
        let public_url = std::env::var("PARTY_PUBLIC_URL")
            .unwrap_or_else(|_| "ws://localhost:8095/party/ws".into());
        Ok(PartyConfig { bind, public_url })
    }
}

enum RoomCmd {
    Connect {
        conn: u64,
        claims: Claims,
        out: mpsc::UnboundedSender<Message>,
    },
    Disconnect {
        conn: u64,
    },
    Client {
        conn: u64,
        msg: ClientMsg,
    },
    Recorder(RecorderStatus),
}

struct Inner {
    secret: [u8; 32],
    public_url: String,
    rooms: DashMap<String, mpsc::UnboundedSender<RoomCmd>>,
    next_conn: AtomicU64,
    recorder: std::sync::RwLock<Option<Arc<dyn SessionRecorder>>>,
}

#[derive(Clone)]
pub struct PartyServer {
    inner: Arc<Inner>,
}

impl PartyServer {
    pub async fn start(config: PartyConfig) -> anyhow::Result<Self> {
        let secret = signing_secret("PARTY_SECRET");
        let server = PartyServer {
            inner: Arc::new(Inner {
                secret,
                public_url: config.public_url.clone(),
                rooms: DashMap::new(),
                next_conn: AtomicU64::new(1),
                recorder: std::sync::RwLock::new(None),
            }),
        };
        let app = Router::new()
            .route("/party/ws", get(ws_handler))
            .route("/party/health", get(|| async { "ok" }))
            .with_state(server.clone());
        let listener = tokio::net::TcpListener::bind(config.bind).await?;
        info!(bind = %config.bind, public_url = %config.public_url, "party server listening");
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!(error = %e, "party server stopped");
            }
        });
        Ok(server)
    }

    pub fn public_url(&self) -> &str {
        &self.inner.public_url
    }

    pub fn issue_token(
        &self,
        match_id: &str,
        user_id: &str,
        name: &str,
        avatar_url: &str,
    ) -> String {
        self.issue_token_for(match_id, user_id, name, avatar_url, None, false)
    }

    pub fn issue_token_for(
        &self,
        match_id: &str,
        user_id: &str,
        name: &str,
        avatar_url: &str,
        game: Option<&str>,
        guest: bool,
    ) -> String {
        let claims = Claims {
            match_id: match_id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            avatar_url: avatar_url.to_string(),
            exp: auth::now_unix() + auth::TOKEN_TTL_SECS,
            spectator: false,
            guest,
            drives: Vec::new(),
            game: game.map(str::to_string),
        };
        auth::sign(&claims, &self.inner.secret)
    }

    pub fn set_recorder(&self, recorder: Arc<dyn SessionRecorder>) {
        *self.inner.recorder.write().unwrap() = Some(recorder);
    }

    fn recorder(&self) -> Option<Arc<dyn SessionRecorder>> {
        self.inner.recorder.read().unwrap().clone()
    }

    /// Passes word from the recorder to the room it is filming, if still open.
    pub fn recorder_status(&self, match_id: &str, status: RecorderStatus) {
        if let Some(room) = self.inner.rooms.get(match_id) {
            let _ = room.send(RoomCmd::Recorder(status));
        }
    }

    /// A watch-only pass for the recorder's browser.
    fn recorder_pass(&self, match_id: &str) -> String {
        let claims = Claims {
            match_id: match_id.to_string(),
            user_id: format!("rec-{match_id}"),
            name: "Recorder".into(),
            avatar_url: String::new(),
            exp: auth::now_unix() + auth::TOKEN_TTL_SECS,
            spectator: true,
            guest: false,
            drives: Vec::new(),
            game: None,
        };
        auth::sign(&claims, &self.inner.secret)
    }

    fn room(&self, match_id: &str) -> mpsc::UnboundedSender<RoomCmd> {
        let mut entry = self
            .inner
            .rooms
            .entry(match_id.to_string())
            .or_insert_with(|| self.spawn_room(match_id));
        if entry.is_closed() {
            *entry = self.spawn_room(match_id);
        }
        entry.clone()
    }

    fn spawn_room(&self, match_id: &str) -> mpsc::UnboundedSender<RoomCmd> {
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(run_room(self.clone(), match_id.to_string(), rx));
        tx
    }
}

async fn ws_handler(ws: WebSocketUpgrade, State(server): State<PartyServer>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(server, socket))
}

fn encode(msg: &ServerMsg) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

async fn handle_socket(server: PartyServer, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let hello = tokio::time::timeout(Duration::from_secs(10), stream.next()).await;
    let token = match hello {
        Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str::<ClientMsg>(&text) {
            Ok(ClientMsg::Hello { token }) => token,
            _ => return,
        },
        _ => return,
    };
    let Some(claims) = auth::verify(&token, &server.inner.secret, auth::now_unix()) else {
        let _ = sink
            .send(encode(&ServerMsg::Error {
                message: "Your party pass expired. Reconnecting.".into(),
                code: Some("auth"),
            }))
            .await;
        let _ = sink.close().await;
        return;
    };

    let conn = server.inner.next_conn.fetch_add(1, Ordering::Relaxed);
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Message>();
    let writer = tokio::spawn(async move {
        while let Some(message) = out_rx.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    let match_id = claims.match_id.clone();
    let mut room = server.room(&match_id);
    let connect = |claims: Claims, out: mpsc::UnboundedSender<Message>| RoomCmd::Connect {
        conn,
        claims,
        out,
    };
    if room.send(connect(claims.clone(), out_tx.clone())).is_err() {
        // The room shut down between lookup and send; start a fresh one.
        room = server.room(&match_id);
        let _ = room.send(connect(claims, out_tx.clone()));
    }
    drop(out_tx);

    while let Some(Ok(message)) = stream.next().await {
        match message {
            Message::Text(text) => {
                if let Ok(msg) = serde_json::from_str::<ClientMsg>(&text) {
                    if room.send(RoomCmd::Client { conn, msg }).is_err() {
                        break;
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
    let _ = room.send(RoomCmd::Disconnect { conn });
    // The room drops its sender for this connection, which ends the writer.
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        writer.abort();
    });
}

struct Conn {
    user_id: String,
    name: String,
    avatar_url: String,
    out: mpsc::UnboundedSender<Message>,
    seated: bool,
    spectator: bool,
    guest: bool,
}

fn send_to(conn: &Conn, msg: &ServerMsg) {
    let _ = conn.out.send(encode(msg));
}

fn broadcast_room(room: &Room, conns: &HashMap<u64, Conn>) {
    let view = room.view();
    let seated = encode(&ServerMsg::Room {
        room: view.clone(),
        spectating: false,
    });
    let watching = encode(&ServerMsg::Room {
        room: view,
        spectating: true,
    });
    for conn in conns.values() {
        let _ = conn.out.send(if conn.seated {
            seated.clone()
        } else {
            watching.clone()
        });
    }
}

fn snap_message(room: &Room, events: Vec<serde_json::Value>) -> Option<Message> {
    room.snapshot().map(|game| {
        encode(&ServerMsg::Snap {
            tick: room.tick,
            game,
            events,
        })
    })
}

async fn run_room(server: PartyServer, match_id: String, mut rx: mpsc::UnboundedReceiver<RoomCmd>) {
    info!(%match_id, "party room opened");
    let mut room = Room::new(match_id.clone(), rand::rng().random());
    let mut conns: HashMap<u64, Conn> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs_f32(TICK_DT));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut idle_ticks = 0u64;
    let mut events = Vec::new();

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    RoomCmd::Connect { conn, claims, out } => {
                        if let Some(game) = claims.game.as_deref() {
                            room.prefer_game(game);
                        }
                        let seated = !claims.spectator && room.connect(&claims.user_id, &claims.name, &claims.avatar_url).is_ok();
                        let entry = Conn { user_id: claims.user_id, name: claims.name, avatar_url: claims.avatar_url, out, seated, spectator: claims.spectator, guest: claims.guest };
                        send_to(&entry, &ServerMsg::Welcome {
                            user_id: &entry.user_id,
                            match_id: &match_id,
                            roster: ROSTER,
                            tick_hz: TICK_HZ,
                            snapshot_hz: TICK_HZ / SNAPSHOT_EVERY_TICKS as u32,
                            guest: entry.guest,
                        });
                        send_to(&entry, &ServerMsg::Room { room: room.view(), spectating: !entry.seated });
                        if let Some(snap) = snap_message(&room, Vec::new()) {
                            let _ = entry.out.send(snap);
                        }
                        if let Some((game, view)) = room.table_view(room.table_seat_of(&entry.user_id)) {
                            send_to(&entry, &ServerMsg::Table { game, view });
                        }
                        conns.insert(conn, entry);
                    }
                    RoomCmd::Disconnect { conn } => {
                        if let Some(gone) = conns.remove(&conn) {
                            if !gone.spectator && !conns.values().any(|c| c.user_id == gone.user_id) {
                                room.disconnect(&gone.user_id);
                            }
                        }
                        if room.recording().is_some() && !conns.values().any(|c| !c.spectator) {
                            stop_recording(&server, &match_id, &mut room);
                        }
                    }
                    RoomCmd::Client { conn, msg } => {
                        let Some(entry) = conns.get(&conn) else { continue };
                        let user_id = entry.user_id.clone();
                        if entry.spectator && !matches!(msg, ClientMsg::Ping { .. }) {
                            continue;
                        }
                        let result = match msg {
                            ClientMsg::Input { x, z, yaw, pitch, jump, crouch } => {
                                room.set_input(&user_id, crate::bout::Stick { x, z }, yaw, pitch, jump, crouch);
                                Ok(())
                            }
                            ClientMsg::Fire { yaw, pitch } => {
                                room.queue_fire(&user_id, yaw, pitch);
                                Ok(())
                            }
                            ClientMsg::ChooseGame { key } => room.choose_game(&user_id, &key),
                            ClientMsg::Vote { key } => room.vote(&user_id, &key),
                            ClientMsg::ChooseSkill { level } => room.choose_skill(&user_id, &level),
                            ClientMsg::Pick { character } => room.pick(&user_id, &character),
                            ClientMsg::Start => room.start(&user_id),
                            ClientMsg::StartParty => room.start_party(&user_id),
                            ClientMsg::UseItem { item, target } => room.use_item(&user_id, item, target),
                            ClientMsg::PassItems => room.pass_items(&user_id),
                            ClientMsg::Roll => room.roll(&user_id),
                            ClientMsg::ChooseBranch { node } => room.choose_branch(&user_id, node),
                            ClientMsg::EndTurn => room.end_turn(&user_id),
                            ClientMsg::AddBot => room.add_bot(&user_id),
                            ClientMsg::RemoveBot { seat } => room.remove_bot(&user_id, seat),
                            ClientMsg::Table { action } => room.table_act(&user_id, action),
                            ClientMsg::Record { on } => {
                                if let Err(message) = record(&server, &match_id, &mut room, entry, on) {
                                    send_to(entry, &ServerMsg::Error { message, code: None });
                                }
                                Ok(())
                            }
                            ClientMsg::Ping { t } => {
                                send_to(entry, &ServerMsg::Pong { t });
                                Ok(())
                            }
                            ClientMsg::Hello { .. } => Ok(()),
                        };
                        if let Err(e) = result {
                            send_to(entry, &ServerMsg::Error { message: e.to_string(), code: None });
                        }
                    }
                    RoomCmd::Recorder(status) => match status {
                        RecorderStatus::Live => {
                            if let Some(recording) = room.recording().cloned() {
                                room.set_recording(Some(Recording { live: true, ..recording }));
                            }
                        }
                        RecorderStatus::Ended(message) => {
                            let was_recording = room.recording().is_some();
                            room.set_recording(None);
                            if let (true, Some(message)) = (was_recording, message) {
                                let notice = ServerMsg::Error { message: format!("Recording stopped: {message}"), code: None };
                                for entry in conns.values() {
                                    send_to(entry, &notice);
                                }
                            }
                        }
                    },
                }
            }
            _ = interval.tick() => {
                events.extend(room.step());

                // Anyone watching gets a seat as soon as the lobby has room.
                if room.phase == Phase::Lobby && room.tick % TICK_HZ as u64 == 0 {
                    for entry in conns.values_mut().filter(|c| !c.seated && !c.spectator) {
                        entry.seated = room.connect(&entry.user_id, &entry.name, &entry.avatar_url).is_ok();
                    }
                }

                let dirty = room.take_dirty();
                if dirty || room.tick % TICK_HZ as u64 == 0 {
                    broadcast_room(&room, &conns);
                }
                if room.table.is_some() && (dirty || room.tick % 30 == 0) {
                    for entry in conns.values() {
                        if let Some((game, view)) = room.table_view(room.table_seat_of(&entry.user_id)) {
                            send_to(entry, &ServerMsg::Table { game, view });
                        }
                    }
                }
                let live = room.phase != Phase::Lobby;
                if (live && room.tick % SNAPSHOT_EVERY_TICKS == 0) || (!live && (dirty || room.tick % 30 == 0)) {
                    if let Some(snap) = snap_message(&room, std::mem::take(&mut events)) {
                        for entry in conns.values() {
                            let _ = entry.out.send(snap.clone());
                        }
                    }
                }

                if conns.values().all(|c| c.spectator) {
                    idle_ticks += 1;
                    if idle_ticks > IDLE_ROOM_TICKS {
                        break;
                    }
                } else {
                    idle_ticks = 0;
                }
            }
        }
    }
    if room.recording().is_some() {
        stop_recording(&server, &match_id, &mut room);
    }
    drop(rx);
    server
        .inner
        .rooms
        .remove_if(&match_id, |_, tx| tx.is_closed());
    if conns.is_empty() {
        info!(%match_id, "party room closed after sitting empty");
    } else {
        warn!(%match_id, "party room closed with connections open");
    }
}

fn record(server: &PartyServer, match_id: &str, room: &mut Room, by: &Conn, on: bool) -> Result<(), String> {
    if !on {
        stop_recording(server, match_id, room);
        return Ok(());
    }
    if by.guest {
        return Err("Sign in to record.".into());
    }
    if !by.seated {
        return Err("Take a seat to record.".into());
    }
    if room.recording().is_some() {
        return Ok(());
    }
    let recorder = server.recorder().ok_or("Recording is not available here.")?;
    let watch = serde_json::json!({
        "ws_url": server.public_url(),
        "token": server.recorder_pass(match_id),
        "match_id": match_id,
    });
    recorder.start(match_id, room.view().minigame_name, &by.user_id, watch)?;
    room.set_recording(Some(Recording {
        by: by.name.clone(),
        by_user_id: by.user_id.clone(),
        since: auth::now_unix(),
        live: false,
    }));
    Ok(())
}

fn stop_recording(server: &PartyServer, match_id: &str, room: &mut Room) {
    if room.recording().is_some() {
        if let Some(recorder) = server.recorder() {
            recorder.stop(match_id);
        }
        room.set_recording(None);
    }
}

/// The key passes are signed with. Set `PARTY_SECRET` so passes stay valid across a
/// restart or a failover to another copy; without it each process makes its own.
fn signing_secret(var: &str) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    match std::env::var(var) {
        Ok(value) if !value.is_empty() => Sha256::digest(value.as_bytes()).into(),
        _ => {
            let mut secret = [0u8; 32];
            rand::rng().fill(&mut secret);
            secret
        }
    }
}
