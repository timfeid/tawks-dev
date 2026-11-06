//! Routes served on the party port: the song list, charts and stems for
//! players holding a pass, and a WebSocket room per lobby.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use party::auth::{self, Claims};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tower::ServiceExt;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeFile;
use tracing::{info, warn};

use crate::access::LibraryAccess;
use crate::chart::{Difficulty, Instrument};
use crate::library::{Library, SongInfo, read_chart_file};
use crate::room::{Finished, PartChoice, Progress, Room, RoomPresence, run_record};
use crate::scores::{ApiScores, ChartQuery, ScoreBackend};
use crate::store::Store;
use crate::uploads::{
    self, ApiBackend, FileCache, MAX_TOTAL_BYTES, SongBackend, UploadForm, UploadRecord,
    api_message,
};

const MILESTONE_GAME_ID: &str = "encore";
const ROOM_TICK: Duration = Duration::from_millis(100);
const IDLE_ROOM_TICKS: u32 = 600;
const CACHE_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const REPORT_REASONS: [&str; 4] = ["copyright", "offensive", "broken", "other"];
const MAX_REPORT_DETAILS: usize = 1000;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Hello {
        token: String,
    },
    Ping {
        t: f64,
    },
    Pick {
        song_id: String,
    },
    Part {
        instrument: Option<Instrument>,
        difficulty: Option<Difficulty>,
    },
    Ready {
        ready: bool,
    },
    Loaded,
    Progress {
        progress: Progress,
        #[serde(default)]
        done: bool,
    },
    Again,
    Stop,
    Settings {
        settings: Value,
    },
    /// The sender's part dropped out (`out`) or came back at song time `t`.
    Cue {
        t: f64,
        out: bool,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg<'a> {
    Welcome {
        conn: u64,
        user_id: &'a str,
        settings: &'a Value,
        library: bool,
        uploads: bool,
    },
    Room {
        room: crate::room::RoomView<'a>,
    },
    Pong {
        t: f64,
        now: f64,
    },
    Error {
        message: &'a str,
    },
    Cue {
        conn: u64,
        t: f64,
        out: bool,
    },
    /// The player's own run as it was kept: their previous best and ranks.
    Run {
        run: &'a Value,
    },
}

fn text(msg: &ServerMsg) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

enum Cmd {
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
    Pick {
        conn: u64,
        song: SongInfo,
        length_ms: f64,
    },
    Forget {
        song_id: String,
    },
}

/// A song a player asked for, after the checks on who may hear it.
enum Resolved {
    Library(SongInfo),
    Upload(UploadRecord),
}

struct Inner {
    library: Library,
    access: LibraryAccess,
    /// None when uploads are off.
    uploads: Option<Arc<dyn SongBackend>>,
    cache: FileCache,
    store: Store,
    scores: Arc<dyn ScoreBackend>,
    secret: [u8; 32],
    rooms: DashMap<String, mpsc::UnboundedSender<Cmd>>,
    presence: DashMap<String, RoomPresence>,
    next_conn: AtomicU64,
    ws_url: String,
    api_url: String,
    /// Set once the bot connects; finished songs are reported as milestone events through it.
    bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>,
}

pub struct EncoreConfig {
    pub library: Library,
    pub access: LibraryAccess,
    pub store: Store,
    pub scores: Arc<dyn ScoreBackend>,
    pub cache_dir: PathBuf,
    pub secret: [u8; 32],
    pub ws_url: String,
    pub api_url: String,
}

fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| matches!(v.trim(), "1" | "true" | "yes" | "on"))
}

#[derive(Clone)]
pub struct EncoreServer {
    inner: Arc<Inner>,
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

impl EncoreServer {
    /// Scans `ENCORE_SONGS_DIR` and keeps player data in `ENCORE_DATA_DIR`.
    /// `ENCORE_UPLOADS=1` lets players upload their own songs. Returns None
    /// when there is neither a song folder nor uploads, so the game stays off.
    pub async fn from_env(party_public_url: &str) -> anyhow::Result<Option<Self>> {
        let uploads = env_flag("ENCORE_UPLOADS");
        let songs = std::env::var("ENCORE_SONGS_DIR")
            .ok()
            .filter(|s| !s.is_empty());
        let library = match songs {
            Some(songs) => {
                let root = PathBuf::from(songs);
                match tokio::task::spawn_blocking(move || Library::scan(&root)).await? {
                    Ok(library) => library,
                    Err(error) => {
                        warn!(%error, "the Encore song folder could not be read");
                        Library::empty()
                    }
                }
            }
            None => Library::empty(),
        };
        if library.is_empty() && !uploads {
            info!("no Encore songs and ENCORE_UPLOADS is off; Encore is off");
            return Ok(None);
        }
        let access = LibraryAccess::from_env();
        if !library.is_empty() {
            info!(
                songs = library.len(),
                accounts = access.len(),
                "the Encore library is open to its allow-list only"
            );
        }
        let data_dir = std::env::var("ENCORE_DATA_DIR").ok().map(PathBuf::from);
        let cache_dir = data_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("encore"))
            .join("upload-cache");
        let store = Store::open(data_dir);
        let base = party_public_url
            .trim_end_matches('/')
            .trim_end_matches("/party/ws");
        let api_base = base
            .replacen("wss://", "https://", 1)
            .replacen("ws://", "http://", 1);
        let bot = Arc::new(RwLock::new(None));
        let backend: Option<Arc<dyn SongBackend>> =
            uploads.then(|| Arc::new(ApiBackend::new(bot.clone())) as Arc<dyn SongBackend>);
        let config = EncoreConfig {
            library,
            access,
            store,
            scores: Arc::new(ApiScores::new(bot.clone())),
            cache_dir,
            secret: party::server::signing_secret("ENCORE_SECRET"),
            ws_url: format!("{base}/party/encore/ws"),
            api_url: format!("{api_base}/party/encore"),
        };
        Ok(Some(Self::new(config, backend, bot)))
    }

