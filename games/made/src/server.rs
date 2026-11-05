//! The made relay's socket route and its rooms, one task per lobby.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use made_protocol::protocol::{ClientMsg, MAX_CLIENT_MSG_BYTES, RelayMsg};
use sha2::Digest;
use tokio::sync::mpsc;

use crate::link::SimdLink;
use crate::room::async_trait_free::{Backend, Fut};
use crate::room::{Room, RoomCmd, RoomConfig};
use crate::sim_room::SimRoom;
use crate::token::{self, Claims};
use made_protocol::sim::MadeKind;

const HELLO_TIMEOUT: Duration = Duration::from_secs(10);
/// A socket that falls this far behind on frames is dropped rather than buffered.
const MAX_QUEUED: usize = 256;

/// The API, through the games bot's web client once it connects.
#[derive(Default)]
pub struct ApiBackend {
    web: RwLock<Option<Arc<tawks_bot::web_client::WebClient>>>,
}

impl ApiBackend {
    fn web(&self) -> Option<Arc<tawks_bot::web_client::WebClient>> {
        self.web.read().ok()?.clone()
    }
}

impl Backend for ApiBackend {
    fn room<'a>(&'a self, lobby_id: &'a str) -> Fut<'a, Option<RoomConfig>> {
        Box::pin(async move {
            let web = self.web()?;
            match web
                .query::<_, Option<RoomConfig>>(
                    "made_room",
                    &serde_json::json!({ "lobby_id": lobby_id }),
                )
                .await
            {
                Ok(config) => config,
                Err(error) => {
                    tracing::warn!(%error, "made_room failed");
                    None
                }
            }
        })
    }

    fn costume<'a>(&'a self, user_id: &'a str) -> Fut<'a, Option<serde_json::Value>> {
        Box::pin(async move {
            let web = self.web()?;
            tokio::time::timeout(
                Duration::from_secs(3),
                web.query::<_, serde_json::Value>("party_wardrobe_for", &user_id),
            )
            .await
            .ok()?
            .ok()
            .filter(|v| !v.is_null())
        })
    }

    fn log<'a>(
        &'a self,
        lobby_id: &'a str,
        activity_id: &'a str,
        version: i32,
        lines: Vec<(String, String)>,
    ) -> Fut<'a, ()> {
        Box::pin(async move {
            let Some(web) = self.web() else { return };
            let lines: Vec<_> = lines
                .into_iter()
                .map(|(level, text)| serde_json::json!({ "level": level, "text": text }))
                .collect();
            let body = serde_json::json!({ "lobby_id": lobby_id, "activity_id": activity_id, "version": version, "lines": lines });
            if let Err(error) = web.mutate::<_, ()>("made_log", &body).await {
                tracing::warn!(%error, "made_log failed");
            }
        })
    }

    fn award(&self, args: serde_json::Value) -> Fut<'_, ()> {
        Box::pin(async move {
            let Some(web) = self.web() else { return };
            if let Err(error) = web.mutate::<_, serde_json::Value>("game_award", &args).await {
                tracing::warn!(%error, "made game_award failed");
            }
        })
    }
}

struct Inner {
    secret: [u8; 32],
    ws_url: String,
    link: SimdLink,
    backend: Arc<dyn Backend>,
    api: Option<Arc<ApiBackend>>,
    rooms: DashMap<String, mpsc::UnboundedSender<RoomCmd>>,
    next_conn: AtomicU64,
}

#[derive(Clone)]
pub struct MadeServer {
    inner: Arc<Inner>,
}

fn signing_secret() -> [u8; 32] {
    match std::env::var("MADE_SECRET") {
        Ok(value) if !value.is_empty() => sha2::Sha256::digest(value.as_bytes()).into(),
        _ => rand::random(),
    }
}

impl MadeServer {
    /// `party_public_url` is the party socket's public address; the made
    /// relay sits beside it at `/party/made/ws`.
    pub fn from_env(party_public_url: &str) -> Self {
        let base = party_public_url
            .trim_end_matches('/')
            .trim_end_matches("/party/ws");
        let socket =
            std::env::var("MADE_SIMD_SOCKET").unwrap_or_else(|_| "/run/made-simd/simd.sock".into());
        let api = Arc::new(ApiBackend::default());
        Self::new(
            format!("{base}/party/made/ws"),
            signing_secret(),
            SimdLink::start(socket),
            api.clone(),
            Some(api),
        )
    }

