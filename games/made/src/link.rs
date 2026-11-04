//! One connection to made-simd, shared by every room. It reconnects on its
//! own; while it is down, rooms are told their round stopped.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use made_protocol::simd::{SimdEvent, SimdRequest};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Clone)]
pub struct SimdLink {
    requests: mpsc::UnboundedSender<SimdRequest>,
    rooms: Arc<DashMap<String, mpsc::UnboundedSender<SimdEvent>>>,
}

fn room_of(event: &SimdEvent) -> Option<&str> {
    match event {
        SimdEvent::Opened { room, .. }
        | SimdEvent::Frame { room, .. }
        | SimdEvent::End { room, .. }
        | SimdEvent::Log { room, .. }
        | SimdEvent::Failed { room, .. } => Some(room),
        _ => None,
    }
}

impl SimdLink {
    pub fn start(path: String) -> Self {
        let (requests, mut inbox) = mpsc::unbounded_channel::<SimdRequest>();
        let rooms: Arc<DashMap<String, mpsc::UnboundedSender<SimdEvent>>> = Arc::default();
        let link = SimdLink {
            requests,
            rooms: rooms.clone(),
        };
        tokio::spawn(async move {
            let mut backoff = Duration::from_millis(250);
            loop {
                let stream = match UnixStream::connect(&path).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        warn!(%error, path, "made-simd is not reachable");
                        // Requests made while it is down fail their rooms now rather than later.
                        while let Ok(request) = inbox.try_recv() {
                            if let SimdRequest::Open { room, .. } = request {
                                if let Some(tx) = rooms.get(&room) {
                                    let _ = tx.send(SimdEvent::Failed {
                                        room: room.clone(),
                                        message: "the game server is not running".into(),
                                    });
                                }
                            }
                        }
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(5));
                        continue;
                    }
                };
                backoff = Duration::from_millis(250);
                info!(path, "connected to made-simd");
                let (read, mut write) = stream.into_split();
                let mut lines = BufReader::new(read).lines();
                loop {
                    tokio::select! {
                        request = inbox.recv() => {
                            let Some(request) = request else { return };
                            let mut line = serde_json::to_vec(&request).unwrap_or_default();
                            line.push(b'\n');
                            if write.write_all(&line).await.is_err() {
                                break;
                            }
                        }
                        line = lines.next_line() => {
                            let Ok(Some(line)) = line else { break };
                            let Ok(event) = serde_json::from_str::<SimdEvent>(&line) else { continue };
                            if let Some(room) = room_of(&event) {
                                if let Some(tx) = rooms.get(room) {
                                    let _ = tx.send(event);
                                }
                            }
                        }
                    }
                }
                warn!("lost made-simd; telling its rooms");
                for entry in rooms.iter() {
                    let _ = entry.value().send(SimdEvent::Failed {
                        room: entry.key().clone(),
                        message: "the game server restarted".into(),
                    });
                }
            }
        });
        link
    }

    pub fn subscribe(&self, room: &str) -> mpsc::UnboundedReceiver<SimdEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.rooms.insert(room.to_string(), tx);
        rx
    }

    pub fn unsubscribe(&self, room: &str) {
        self.rooms.remove(room);
        let _ = self.requests.send(SimdRequest::Close {
            room: room.to_string(),
        });
    }

    pub fn send(&self, request: SimdRequest) {
        let _ = self.requests.send(request);
    }
}