    pub fn new(
        config: EncoreConfig,
        uploads: Option<Arc<dyn SongBackend>>,
        bot: Arc<RwLock<Option<Arc<tawks_bot::TawksSdk>>>>,
    ) -> Self {
        EncoreServer {
            inner: Arc::new(Inner {
                library: config.library,
                access: config.access,
                uploads,
                cache: FileCache::new(config.cache_dir, CACHE_BYTES),
                store: config.store,
                scores: config.scores,
                secret: config.secret,
                rooms: DashMap::new(),
                presence: DashMap::new(),
                next_conn: AtomicU64::new(1),
                ws_url: config.ws_url,
                api_url: config.api_url,
                bot,
            }),
        }
    }

    pub fn issue_token(
        &self,
        lobby_id: &str,
        user_id: &str,
        name: &str,
        avatar_url: &str,
        guest: bool,
    ) -> String {
        let claims = Claims {
            match_id: lobby_id.to_string(),
            user_id: user_id.to_string(),
            name: name.to_string(),
            avatar_url: avatar_url.to_string(),
            exp: auth::now_unix() + auth::TOKEN_TTL_SECS,
            spectator: false,
            guest,
            drives: Vec::new(),
            game: Some("encore".into()),
            owner: false,
        };
        auth::sign(&claims, &self.inner.secret)
    }

    pub fn set_bot(&self, sdk: Arc<tawks_bot::TawksSdk>) {
        *self.inner.bot.write().unwrap() = Some(sdk);
    }

    /// Reports each finished part of an official song as an Encore
    /// `song_finished` milestone event. Library songs and uploads earn nothing.
    fn report_milestones(&self, lobby_id: &str, finished: &[Finished]) {
        let finished: Vec<&Finished> = finished.iter().filter(|f| f.official).collect();
        if finished.is_empty() {
            return;
        }
        let Some(sdk) = self.inner.bot.read().unwrap().clone() else {
            warn!("no bot connection yet; Encore milestones were not reported");
            return;
        };
        info!(
            parts = finished.len(),
            lobby = lobby_id,
            "reporting Encore milestones"
        );
        for part in finished {
            let progress = &part.progress;
            let accuracy = if progress.total > 0 {
                progress.hit * 100 / progress.total
            } else {
                0
            };
            let event = json!({
                "game_id": MILESTONE_GAME_ID,
                "event_key": "song_finished",
                "user_id": part.user_id,
                "lobby_id": lobby_id,
                "meta": {
                    "instrument": part.part.instrument.key(),
                    "difficulty": part.part.difficulty.key(),
                    "accuracy": accuracy,
                    "full_combo": progress.full_combo(),
                    "auto_kick": progress.auto_kick,
                    "band_size": part.band_size,
                    "score": part.score,
                    "song_id": part.song_id,
                    "official": part.official,
                },
            });
            let sdk = sdk.clone();
            tokio::spawn(async move {
                if let Err(error) = sdk
                    .web_client
                    .mutate::<_, ()>("milestone_event", &event)
                    .await
                {
                    warn!(%error, "an Encore milestone event did not send");
                }
            });
        }
    }