    pub fn new(
        ws_url: String,
        secret: [u8; 32],
        link: SimdLink,
        backend: Arc<dyn Backend>,
        api: Option<Arc<ApiBackend>>,
    ) -> Self {
        MadeServer {
            inner: Arc::new(Inner {
                secret,
                ws_url,
                link,
                backend,
                api,
                rooms: DashMap::new(),
                next_conn: AtomicU64::new(1),
            }),
        }
    }

    pub fn set_bot(&self, web: Arc<tawks_bot::web_client::WebClient>) {
        if let Some(api) = &self.inner.api {
            if let Ok(mut slot) = api.web.write() {
                *slot = Some(web);
            }
        }
    }

    pub fn ws_url(&self) -> &str {
        &self.inner.ws_url
    }

    pub fn issue(&self, mut claims: Claims) -> String {
        claims.exp = token::now_unix() + token::TTL_SECS;
        token::sign(&claims, &self.inner.secret)
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/party/made/ws", get(ws_handler))
            .with_state(self.clone())
    }

    /// A lobby's room; a sim and a game in the same lobby are kept apart by kind.
    fn room(&self, lobby_id: &str, kind: MadeKind) -> mpsc::UnboundedSender<RoomCmd> {
        let key = match kind {
            MadeKind::Game => lobby_id.to_string(),
            MadeKind::Sim => format!("{lobby_id}#sim"),
        };
        if let Some(tx) = self.inner.rooms.get(&key) {
            if !tx.is_closed() {
                return tx.clone();
            }
        }
        let (tx, rx) = mpsc::unbounded_channel();
        self.inner.rooms.insert(key.clone(), tx.clone());
        let server = self.clone();
        let (link, backend, lobby) = (
            self.inner.link.clone(),
            self.inner.backend.clone(),
            lobby_id.to_string(),
        );
        tokio::spawn(async move {
            match kind {
                MadeKind::Game => Room::new(lobby, link, backend).run(rx).await,
                MadeKind::Sim => SimRoom::new(lobby, link, backend).run(rx).await,
            }
            server.inner.rooms.remove_if(&key, |_, tx| tx.is_closed());
        });
        tx
    }
}

async fn ws_handler(State(server): State<MadeServer>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.max_message_size(4096)
        .on_upgrade(move |socket| serve(server, socket))
}

fn send_now(msg: &RelayMsg) -> Message {
    Message::Text(serde_json::to_string(msg).unwrap_or_default())
}

async fn serve(server: MadeServer, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let hello = tokio::time::timeout(HELLO_TIMEOUT, async {
        while let Some(Ok(message)) = stream.next().await {
            if let Message::Text(raw) = message {
                return serde_json::from_str::<ClientMsg>(&raw).ok();
            }
        }
        None
    })
    .await
    .ok()
    .flatten();
    let claims = match hello {
        Some(ClientMsg::Hello { token: pass }) => {
            token::verify(&pass, &server.inner.secret, token::now_unix())
        }
        _ => None,
    };
    let Some(claims) = claims else {
        let _ = sink
            .send(send_now(&RelayMsg::Error {
                message: "pass_expired".into(),
                fatal: true,
            }))
            .await;
        return;
    };
    let conn = server.inner.next_conn.fetch_add(1, Ordering::Relaxed);
    let room = server.room(&claims.lobby_id, claims.kind);
    let (out, mut outbox) = mpsc::unbounded_channel::<String>();
    if room.send(RoomCmd::Join { conn, claims, out }).is_err() {
        return;
    }
    let writer = tokio::spawn(async move {
        while let Some(body) = outbox.recv().await {
            if outbox.len() > MAX_QUEUED {
                break;
            }
            let fatal = body.contains("\"fatal\":true");
            if sink.send(Message::Text(body)).await.is_err() || fatal {
                break;
            }
        }
        let _ = sink.close().await;
    });
    let mut oversized = 0u32;
    while let Some(Ok(message)) = stream.next().await {
        let raw = match message {
            Message::Text(raw) => raw,
            Message::Close(_) => break,
            _ => continue,
        };
        if raw.len() > MAX_CLIENT_MSG_BYTES {
            oversized += 1;
            if oversized > 20 {
                break;
            }
            continue;
        }
        match serde_json::from_str::<ClientMsg>(&raw) {
            Ok(msg) => {
                if room.send(RoomCmd::Client { conn, msg }).is_err() {
                    break;
                }
            }
            Err(_) => continue,
        }
        if writer.is_finished() {
            break;
        }
    }
    let _ = room.send(RoomCmd::Leave { conn });
    writer.abort();
}
