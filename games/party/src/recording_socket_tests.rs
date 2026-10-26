//! A member asks for a recording: the recorder is handed a watch-only pass,
//! the browser it opens never takes a seat, and the room shows the recording
//! from request to live to stopped.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::server::{PartyConfig, PartyServer, RecorderStatus, SessionRecorder};

#[derive(Default)]
struct FakeRecorder {
    started: Mutex<Vec<(String, String, String, Value)>>,
    stopped: Mutex<Vec<String>>,
}

impl SessionRecorder for FakeRecorder {
    fn start(&self, match_id: &str, title: &str, recorded_by: &str, watch: Value) -> Result<(), String> {
        self.started.lock().unwrap().push((match_id.into(), title.into(), recorded_by.into(), watch));
        Ok(())
    }

    fn stop(&self, match_id: &str) {
        self.stopped.lock().unwrap().push(match_id.into());
    }
}

struct Client {
    socket: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    room: Value,
    spectating: bool,
}

impl Client {
    async fn open(port: u16, token: &str) -> Client {
        let (mut socket, _) = connect_async(format!("ws://127.0.0.1:{port}/party/ws")).await.unwrap();
        socket.send(Message::Text(json!({"type": "hello", "token": token}).to_string().into())).await.unwrap();
        let mut client = Client { socket, room: Value::Null, spectating: false };
        client.wait("a room", |c| c.room["members"].is_array()).await;
        client
    }

    async fn send(&mut self, msg: Value) {
        self.socket.send(Message::Text(msg.to_string().into())).await.unwrap();
    }

    async fn wait(&mut self, what: &str, done: impl Fn(&Client) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while tokio::time::Instant::now() < deadline {
            if done(self) {
                return;
            }
            let Ok(Some(Ok(Message::Text(text)))) = timeout(Duration::from_millis(500), self.socket.next()).await else {
                continue;
            };
            let msg: Value = serde_json::from_str(&text).unwrap();
            match msg["type"].as_str() {
                Some("room") => {
                    self.room = msg["room"].clone();
                    self.spectating = msg["spectating"] == true;
                }
                Some("error") => panic!("error: {}", msg["message"]),
                _ => {}
            }
        }
        panic!("never saw {what}; room {}", self.room);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_recording_runs_from_request_to_stop() {
    let port = 19500 + (std::process::id() % 1000) as u16;
    let server = PartyServer::start(PartyConfig {
        bind: format!("127.0.0.1:{port}").parse().unwrap(),
        public_url: format!("ws://127.0.0.1:{port}/party/ws"),
    })
    .await
    .unwrap();
    let recorder = Arc::new(FakeRecorder::default());
    server.set_recorder(recorder.clone());

    let token = server.issue_token_for("rec-room", "u0", "Player u0", "", Some(crate::coven::KEY), false);
    let mut player = Client::open(port, &token).await;
    player.send(json!({"type": "record", "on": true})).await;
    player.wait("the recording asked for", |c| c.room["recording"]["by"] == "Player u0").await;
    assert_eq!(player.room["recording"]["live"], false);

    let (match_id, title, by, watch) = recorder.started.lock().unwrap()[0].clone();
    assert_eq!((match_id.as_str(), title.as_str(), by.as_str()), ("rec-room", "Coven", "u0"));
    assert_eq!(watch["match_id"], "rec-room");

    let mut browser = Client::open(port, watch["token"].as_str().unwrap()).await;
    assert!(browser.spectating);
    browser.send(json!({"type": "add_bot"})).await;
    server.recorder_status("rec-room", RecorderStatus::Live);
    player.wait("the recording live", |c| c.room["recording"]["live"] == true).await;
    assert_eq!(player.room["members"].as_array().unwrap().len(), 1, "the recorder took no seat and added no bot");

    player.send(json!({"type": "record", "on": false})).await;
    player.wait("the recording stopped", |c| c.room["recording"].is_null()).await;
    assert_eq!(recorder.stopped.lock().unwrap().as_slice(), ["rec-room".to_string()]);
    drop(browser);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_recording_stops_when_the_last_person_leaves() {
    let port = 20500 + (std::process::id() % 1000) as u16;
    let server = PartyServer::start(PartyConfig {
        bind: format!("127.0.0.1:{port}").parse().unwrap(),
        public_url: format!("ws://127.0.0.1:{port}/party/ws"),
    })
    .await
    .unwrap();
    let recorder = Arc::new(FakeRecorder::default());
    server.set_recorder(recorder.clone());
    let mut player = Client::open(port, &server.issue_token("empty-room", "u0", "Player u0", "")).await;
    player.send(json!({"type": "record", "on": true})).await;
    player.wait("the recording asked for", |c| !c.room["recording"].is_null()).await;
    let watch = recorder.started.lock().unwrap()[0].3.clone();
    let _browser = Client::open(port, watch["token"].as_str().unwrap()).await;
    drop(player);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while recorder.stopped.lock().unwrap().is_empty() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(recorder.stopped.lock().unwrap().as_slice(), ["empty-room".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_guest_cannot_record() {
    let port = 21500 + (std::process::id() % 1000) as u16;
    let server = PartyServer::start(PartyConfig {
        bind: format!("127.0.0.1:{port}").parse().unwrap(),
        public_url: format!("ws://127.0.0.1:{port}/party/ws"),
    })
    .await
    .unwrap();
    let recorder = Arc::new(FakeRecorder::default());
    server.set_recorder(recorder.clone());
    let token = server.issue_token_for("guest-room", "g0", "Guest", "", None, true);
    let (mut socket, _) = connect_async(format!("ws://127.0.0.1:{port}/party/ws")).await.unwrap();
    socket.send(Message::Text(json!({"type": "hello", "token": token}).to_string().into())).await.unwrap();
    socket.send(Message::Text(json!({"type": "record", "on": true}).to_string().into())).await.unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let mut refused = None;
    while refused.is_none() && tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(Message::Text(text)))) = timeout(Duration::from_millis(500), socket.next()).await {
            let msg: Value = serde_json::from_str(&text).unwrap();
            if msg["type"] == "error" {
                refused = msg["message"].as_str().map(str::to_string);
            }
        }
    }
    assert_eq!(refused.as_deref(), Some("Sign in to record."));
    assert!(recorder.started.lock().unwrap().is_empty());
}