    /// Keeps each part that counts and tells its player how the run stands.
    fn record_runs(
        &self,
        lobby_id: &str,
        started_at_ms: Option<f64>,
        finished: &[Finished],
        listeners: Vec<(String, mpsc::UnboundedSender<Message>)>,
    ) {
        let runs: Vec<_> = finished
            .iter()
            .filter_map(|part| run_record(lobby_id, started_at_ms, part))
            .collect();
        if runs.is_empty() {
            return;
        }
        let scores = self.inner.scores.clone();
        tokio::spawn(async move {
            let recorded = match scores.record(&runs).await {
                Ok(recorded) => recorded,
                Err(error) => return warn!(%error, "Encore runs were not kept"),
            };
            for run in &recorded {
                let user = run
                    .get("user_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let message = text(&ServerMsg::Run { run });
                for (_, out) in listeners.iter().filter(|(listener, _)| listener == user) {
                    let _ = out.send(message.clone());
                }
            }
        });
    }

    fn report_award(&self, lobby_id: &str, started_at_ms: Option<f64>, finished: &[Finished]) {
        let Some(args) = crate::room::award_args(lobby_id, started_at_ms, finished) else {
            return;
        };
        let Some(sdk) = self.inner.bot.read().unwrap().clone() else {
            warn!("no bot connection yet; an Encore award was not reported");
            return;
        };
        tokio::spawn(async move {
            if let Err(error) = sdk.web_client.mutate::<_, Value>("game_award", &args).await {
                warn!(%error, "an Encore game_award did not send");
            }
        });
    }

    pub fn ws_url(&self) -> &str {
        &self.inner.ws_url
    }

    pub fn api_url(&self) -> &str {
        &self.inner.api_url
    }

    pub fn router(&self) -> Router {
        // Activities run with an opaque origin, so every response needs CORS.
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        Router::new()
            .route("/party/encore/ws", get(ws_handler))
            .route("/party/encore/me", get(me))
            .route("/party/encore/songs", get(catalog))
            .route("/party/encore/scores", get(chart_scores))
            .route("/party/encore/scores/mine", get(my_bests))
            .route("/party/encore/songs/:id/chart", get(chart))
            .route("/party/encore/songs/:id/files/:name", get(file))
            .route("/party/encore/songs/:id/report", post(report))
            .route(
                "/party/encore/uploads",
                post(upload).layer(DefaultBodyLimit::max(MAX_TOTAL_BYTES + 4 * 1024 * 1024)),
            )
            .route("/party/encore/uploads/:id", delete(remove_upload))
            .layer(cors)
            .with_state(self.clone())
    }

    fn check(&self, token: &str) -> Option<Claims> {
        auth::verify(token, &self.inner.secret, auth::now_unix())
    }

    /// Uploads by the people in this pass's room, the holder's own, and
    /// every official song.
    async fn visible_uploads(&self, claims: &Claims) -> Vec<SongInfo> {
        let Some(backend) = &self.inner.uploads else {
            return Vec::new();
        };
        let mut users = self
            .inner
            .presence
            .get(&claims.match_id)
            .map(|p| p.users.clone())
            .unwrap_or_default();
        users.insert(claims.user_id.clone());
        let ids: Vec<String> = users.iter().cloned().collect();
        match backend.list(&ids).await {
            Ok(records) => records
                .into_iter()
                .filter(|r| r.official || users.contains(&r.uploaded_by))
                .map(|r| r.info())
                .collect(),
            Err(error) => {
                warn!(%error, "the Encore uploads did not load");
                Vec::new()
            }
        }
    }

    /// Whether this pass may hear an upload: it is official, it is the
    /// holder's own, its uploader is in the holder's room, or the room is
    /// playing it right now.
    fn may_hear(&self, claims: &Claims, record: &UploadRecord) -> bool {
        if record.official || record.uploaded_by == claims.user_id {
            return true;
        }
        self.inner
            .presence
            .get(&claims.match_id)
            .is_some_and(|room| {
                room.users.contains(&record.uploaded_by)
                    || room.song.as_deref() == Some(record.id.as_str())
            })
    }

    async fn resolve(&self, claims: &Claims, id: &str) -> Option<Resolved> {
        if let Some(song) = self.inner.library.song(id) {
            return self
                .inner
                .access
                .allows(claims)
                .then(|| Resolved::Library(song.clone()));
        }
        let backend = self.inner.uploads.as_ref()?;
        let record = match backend.get(id).await {
            Ok(record) => record?,
            Err(error) => {
                warn!(%error, song = id, "an Encore upload did not load");
                return None;
            }
        };
        self.may_hear(claims, &record)
            .then_some(Resolved::Upload(record))
    }

    fn length_ms(&self, song: &SongInfo) -> f64 {
        if song.length_ms > 0 {
            return song.length_ms as f64;
        }
        self.inner
            .library
            .chart(&song.id)
            .map(|c| c.length + 3_000.0)
            .unwrap_or(600_000.0)
    }

    fn forget_everywhere(&self, song_id: &str) {
        self.inner.cache.forget(song_id);
        for room in self.inner.rooms.iter() {
            let _ = room.value().send(Cmd::Forget {
                song_id: song_id.to_string(),
            });
        }
    }

    fn room(&self, match_id: &str) -> mpsc::UnboundedSender<Cmd> {
        self.inner
            .rooms
            .entry(match_id.to_string())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::unbounded_channel();
                tokio::spawn(run_room(self.clone(), match_id.to_string(), tx.clone(), rx));
                tx
            })
            .clone()
    }
}

#[derive(Deserialize)]
struct Pass {
    token: String,
}

fn unauthorized() -> Response {
    (StatusCode::UNAUTHORIZED, "This pass has expired").into_response()
}

fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "That song is not here").into_response()
}

fn refused(status: StatusCode, message: impl Into<String>) -> Response {
    (status, axum::Json(json!({ "message": message.into() }))).into_response()
}

fn json_body(value: String) -> Response {
    ([(header::CONTENT_TYPE, "application/json")], value).into_response()
}

async fn catalog(State(server): State<EncoreServer>, Query(pass): Query<Pass>) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    let uploads = server.visible_uploads(&claims).await;
    let mut parts = Vec::new();
    if !uploads.is_empty() {
        let json = serde_json::to_string(&uploads).unwrap_or_else(|_| "[]".into());
        parts.push(json[1..json.len() - 1].to_string());
    }
    if server.inner.access.allows(&claims) {
        let library = server.inner.library.catalog_json();
        if library.len() > 2 {
            parts.push(library[1..library.len() - 1].to_string());
        }
    }
    json_body(format!("[{}]", parts.join(",")))
}

#[derive(Deserialize)]
struct ScoresQuery {
    token: String,
    song_id: String,
    instrument: Instrument,
    difficulty: Difficulty,
    #[serde(default)]
    everyone: bool,
}

/// One chart's leaderboard, for a song the pass may hear. Everyone's scores
/// are only kept for official songs; the rest are among friends.
async fn chart_scores(
    State(server): State<EncoreServer>,
    Query(query): Query<ScoresQuery>,
) -> Response {
    let Some(claims) = server.check(&query.token) else {
        return unauthorized();
    };
    if claims.guest {
        return refused(StatusCode::FORBIDDEN, "Sign in to see scores");
    }
    let official = match server.resolve(&claims, &query.song_id).await {
        Some(Resolved::Library(song)) => song.official,
        Some(Resolved::Upload(record)) => record.official,
        None => return not_found(),
    };
    if query.everyone && !official {
        return refused(StatusCode::BAD_REQUEST, "Only official songs rank everyone");
    }
    let chart = ChartQuery {
        viewer: claims.user_id.clone(),
        song_id: query.song_id,
        instrument: query.instrument.key().to_string(),
        difficulty: query.difficulty.key().to_string(),
        everyone: query.everyone,
    };
    match server.inner.scores.chart(&chart).await {
        Ok(scores) => axum::Json(scores).into_response(),
        Err(error) => refused(StatusCode::SERVICE_UNAVAILABLE, api_message(&error)),
    }
}

async fn my_bests(State(server): State<EncoreServer>, Query(pass): Query<Pass>) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    if claims.guest {
        return axum::Json(json!([])).into_response();
    }
    match server.inner.scores.bests(&claims.user_id).await {
        Ok(bests) => axum::Json(bests).into_response(),
        Err(error) => refused(StatusCode::SERVICE_UNAVAILABLE, api_message(&error)),
    }
}

