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

/// Where ratings of Tawkathon bouts go: filed for the team and rewarded.
/// It returns at once and does its work in the background.
pub type FeedbackFuture=std::pin::Pin<Box<dyn std::future::Future<Output=Result<crate::feedback::FeedbackReceipt,String>>+Send>>;
pub trait FeedbackSink: Send + Sync {
    fn submit(&self,feedback:crate::room::BoutFeedback)->FeedbackFuture;
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
        native: bool,
        wardrobe: Option<crate::roster::Wardrobe>,
        out: mpsc::UnboundedSender<Message>,
    },
    Disconnect {
        conn: u64,
    },
    Client {
        conn: u64,
        msg: ClientMsg,
    },
    FeedbackStored{conn:u64,feedback:crate::room::BoutFeedback,result:Result<crate::feedback::FeedbackReceipt,String>},
    Recorder(RecorderStatus),
    CostumeLoaded { user_id: String, costume: crate::roster::Costume },
}

struct Inner {
    wardrobe_web: std::sync::RwLock<Option<Arc<tawks_bot::web_client::WebClient>>>,
    milestone_web: std::sync::RwLock<Option<Arc<tawks_bot::web_client::WebClient>>>,
    secret: [u8; 32],
    native_passes: auth::NativePasses,
    public_url: String,
    rooms: DashMap<String, mpsc::UnboundedSender<RoomCmd>>,
    next_conn: AtomicU64,
    recorder: std::sync::RwLock<Option<Arc<dyn SessionRecorder>>>,
    feedback: std::sync::RwLock<Option<Arc<dyn FeedbackSink>>>,
}

#[derive(Clone)]
pub struct PartyServer {
    inner: Arc<Inner>,
}

impl PartyServer {
    pub async fn start(config: PartyConfig) -> anyhow::Result<Self> {
        Self::start_with(config, Router::new()).await
    }