async fn me(State(server): State<EncoreServer>, Query(pass): Query<Pass>) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    let status = match &server.inner.uploads {
        Some(_) if claims.guest => Some(uploads::UploaderStatus {
            can_upload: false,
            reason: Some("Sign in to upload songs".into()),
            ..Default::default()
        }),
        Some(backend) => match backend.status(&claims.user_id).await {
            Ok(status) => Some(status),
            Err(error) => Some(uploads::UploaderStatus {
                can_upload: false,
                reason: Some(api_message(&error)),
                ..Default::default()
            }),
        },
        None => None,
    };
    axum::Json(json!({
        "library": server.inner.access.allows(&claims),
        "uploads": server.inner.uploads.is_some(),
        "status": status,
        "limits": {
            "max_total_mb": uploads::MAX_TOTAL_BYTES / (1024 * 1024),
            "max_audio_mb": uploads::MAX_AUDIO_BYTES / (1024 * 1024),
            "max_minutes": uploads::MAX_LENGTH_MS / 60_000,
            "formats": uploads::UPLOAD_AUDIO,
        },
    }))
    .into_response()
}

async fn chart(
    State(server): State<EncoreServer>,
    Path(id): Path<String>,
    Query(pass): Query<Pass>,
) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    match server.resolve(&claims, &id).await {
        Some(Resolved::Library(_)) => {
            let library = server.clone();
            match tokio::task::spawn_blocking(move || library.inner.library.chart_json(&id)).await {
                Ok(Ok(json)) => json_body(json.to_string()),
                Ok(Err(error)) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Some(Resolved::Upload(record)) => {
            let backend = server.inner.uploads.clone().expect("uploads are on");
            let path = match server
                .inner
                .cache
                .get(backend.as_ref(), &record.id, &record.song.chart)
                .await
            {
                Ok(path) => path,
                Err(error) => {
                    warn!(%error, song = record.id, "an uploaded chart did not load");
                    return not_found();
                }
            };
            let song = record.song.clone();
            let parsed = tokio::task::spawn_blocking(move || {
                let bytes = std::fs::read(&path)?;
                let chart = read_chart_file(&song.chart, &bytes, song.hints)?;
                anyhow::Ok(serde_json::to_string(&chart)?)
            })
            .await;
            match parsed {
                Ok(Ok(json)) => json_body(json),
                _ => not_found(),
            }
        }
        None => not_found(),
    }
}

async fn file(
    State(server): State<EncoreServer>,
    Path((id, name)): Path<(String, String)>,
    Query(pass): Query<Pass>,
    request: Request,
) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    let (path, cache) = match server.resolve(&claims, &id).await {
        Some(Resolved::Library(_)) => match server.inner.library.file(&id, &name) {
            Some(path) => (path, "private, max-age=86400"),
            None => return not_found(),
        },
        Some(Resolved::Upload(record)) => {
            let Some(stored) = record.song.files.get(&name) else {
                return not_found();
            };
            let backend = server.inner.uploads.clone().expect("uploads are on");
            match server
                .inner
                .cache
                .get(backend.as_ref(), &record.id, stored)
                .await
            {
                Ok(path) => (path, "private, max-age=600"),
                Err(error) => {
                    warn!(%error, song = record.id, "an uploaded file did not load");
                    return not_found();
                }
            }
        }
        None => return not_found(),
    };
    match ServeFile::new(path).oneshot(request).await {
        Ok(response) => {
            let mut response = response.into_response();
            response
                .headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static(cache));
            response
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn upload(
    State(server): State<EncoreServer>,
    Query(pass): Query<Pass>,
    mut multipart: Multipart,
) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    let Some(backend) = server.inner.uploads.clone() else {
        return refused(StatusCode::NOT_FOUND, "Uploads are off");
    };
    if claims.guest || claims.spectator {
        return refused(StatusCode::FORBIDDEN, "Sign in to upload songs");
    }
    match backend.status(&claims.user_id).await {
        Ok(status) if status.can_upload => {}
        Ok(status) => {
            return refused(
                StatusCode::FORBIDDEN,
                status
                    .reason
                    .unwrap_or_else(|| "You can't upload songs right now".into()),
            );
        }
        Err(error) => return refused(StatusCode::SERVICE_UNAVAILABLE, api_message(&error)),
    }
    let mut form = UploadForm::default();
    let mut total = 0usize;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(_) => {
                return refused(
                    StatusCode::BAD_REQUEST,
                    "The upload did not arrive in one piece",
                );
            }
        };
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().map(String::from);
        let Ok(bytes) = field.bytes().await else {
            return refused(StatusCode::PAYLOAD_TOO_LARGE, "That upload is too large");
        };
        total += bytes.len();
        if total > MAX_TOTAL_BYTES + 1024 * 1024 {
            return refused(StatusCode::PAYLOAD_TOO_LARGE, "That upload is too large");
        }
        let text = || String::from_utf8_lossy(&bytes).to_string();
        match (name.as_str(), file_name) {
            (_, Some(file_name)) => form.files.push((file_name, bytes.to_vec())),
            ("name", None) => form.name = Some(text()),
            ("artist", None) => form.artist = Some(text()),
            ("rights", None) => {
                form.rights = matches!(text().trim(), "accepted" | "true" | "on" | "1")
            }
            _ => {}
        }
    }
    let validated = match tokio::task::spawn_blocking(move || uploads::validate(form)).await {
        Ok(Ok(validated)) => validated,
        Ok(Err(message)) => return refused(StatusCode::UNPROCESSABLE_ENTITY, message),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    match backend.create(&claims.user_id, &validated).await {
        Ok(id) => {
            info!(
                song = id,
                user = claims.user_id,
                "an Encore song was uploaded"
            );
            let mut song = validated.song.info.clone();
            song.id = id.clone();
            song.uploaded_by = Some(claims.user_id.clone());
            song.uploader_name = Some(claims.name.clone());
            axum::Json(json!({ "id": id, "song": song, "skipped": validated.skipped }))
                .into_response()
        }
        Err(error) => {
            warn!(%error, "an Encore upload was not stored");
            refused(StatusCode::BAD_REQUEST, api_message(&error))
        }
    }
}

async fn remove_upload(
    State(server): State<EncoreServer>,
    Path(id): Path<String>,
    Query(pass): Query<Pass>,
) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    let Some(backend) = server.inner.uploads.clone() else {
        return not_found();
    };
    match backend.delete(&id, &claims.user_id).await {
        Ok(()) => {
            server.forget_everywhere(&id);
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => refused(StatusCode::BAD_REQUEST, api_message(&error)),
    }
}

#[derive(Deserialize)]
struct ReportBody {
    reason: String,
    #[serde(default)]
    details: String,
}

async fn report(
    State(server): State<EncoreServer>,
    Path(id): Path<String>,
    Query(pass): Query<Pass>,
    axum::Json(body): axum::Json<ReportBody>,
) -> Response {
    let Some(claims) = server.check(&pass.token) else {
        return unauthorized();
    };
    if !REPORT_REASONS.contains(&body.reason.as_str()) {
        return refused(StatusCode::BAD_REQUEST, "Pick a reason");
    }
    let details: String = body
        .details
        .trim()
        .chars()
        .take(MAX_REPORT_DETAILS)
        .collect();
    let Some(Resolved::Upload(record)) = server.resolve(&claims, &id).await else {
        return not_found();
    };
    let backend = server.inner.uploads.clone().expect("uploads are on");
    match backend
        .report(
            &record.id,
            &claims.user_id,
            &claims.match_id,
            &body.reason,
            &details,
        )
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => refused(StatusCode::BAD_REQUEST, api_message(&error)),
    }
}

async fn ws_handler(State(server): State<EncoreServer>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.max_message_size(64 * 1024)
        .on_upgrade(move |socket| serve_socket(server, socket))
}