    /// Starts the party socket with more routes served on the same port.
    pub async fn start_with(config: PartyConfig, extra: Router) -> anyhow::Result<Self> {
        let secret = signing_secret("PARTY_SECRET");
        let server = PartyServer {
            inner: Arc::new(Inner {
                wardrobe_web: std::sync::RwLock::new(None),
                milestone_web: std::sync::RwLock::new(None),
                secret,
                native_passes: auth::NativePasses::default(),
                public_url: config.public_url.clone(),
                rooms: DashMap::new(),
                next_conn: AtomicU64::new(1),
                recorder: std::sync::RwLock::new(None),
                feedback: std::sync::RwLock::new(None),
            }),
        };
        let app = Router::new()
            .route("/party/ws", get(ws_handler))
            .route("/party/health", get(|| async { "ok" }))
            .route("/party/native-pass", axum::routing::post(native_pass_handler).layer(axum::extract::DefaultBodyLimit::max(32 * 1024)))
            .with_state(server.clone())
            .merge(extra);
        let listener = tokio::net::TcpListener::bind(config.bind).await?;
        info!(bind = %config.bind, public_url = %config.public_url, "party server listening");
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!(error = %e, "party server stopped");
            }
        });
        Ok(server)
    }

    pub fn set_wardrobe_client(&self, web: Arc<tawks_bot::web_client::WebClient>) { *self.inner.wardrobe_web.write().unwrap()=Some(web); }
    pub fn set_milestone_client(&self, web: Arc<tawks_bot::web_client::WebClient>) {
        *self.inner.milestone_web.write().unwrap() = Some(web);
    }

    fn report_milestones(&self, events: Vec<serde_json::Value>) {
        if events.is_empty() {
            return;
        }
        let Some(web) = self.inner.milestone_web.read().unwrap().clone() else { return };
        for event in events {
            let web = web.clone();
            tokio::spawn(async move {
                if let Err(error) = web.mutate::<_, ()>("milestone_event", &event).await {
                    warn!(%error, "a Tawkathon milestone event did not send");
                }
            });
        }
    }
    fn pet_hello(&self, user: &str) {
        let Some(web) = self.inner.wardrobe_web.read().ok().and_then(|web| web.clone()) else { return };
        let user = user.to_string();
        tokio::spawn(async move {
            if let Err(error) = web.mutate::<_, bool>("party_pet_hello_for", &user).await {
                warn!(%error, "a pet hello did not send");
            }
        });
    }
    async fn wardrobe(&self, user: &str) -> Option<crate::roster::Wardrobe> {
        let web=self.inner.wardrobe_web.read().ok()?.clone()?;
        tokio::time::timeout(Duration::from_secs(3),web.query("party_wardrobe_for",&user)).await.ok()?.ok()
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
        self.issue_token_for(match_id, user_id, name, avatar_url, None, false, false)
    }

    pub fn issue_token_for(
        &self,
        match_id: &str,
        user_id: &str,
        name: &str,
        avatar_url: &str,
        game: Option<&str>,
        guest: bool,
        owner: bool,
    ) -> String {
        let claims = Claims {
            match_id: match_id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            avatar_url: avatar_url.to_string(),
            exp: auth::now_unix() + auth::TOKEN_TTL_SECS,
            spectator: guest,
            guest,
            drives: Vec::new(),
            game: game.map(str::to_string),
            owner,
        };
        auth::sign(&claims, &self.inner.secret)
    }

    pub fn set_feedback(&self, sink: Arc<dyn FeedbackSink>) {
        *self.inner.feedback.write().unwrap() = Some(sink);
    }

    fn feedback_sink(&self) -> Option<Arc<dyn FeedbackSink>> {
        self.inner.feedback.read().unwrap().clone()
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
            owner: false,
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

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct NativePassRequest { token: String, user_id: String, match_id: String }

async fn native_pass_handler(State(server): State<PartyServer>, axum::Json(request): axum::Json<NativePassRequest>) -> axum::response::Response {
    let now = auth::now_unix();
    let token = server.inner.native_passes.exchange(&request.token, &request.user_id, &request.match_id, &server.inner.secret, now);
    match token {
        Some(token) => (
            [(axum::http::header::CACHE_CONTROL, "no-store")],
            axum::Json(serde_json::json!({"token":token,"user_id":request.user_id,"match_id":request.match_id,"expires_at":now+auth::NATIVE_PASS_TTL_SECS})),
        ).into_response(),
        None => (axum::http::StatusCode::UNAUTHORIZED, "Rejoin the game before opening Tawkathon.").into_response(),
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
    let now = auth::now_unix();
    let native = token.starts_with("native-");
    let claims = if native {
        server.inner.native_passes.consume(&token, now)
    } else {
        auth::verify(&token, &server.inner.secret, now)
    };
    let Some(claims) = claims else {
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
            // About 20s of snapshots behind: the player has stalled, so close rather than queue without limit.
            if out_rx.len() > 600 || sink.send(message).await.is_err() {
                break;
            }
        }
        let _ = sink.close().await;
    });

    let wardrobe = if !claims.guest && !claims.spectator { server.wardrobe(&claims.user_id).await } else {None};
    let match_id = claims.match_id.clone();
    let mut room = server.room(&match_id);
    let connect = |claims: Claims, out: mpsc::UnboundedSender<Message>| RoomCmd::Connect {
        conn,
        claims,
        native,
        wardrobe: wardrobe.clone(),
        out,
    };
    if room.send(connect(claims.clone(), out_tx.clone())).is_err() {
        // The room shut down between lookup and send; start a fresh one.
        room = server.room(&match_id);
        let _ = room.send(connect(claims, out_tx.clone()));
    }
    drop(out_tx);

    // Clients ping every 2s; a background tab can stretch that to a minute, so 90s of silence is a half-open socket.
    while let Ok(Some(Ok(message))) = tokio::time::timeout(Duration::from_secs(90), stream.next()).await {
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
    // Each seat sees its own traps and nobody else's, so the view is built
    // per seat; spectators and the unseated share one with none of them.
    let mut by_seat: HashMap<Option<u8>, Message> = HashMap::new();
    for conn in conns.values() {
        let seat = if conn.seated { room.seat_of(&conn.user_id) } else { None };
        let message = by_seat.entry(seat).or_insert_with(|| {
            encode(&ServerMsg::Room {
                room: room.view_for(seat),
                spectating: !conn.seated,
            })
        });
        let _ = conn.out.send(message.clone());
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

/// Sends everyone the bout. A bout that hides something gets each seat its
/// own view, looked up by the user's seat even on a spectator socket so a
/// player cannot watch their own game from the stands.
fn send_snaps(room: &Room, conns: &HashMap<u64, Conn>, events: Vec<serde_json::Value>) {
    let mut shared: Option<Option<Message>> = None;
    let mut private: HashMap<Option<u8>, Message> = HashMap::new();
    for conn in conns.values() {
        let seat = room.seat_of(&conn.user_id);
        let message = match room.private_snapshot(seat) {
            Some(game) => private
                .entry(seat)
                .or_insert_with(|| {
                    encode(&ServerMsg::Snap {
                        tick: room.tick,
                        game,
                        events: events.clone(),
                    })
                })
                .clone(),
            None => match shared.get_or_insert_with(|| snap_message(room, events.clone())) {
                Some(message) => message.clone(),
                None => continue,
            },
        };
        let _ = conn.out.send(message);
    }
}

async fn run_room(server: PartyServer, match_id: String, mut rx: mpsc::UnboundedReceiver<RoomCmd>) {
    info!(%match_id, "party room opened");
    let mut room = Room::new(match_id.clone(), rand::rng().random());
    room.dev_tools = dev_tools();
    let mut conns: HashMap<u64, Conn> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs_f32(TICK_DT));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut idle_ticks = 0u64;
    let mut events = Vec::new();
    let mut pending_feedback=std::collections::HashSet::new();
    let mut milestones = crate::milestones::MilestoneTracker::default();
    // Tawks counts a pet's hello once a day; once per room is enough to ask.
    let mut greeted = std::collections::HashSet::new();

    loop {
        tokio::select! {
            cmd = rx.recv() => {
                let Some(cmd) = cmd else { break };
                match cmd {
                    RoomCmd::FeedbackStored{conn,feedback,result}=>{
                        pending_feedback.remove(&(feedback.party_id.clone(),feedback.user_id.clone(),feedback.bout.number,!feedback.note.is_empty()));
                        let (saved,rewarded,message)=match result{
                            Ok(receipt)=>{room.commit_feedback(&feedback);(true,receipt.rewarded,None)},
                            Err(error)=>{warn!(%error,party=%feedback.party_id,"saving bout feedback failed");(false,false,Some("Your feedback wasn't saved. Please try again.".to_string()))}
                        };
                        if let Some(entry)=conns.get(&conn){send_to(entry,&ServerMsg::FeedbackResult{number:feedback.bout.number,with_note:!feedback.note.is_empty(),saved,rewarded,message});}
                    }
                    RoomCmd::CostumeLoaded { user_id, costume } => { room.apply_costume(&user_id,costume); }
                    RoomCmd::Connect { conn, claims, native, wardrobe, out } => {
                        if native && !room.native_compatible() {
                            let _ = out.send(encode(&ServerMsg::Error {message:"This match is using a browser-only game or board. Rejoin from native Tawkathon after it finishes.".into(),code:Some("unsupported")}));
                            continue;
                        }
                        if let Some(game) = claims.game.as_deref().filter(|_| !claims.spectator) {
                            room.prefer_game(game);
                        }
                        let seated = !claims.spectator && room.connect(&claims.user_id, &claims.name, &claims.avatar_url).is_ok();
                        milestones.note_player(&claims.user_id, claims.guest);
                        if seated && native { room.enable_native_mode(); }
                        if seated { if let Some(wardrobe)=wardrobe {room.apply_costume(&claims.user_id,wardrobe.costume);} }
                        if seated && claims.owner {
                            room.set_owner(&claims.user_id);
                        }
                        let entry = Conn { user_id: claims.user_id, name: claims.name, avatar_url: claims.avatar_url, out, seated, spectator: claims.spectator, guest: claims.guest };
                        send_to(&entry, &ServerMsg::Welcome {
                            user_id: &entry.user_id,
                            match_id: &match_id,
                            roster: ROSTER,
                            tick_hz: TICK_HZ,
                            snapshot_hz: TICK_HZ / SNAPSHOT_EVERY_TICKS as u32,
                            guest: entry.guest,
                        });
                        let seat = if entry.seated { room.seat_of(&entry.user_id) } else { None };
                        send_to(&entry, &ServerMsg::Room { room: room.view_for(seat), spectating: !entry.seated });
                        if let Some(game) = room.snapshot_for(room.seat_of(&entry.user_id)) {
                            let _ = entry.out.send(encode(&ServerMsg::Snap { tick: room.tick, game, events: Vec::new() }));
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
                        if room.board_paused()&&!msg.allowed_while_candy_paused(){continue;}
                        let result = match msg {
                            ClientMsg::Face { yaw } => room.face(&user_id, yaw),
                            ClientMsg::PositionInput { x, z } => room.position_input(&user_id, x, z),
                            ClientMsg::ActivateItem => room.activate_item(&user_id),
                            ClientMsg::CancelItem => room.cancel_item(&user_id),
                            ClientMsg::DriveInput {throttle,steer,horn} => room.drive_input(&user_id,throttle,steer,horn),
                            ClientMsg::RaidInput {x,z,yaw} => room.raid_input(&user_id,x,z,yaw),
                            ClientMsg::RaidSlash => room.raid_slash(&user_id),
                            ClientMsg::CrawlerInput{x,z} => room.crawler_input(&user_id,x,z),
                            ClientMsg::RaidFire{on,yaw} => room.raid_fire(&user_id,on,yaw),
                            ClientMsg::BuyCandy => room.buy_candy(&user_id),
                            ClientMsg::JoinCandyVisit => room.join_candy_visit(&user_id),
                            ClientMsg::CandyVisitInput{x,z} => room.candy_visit_input(&user_id,x,z),
                            ClientMsg::WalkVacuum {yaw,pitch,on,blow} => room.aim_walk_vacuum_with(&user_id,yaw,pitch,on,blow),
                            ClientMsg::RefreshCostume => {
                                if room.phase == crate::room::Phase::Lobby && !entry.guest {
                                    let server=server.clone(); let sender=server.room(&match_id); let user_id=user_id.clone();
                                    tokio::spawn(async move {if let Some(wardrobe)=server.wardrobe(&user_id).await {let _=sender.send(RoomCmd::CostumeLoaded{user_id,costume:wardrobe.costume});}});
                                }
                                Ok(())
                            },
                            ClientMsg::Input { x, z, yaw, pitch, jump, crouch } => {
                                room.set_input(&user_id, crate::bout::Stick { x, z }, yaw, pitch, jump, crouch);
                                Ok(())
                            }
                            ClientMsg::Fire { yaw, pitch } => {
                                room.queue_fire(&user_id, yaw, pitch);
                                Ok(())
                            }
                            ClientMsg::Press { hand } => {
                                room.queue_press(&user_id, hand);
                                Ok(())
                            }
                            ClientMsg::ChooseGame { key } => room.choose_game(&user_id, &key),
                            ClientMsg::Vote { key } => room.vote(&user_id, &key),
                            ClientMsg::Feedback { rating, note } => {
                                if entry.guest {Ok(())}else{
                                    room.prepare_feedback(&user_id,rating,&note).map(|feedback|{
                                        let key=(feedback.party_id.clone(),feedback.user_id.clone(),feedback.bout.number,!feedback.note.is_empty());
                                        if pending_feedback.insert(key.clone()){
                                            if let Some(sink)=server.feedback_sink(){
                                                let sender=server.room(&match_id);
                                                tokio::spawn(async move{let result=sink.submit(feedback.clone()).await;let _=sender.send(RoomCmd::FeedbackStored{conn,feedback,result});});
                                            }else{
                                                pending_feedback.remove(&key);
                                                send_to(entry,&ServerMsg::FeedbackResult{number:feedback.bout.number,with_note:!feedback.note.is_empty(),saved:false,rewarded:false,message:Some("Your feedback wasn't saved. The feedback service is unavailable; please try again.".into())});
                                            }
                                        }
                                    })
                                }
                            }
                            ClientMsg::ChooseSkill { level } => room.choose_skill(&user_id, &level),
                            ClientMsg::ChooseMap { map } => room.choose_map(&user_id, map),
                            ClientMsg::ChooseMode { party } => room.choose_mode(&user_id, party),
                            ClientMsg::ChooseRounds { rounds } => room.choose_rounds(&user_id,rounds),
                            ClientMsg::ChooseCity{police,jobs,host_police}=>room.choose_city(&user_id,police,jobs,host_police),
                            ClientMsg::StartCity{watch}=>room.start_city(&user_id,watch),
                            ClientMsg::CityAction{held,suction}=>room.city_action(&user_id,held,suction),
                            ClientMsg::CityTool{tool}=>room.city_tool(&user_id,tool),
                            ClientMsg::CityAutomate{on}=>room.city_automate(&user_id,on),
                            ClientMsg::CityReady=>room.city_ready(&user_id),
                            ClientMsg::CityVehicle=>room.city_vehicle(&user_id),
                            ClientMsg::CityResetCar=>room.city_reset_car(&user_id),
                            ClientMsg::CityTrace{x,y}=>room.city_trace(&user_id,x,y),
                            ClientMsg::CityStruggle=>room.city_struggle(&user_id),
                            ClientMsg::CityItem{act,yaw}=>room.city_item(&user_id,act,yaw),
                            ClientMsg::CityDodge{yaw}=>room.city_dodge(&user_id,yaw),
                            ClientMsg::CityTakedown{yaw}=>room.city_takedown(&user_id,yaw),
                            ClientMsg::CityCallout{yaw}=>room.city_callout(&user_id,yaw),
                            ClientMsg::CityClarity{held}=>room.city_clarity(&user_id,held),
                            ClientMsg::CityCheck{input}=>room.city_check(&user_id,input),
                            ClientMsg::CitySlip{input}=>room.city_slip(&user_id,input),
                            ClientMsg::ChooseMature { on } => room.choose_mature(&user_id, on),
                            ClientMsg::ChoosePairs { on } => room.choose_pairs(&user_id, on),
                            ClientMsg::Ready => room.ready(&user_id),
                            ClientMsg::Pick { character } => room.pick(&user_id, &character),
                            ClientMsg::Start => room.start(&user_id),
                            ClientMsg::StartParty => room.start_party(&user_id),
                            ClientMsg::UseItem {
                                item,
                                target,
                                number,
                            } => room.use_item(&user_id, item, target, number),
                            ClientMsg::PassItems => room.pass_items(&user_id),
                            ClientMsg::Roll => room.roll(&user_id),
                            ClientMsg::Shop { item, upgrade } => room.shop(&user_id, item, upgrade),
                            ClientMsg::FireCannon { power, stop } => room.fire(&user_id, power, stop),
                            ClientMsg::Lob { yaw, pitch, power } => room.lob(&user_id, yaw, pitch, power),
                            ClientMsg::Swing { yaw, stop } => room.swing(&user_id, yaw, stop),
                            ClientMsg::Check { stop } => room.check(&user_id, stop),
                            ClientMsg::Suck => room.suck(&user_id),
                            ClientMsg::Vacuum { yaw, on, blow } => room.aim_vacuum(&user_id, yaw, on, blow),
                            ClientMsg::StowVacuum => room.stow_vacuum(&user_id),
                            ClientMsg::Brace { on } => room.brace(&user_id, on),
                            ClientMsg::ThrowCrate => room.throw_crate(&user_id),
                            ClientMsg::ThrowSnowball => room.throw_snowball(&user_id),
                            ClientMsg::StartSandbox { costume } => {
                                match room.check_start_sandbox(&user_id) {
                                    Err(error) => Err(error),
                                    Ok(()) => if let Some(costume) = costume {
                                        let imported = costume.model_id.as_deref().is_some_and(|id| !crate::roster::cosmetics().iter().any(|item| item.slot == "model" && item.id == id));
                                        let wardrobe = if imported && !entry.guest { server.wardrobe(&user_id).await } else { None };
                                        room.start_sandbox_with_preview(&user_id, costume, wardrobe.as_ref())
                                    } else { room.start_sandbox(&user_id) },
                                }
                            },
                            ClientMsg::Dev { tool } => room.dev(&user_id, tool),
                            ClientMsg::StartCitySandbox { police } => room.start_city_sandbox(&user_id, police),
                            ClientMsg::Bet { coins } => room.bet(&user_id, coins),
                            ClientMsg::Taunt { taunt } => room.taunt(&user_id, taunt),
                            ClientMsg::PetReact { action } => room.pet_react(&user_id, action).map(|fresh| {
                                if fresh && !entry.guest && greeted.insert(user_id.clone()) {
                                    server.pet_hello(&user_id);
                                }
                            }),
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
                server.report_milestones(milestones.observe(&room));

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
                    send_snaps(&room, &conns, std::mem::take(&mut events));
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
pub fn signing_secret(var: &str) -> [u8; 32] {
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

/// Whether this is a dev stack, where the sandbox is open: `PARTY_DEV_TOOLS=1`.
fn dev_tools() -> bool {
    std::env::var("PARTY_DEV_TOOLS").is_ok_and(|v| v == "1")
}

#[cfg(test)]
mod native_handoff_tests {
    use super::*;
    use serde_json::{Value, json};
    use tokio_tungstenite::{connect_async, tungstenite::Message as ClientMessage};

    #[tokio::test]
    async fn native_ticket_exchange_authenticates_the_same_player_and_consumes_once() {
        let reserve = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = reserve.local_addr().unwrap().port(); drop(reserve);
        let url = format!("ws://127.0.0.1:{port}/party/ws");
        let server = PartyServer::start(PartyConfig { bind: format!("127.0.0.1:{port}").parse().unwrap(), public_url: url.clone() }).await.unwrap();
        let source = server.issue_token("native-room", "native-player", "Player", "");
        let client = reqwest::Client::new();
        let endpoint = format!("http://127.0.0.1:{port}/party/native-pass");
        let wrong = client.post(&endpoint).json(&json!({"token":source,"user_id":"different","match_id":"native-room"})).send().await.unwrap();
        assert_eq!(wrong.status(), reqwest::StatusCode::UNAUTHORIZED);
        let response = client.post(&endpoint).json(&json!({"token":source,"user_id":"native-player","match_id":"native-room"})).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let ticket: Value = response.json().await.unwrap();
        assert_eq!(ticket["user_id"], "native-player"); assert_eq!(ticket["match_id"], "native-room");
        assert_ne!(ticket["token"], source);
        let token = ticket["token"].as_str().unwrap();
        let (mut first, _) = connect_async(&url).await.unwrap();
        first.send(ClientMessage::Text(json!({"type":"hello","token":token}).to_string().into())).await.unwrap();
        let message = tokio::time::timeout(Duration::from_secs(3), first.next()).await.unwrap().unwrap().unwrap();
        let data: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert_eq!(data["type"], "welcome"); assert_eq!(data["user_id"], "native-player");
        let (mut second, _) = connect_async(&url).await.unwrap();
        second.send(ClientMessage::Text(json!({"type":"hello","token":token}).to_string().into())).await.unwrap();
        let message = tokio::time::timeout(Duration::from_secs(3), second.next()).await.unwrap().unwrap().unwrap();
        let data: Value = serde_json::from_str(message.to_text().unwrap()).unwrap();
        assert_eq!(data["type"], "error"); assert_eq!(data["code"], "auth");
        let _ = first.close(None).await;
    }
}