async fn serve_socket(server: EncoreServer, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let claims = loop {
        match stream.next().await {
            Some(Ok(Message::Text(raw))) => match serde_json::from_str::<ClientMsg>(&raw) {
                Ok(ClientMsg::Hello { token }) => match server.check(&token) {
                    Some(claims) => break claims,
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
    let mut room = server.room(&claims.match_id);
    let user_id = claims.user_id.clone();
    let match_id = claims.match_id.clone();
    if let Err(mpsc::error::SendError(Cmd::Connect { claims, out, .. })) = room.send(Cmd::Connect {
        conn,
        claims: claims.clone(),
        out: out.clone(),
    }) {
        server.inner.rooms.remove(&match_id);
        room = server.room(&match_id);
        let _ = room.send(Cmd::Connect { conn, claims, out });
    }

    let writer = tokio::spawn(async move {
        while let Some(message) = outbox.recv().await {
            if sink.send(message).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = stream.next().await {
        let Message::Text(raw) = message else {
            continue;
        };
        let Ok(msg) = serde_json::from_str::<ClientMsg>(&raw) else {
            continue;
        };
        match msg {
            ClientMsg::Ping { t } => {
                let _ = out.send(text(&ServerMsg::Pong { t, now: now_ms() }));
            }
            ClientMsg::Settings { settings } => {
                if let Err(message) = server.inner.store.set_settings(&user_id, settings) {
                    let _ = out.send(text(&ServerMsg::Error { message: &message }));
                }
            }
            ClientMsg::Hello { .. } => {}
            ClientMsg::Pick { song_id } => match server.resolve(&claims, &song_id).await {
                Some(resolved) => {
                    let song = match resolved {
                        Resolved::Library(song) => song,
                        Resolved::Upload(record) => record.info(),
                    };
                    let length_ms = server.length_ms(&song);
                    let _ = room.send(Cmd::Pick {
                        conn,
                        song,
                        length_ms,
                    });
                }
                None => {
                    let _ = out.send(text(&ServerMsg::Error {
                        message: "That song is gone",
                    }));
                }
            },
            msg => {
                let _ = room.send(Cmd::Client { conn, msg });
            }
        }
    }
    let _ = room.send(Cmd::Disconnect { conn });
    writer.abort();
}

async fn run_room(
    server: EncoreServer,
    match_id: String,
    me: mpsc::UnboundedSender<Cmd>,
    mut commands: mpsc::UnboundedReceiver<Cmd>,
) {
    let mut room = Room::default();
    let mut outs: HashMap<u64, mpsc::UnboundedSender<Message>> = HashMap::new();
    let mut ticker = tokio::time::interval(ROOM_TICK);
    let mut idle = 0u32;
    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                handle(&server, &mut room, &mut outs, command);
                server.inner.presence.insert(match_id.clone(), room.presence());
            }
            _ = ticker.tick() => {
                let finished = room.tick(now_ms());
                if !finished.is_empty() {
                    let listeners = outs
                        .iter()
                        .filter_map(|(conn, out)| Some((room.players.get(conn)?.user_id.clone(), out.clone())))
                        .collect();
                    server.record_runs(&match_id, room.start_at, &finished, listeners);
                }
                server.report_milestones(&match_id, &finished);
                server.report_award(&match_id, room.start_at, &finished);
                if room.dirty {
                    room.dirty = false;
                    server.inner.presence.insert(match_id.clone(), room.presence());
                    let full = text(&ServerMsg::Room { room: room.view_for(true) });
                    let limited = text(&ServerMsg::Room { room: room.view_for(false) });
                    outs.retain(|conn, out| {
                        let library = room.players.get(conn).is_some_and(|p| p.library);
                        out.send(if library { full.clone() } else { limited.clone() }).is_ok()
                    });
                }
                idle = if outs.is_empty() { idle + 1 } else { 0 };
                if idle > IDLE_ROOM_TICKS {
                    break;
                }
            }
        }
    }
    if server
        .inner
        .rooms
        .remove_if(&match_id, |_, tx| tx.same_channel(&me))
        .is_some()
    {
        server.inner.presence.remove(&match_id);
    }
}

fn handle(
    server: &EncoreServer,
    room: &mut Room,
    outs: &mut HashMap<u64, mpsc::UnboundedSender<Message>>,
    command: Cmd,
) {
    match command {
        Cmd::Connect { conn, claims, out } => {
            let data = server.inner.store.get(&claims.user_id);
            let library = server.inner.access.allows(&claims);
            let _ = out.send(text(&ServerMsg::Welcome {
                conn,
                user_id: &claims.user_id,
                settings: &data.settings,
                library,
                uploads: server.inner.uploads.is_some(),
            }));
            room.join(
                conn,
                &claims.user_id,
                &claims.name,
                &claims.avatar_url,
                library,
            );
            outs.insert(conn, out);
        }
        Cmd::Pick {
            conn,
            song,
            length_ms,
        } => {
            if let Err(message) = room.pick(conn, song, length_ms) {
                if let Some(out) = outs.get(&conn) {
                    let _ = out.send(text(&ServerMsg::Error { message: &message }));
                }
            }
        }
        Cmd::Forget { song_id } => room.forget(&song_id),
        Cmd::Disconnect { conn } => {
            room.leave(conn, now_ms());
            outs.remove(&conn);
        }
        Cmd::Client { conn, msg } => {
            let result = match msg {
                ClientMsg::Part {
                    instrument,
                    difficulty,
                } => {
                    let part = instrument.map(|instrument| PartChoice {
                        instrument,
                        difficulty: difficulty.unwrap_or(Difficulty::Medium),
                    });
                    room.set_part(conn, part)
                }
                ClientMsg::Ready { ready } => room.set_ready(conn, ready),
                ClientMsg::Loaded => {
                    room.loaded(conn);
                    Ok(())
                }
                ClientMsg::Progress { progress, done } => {
                    room.progress(conn, progress, done);
                    Ok(())
                }
                ClientMsg::Again => {
                    room.again();
                    Ok(())
                }
                ClientMsg::Stop => room.stop(conn),
                ClientMsg::Cue { t, out } => {
                    if room.in_song(conn) && t.is_finite() {
                        let cue = text(&ServerMsg::Cue { conn, t, out });
                        for (other, sender) in outs.iter() {
                            if *other != conn {
                                let _ = sender.send(cue.clone());
                            }
                        }
                    }
                    Ok(())
                }
                ClientMsg::Hello { .. }
                | ClientMsg::Ping { .. }
                | ClientMsg::Settings { .. }
                | ClientMsg::Pick { .. } => Ok(()),
            };
            if let Err(message) = result {
                if let Some(out) = outs.get(&conn) {
                    let _ = out.send(text(&ServerMsg::Error { message: &message }));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scores::MemoryScores;
    use crate::uploads::MemoryBackend;
    use crate::uploads::tests::{CHART, good_upload};
    use axum::body::Body;
    use axum::http::Method;

    struct Fixture {
        server: EncoreServer,
        backend: Arc<MemoryBackend>,
        scores: Arc<MemoryScores>,
        library_id: String,
        root: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn fixture(name: &str) -> Fixture {
        let root =
            std::env::temp_dir().join(format!("encore-server-{name}-{}", std::process::id()));
        let song = root.join("songs").join("Band - Tune");
        std::fs::create_dir_all(&song).unwrap();
        std::fs::write(
            song.join("song.ini"),
            "[song]\nname = Tune\nartist = Band\ndiff_guitar = 3\n",
        )
        .unwrap();
        std::fs::write(song.join("notes.chart"), CHART).unwrap();
        std::fs::write(song.join("song.ogg"), b"library audio").unwrap();
        let library = Library::scan(&root.join("songs")).unwrap();
        let catalog: Vec<Value> = serde_json::from_str(library.catalog_json()).unwrap();
        let library_id = catalog[0]["id"].as_str().unwrap().to_string();
        let backend = Arc::new(MemoryBackend::default());
        let scores = Arc::new(MemoryScores::default());
        let server = EncoreServer::new(
            EncoreConfig {
                library,
                access: LibraryAccess::parse("owner"),
                store: Store::open(None),
                scores: scores.clone() as Arc<dyn ScoreBackend>,
                cache_dir: root.join("cache"),
                secret: [7; 32],
                ws_url: "ws://test/party/encore/ws".into(),
                api_url: "http://test/party/encore".into(),
            },
            Some(backend.clone() as Arc<dyn SongBackend>),
            Arc::new(RwLock::new(None)),
        );
        Fixture {
            server,
            backend,
            scores,
            library_id,
            root,
        }
    }

    fn pass(server: &EncoreServer, lobby: &str, user: &str) -> String {
        server.issue_token(lobby, user, user, "", false)
    }

    fn in_room(server: &EncoreServer, lobby: &str, users: &[&str]) {
        server.inner.presence.insert(
            lobby.to_string(),
            RoomPresence {
                users: users.iter().map(|u| u.to_string()).collect(),
                song: None,
            },
        );
    }

    async fn call(
        server: &EncoreServer,
        method: Method,
        uri: &str,
        body: Body,
        content_type: &str,
    ) -> (StatusCode, Vec<u8>) {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::CONTENT_TYPE, content_type)
            .body(body)
            .unwrap();
        let response = server.router().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, bytes.to_vec())
    }

    async fn get(server: &EncoreServer, uri: &str) -> (StatusCode, Vec<u8>) {
        call(server, Method::GET, uri, Body::empty(), "text/plain").await
    }

    async fn catalog_ids(server: &EncoreServer, token: &str) -> Vec<String> {
        let (status, body) = get(server, &format!("/party/encore/songs?token={token}")).await;
        assert_eq!(status, StatusCode::OK);
        let songs: Vec<Value> = serde_json::from_slice(&body).unwrap();
        songs
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    }

    fn multipart(fields: &[(&str, &str)], files: &[(String, Vec<u8>)]) -> (Body, String) {
        let boundary = "encoreboundary";
        let mut body = Vec::new();
        for (name, value) in fields {
            body.extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").bytes());
        }
        for (name, bytes) in files {
            body.extend(
                format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/octet-stream\r\n\r\n")
                    .bytes(),
            );
            body.extend(bytes);
            body.extend(b"\r\n");
        }
        body.extend(format!("--{boundary}--\r\n").bytes());
        (
            Body::from(body),
            format!("multipart/form-data; boundary={boundary}"),
        )
    }

    async fn upload_as(server: &EncoreServer, token: &str, rights: &str) -> (StatusCode, Value) {
        let (body, content_type) = multipart(
            &[("name", "Mine"), ("artist", "Me"), ("rights", rights)],
            &good_upload().files,
        );
        let (status, bytes) = call(
            server,
            Method::POST,
            &format!("/party/encore/uploads?token={token}"),
            body,
            &content_type,
        )
        .await;
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    #[tokio::test]
    async fn the_library_is_only_for_the_allow_list() {
        let f = fixture("library");
        let owner = pass(&f.server, "L1", "owner");
        let stranger = pass(&f.server, "L1", "stranger");
        let guest = f.server.issue_token("L1", "owner", "owner", "", true);
        let id = &f.library_id;

        assert_eq!(catalog_ids(&f.server, &owner).await, vec![id.clone()]);
        assert!(catalog_ids(&f.server, &stranger).await.is_empty());
        assert!(catalog_ids(&f.server, &guest).await.is_empty());

        for path in [
            format!("songs/{id}/chart"),
            format!("songs/{id}/files/song"),
            format!("songs/{id}/files/album"),
        ] {
            let (status, _) =
                get(&f.server, &format!("/party/encore/{path}?token={stranger}")).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        }
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/chart?token={owner}")
            )
            .await
            .0,
            StatusCode::OK
        );
        let (status, body) = get(
            &f.server,
            &format!("/party/encore/songs/{id}/files/song?token={owner}"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, b"library audio");
        assert_eq!(
            get(&f.server, "/party/encore/songs?token=forged").await.0,
            StatusCode::UNAUTHORIZED
        );

        let (status, _) = call(
            &f.server,
            Method::POST,
            &format!("/party/encore/songs/{id}/report?token={owner}"),
            Body::from(r#"{"reason":"copyright"}"#),
            "application/json",
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn uploads_stay_with_their_uploader() {
        let f = fixture("uploads");
        let ann = pass(&f.server, "L1", "ann");
        let (status, body) = upload_as(&f.server, &ann, "no").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let guest = f.server.issue_token("L1", "g1", "Guest", "", true);
        assert_eq!(
            upload_as(&f.server, &guest, "accepted").await.0,
            StatusCode::FORBIDDEN
        );

        let (status, body) = upload_as(&f.server, &ann, "accepted").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let id = body["id"].as_str().unwrap().to_string();
        assert_eq!(catalog_ids(&f.server, &ann).await, vec![id.clone()]);

        let bo_elsewhere = pass(&f.server, "L2", "bo");
        in_room(&f.server, "L2", &["bo"]);
        assert!(catalog_ids(&f.server, &bo_elsewhere).await.is_empty());
        let stem = format!("/party/encore/songs/{id}/files/song?token={bo_elsewhere}");
        assert_eq!(get(&f.server, &stem).await.0, StatusCode::NOT_FOUND);
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/chart?token={bo_elsewhere}")
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );

        let bo = pass(&f.server, "L1", "bo");
        in_room(&f.server, "L1", &["ann", "bo"]);
        assert_eq!(catalog_ids(&f.server, &bo).await, vec![id.clone()]);
        let (status, audio) = get(
            &f.server,
            &format!("/party/encore/songs/{id}/files/song?token={bo}"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(audio.starts_with(b"OggS"));
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/chart?token={bo}")
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/files/notes.chart?token={bo}")
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );

        in_room(&f.server, "L1", &["bo"]);
        assert!(catalog_ids(&f.server, &bo).await.is_empty());
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/chart?token={bo}")
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );

        let report = |token: &str, reason: &str| {
            let uri = format!("/party/encore/songs/{id}/report?token={token}");
            let body = Body::from(format!(r#"{{"reason":"{reason}","details":"not theirs"}}"#));
            let server = f.server.clone();
            async move {
                call(&server, Method::POST, &uri, body, "application/json")
                    .await
                    .0
            }
        };
        assert_eq!(report(&bo, "copyright").await, StatusCode::NOT_FOUND);
        in_room(&f.server, "L1", &["ann", "bo"]);
        assert_eq!(report(&bo, "made-up").await, StatusCode::BAD_REQUEST);
        assert_eq!(report(&bo, "copyright").await, StatusCode::NO_CONTENT);
        assert_eq!(
            f.backend.reports.lock().unwrap().as_slice(),
            &[(id.clone(), "bo".to_string(), "copyright".to_string())]
        );

        let delete = |token: &str| {
            let uri = format!("/party/encore/uploads/{id}?token={token}");
            let server = f.server.clone();
            async move {
                call(&server, Method::DELETE, &uri, Body::empty(), "text/plain")
                    .await
                    .0
            }
        };
        assert_eq!(delete(&bo).await, StatusCode::BAD_REQUEST);
        assert_eq!(delete(&ann).await, StatusCode::NO_CONTENT);
        assert!(catalog_ids(&f.server, &ann).await.is_empty());
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/files/song?token={bo}")
            )
            .await
            .0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn official_songs_are_for_everyone_and_blocked_uploaders_are_refused() {
        let f = fixture("official");
        let ann = pass(&f.server, "L1", "ann");
        let (_, body) = upload_as(&f.server, &ann, "accepted").await;
        let id = body["id"].as_str().unwrap().to_string();
        f.backend.songs.lock().unwrap()[0].0.official = true;
        let stranger = pass(&f.server, "L9", "stranger");
        assert_eq!(catalog_ids(&f.server, &stranger).await, vec![id.clone()]);
        assert_eq!(
            get(
                &f.server,
                &format!("/party/encore/songs/{id}/chart?token={stranger}")
            )
            .await
            .0,
            StatusCode::OK
        );

        f.backend.blocked.lock().unwrap().insert("ann".into());
        let (status, body) = upload_as(&f.server, &ann, "accepted").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body["message"].as_str().unwrap().contains("removed"));
        let (status, me) = get(&f.server, &format!("/party/encore/me?token={ann}")).await;
        assert_eq!(status, StatusCode::OK);
        let me: Value = serde_json::from_slice(&me).unwrap();
        assert_eq!(me["library"], false);
        assert_eq!(me["status"]["can_upload"], false);
    }

    #[tokio::test]
    async fn scores_are_for_songs_the_pass_may_hear_and_everyone_only_on_official_ones() {
        let f = fixture("scores");
        let owner = pass(&f.server, "L1", "owner");
        let stranger = pass(&f.server, "L1", "stranger");
        let guest = f.server.issue_token("L1", "owner", "owner", "", true);
        let id = f.library_id.clone();
        let scores = |token: &str, song: &str, everyone: bool| {
            let uri = format!(
                "/party/encore/scores?token={token}&song_id={song}&instrument=guitar&difficulty=expert&everyone={everyone}"
            );
            let server = f.server.clone();
            async move { get(&server, &uri).await.0 }
        };
        assert_eq!(scores(&owner, &id, false).await, StatusCode::OK);
        assert_eq!(scores(&owner, &id, true).await, StatusCode::BAD_REQUEST);
        assert_eq!(scores(&stranger, &id, false).await, StatusCode::NOT_FOUND);
        assert_eq!(scores(&guest, &id, false).await, StatusCode::FORBIDDEN);
        assert_eq!(scores("forged", &id, false).await, StatusCode::UNAUTHORIZED);
        let bad = format!(
            "/party/encore/scores?token={owner}&song_id={id}&instrument=kazoo&difficulty=expert"
        );
        assert_eq!(get(&f.server, &bad).await.0, StatusCode::BAD_REQUEST);

        let ann = pass(&f.server, "L1", "ann");
        let (_, body) = upload_as(&f.server, &ann, "accepted").await;
        let upload = body["id"].as_str().unwrap().to_string();
        f.backend.songs.lock().unwrap()[0].0.official = true;
        assert_eq!(scores(&stranger, &upload, true).await, StatusCode::OK);

        let asked = f.scores.asked.lock().unwrap().clone();
        assert_eq!(
            asked,
            vec![
                ChartQuery {
                    viewer: "owner".into(),
                    song_id: id.clone(),
                    instrument: "guitar".into(),
                    difficulty: "expert".into(),
                    everyone: false,
                },
                ChartQuery {
                    viewer: "stranger".into(),
                    song_id: upload,
                    instrument: "guitar".into(),
                    difficulty: "expert".into(),
                    everyone: true,
                },
            ]
        );
    }

    #[tokio::test]
    async fn finished_parts_are_kept_and_each_player_hears_about_their_own() {
        let f = fixture("runs");
        let part = PartChoice {
            instrument: Instrument::Guitar,
            difficulty: Difficulty::Expert,
        };
        let finished = |user: &str, hit: u32, missed: u32| Finished {
            user_id: user.into(),
            song_id: "s1".into(),
            part,
            score: hit as u64 * 50,
            progress: Progress {
                score: hit as u64 * 50,
                hit,
                missed,
                total: 100,
                best_streak: hit,
                ..Default::default()
            },
            band_size: 3,
            official: false,
        };
        let (ann_out, mut ann_in) = mpsc::unbounded_channel();
        let (bo_out, mut bo_in) = mpsc::unbounded_channel();
        f.server.record_runs(
            "L1",
            Some(1_000.0),
            &[
                finished("ann", 90, 10),
                finished("bo", 40, 0),
                finished("cy", 100, 0),
            ],
            vec![("ann".into(), ann_out), ("bo".into(), bo_out)],
        );
        let Some(Message::Text(raw)) = ann_in.recv().await else {
            panic!("ann heard nothing")
        };
        let heard: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(heard["type"], "run");
        assert_eq!(heard["run"]["user_id"], "ann");
        assert!(bo_in.try_recv().is_err());

        let recorded = f.scores.recorded.lock().unwrap().clone();
        let users: Vec<&str> = recorded.iter().map(|r| r.user_id.as_str()).collect();
        assert_eq!(users, vec!["ann", "cy"]);
        assert_eq!(recorded[0].reference_id, "encore:L1:1000:s1:ann:guitar");
        assert!(recorded[1].full_combo);
        assert_eq!(recorded[1].stars, 5);

        let mine = format!(
            "/party/encore/scores/mine?token={}",
            pass(&f.server, "L1", "cy")
        );
        let (status, body) = get(&f.server, &mine).await;
        assert_eq!(status, StatusCode::OK);
        let bests: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(bests[0]["score"], 5_000);
    }
}
