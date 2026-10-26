//! Four people at one table over real sockets: the room deals Coven, every
//! seat gets its own hand, and a turn goes round.

use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

use crate::server::{PartyConfig, PartyServer};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Client {
    socket: Socket,
    user: String,
    room: Value,
    coven: Value,
}

impl Client {
    async fn send(&mut self, msg: Value) {
        self.socket.send(Message::Text(msg.to_string().into())).await.unwrap();
    }

    /// Reads until `done` is satisfied by the latest room or coven view.
    async fn wait(&mut self, what: &str, done: impl Fn(&Value, &Value) -> bool) {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while tokio::time::Instant::now() < deadline {
            if done(&self.room, &self.coven) {
                return;
            }
            let Ok(Some(Ok(Message::Text(text)))) = timeout(Duration::from_millis(500), self.socket.next()).await else {
                continue;
            };
            let msg: Value = serde_json::from_str(&text).unwrap();
            match msg["type"].as_str() {
                Some("room") => self.room = msg["room"].clone(),
                Some("table") => self.coven = msg["view"].clone(),
                Some("error") => panic!("{} got an error: {}", self.user, msg["message"]),
                _ => {}
            }
        }
        panic!("{} never saw {what}; room phase {} coven phase {}", self.user, self.room["phase"], self.coven["phase"]);
    }
}

async fn connect(server: &PartyServer, port: u16, user: &str) -> Client {
    let token = server.issue_token("table-1", user, &format!("Player {user}"), "");
    let (mut socket, _) = connect_async(format!("ws://127.0.0.1:{port}/party/ws")).await.unwrap();
    socket.send(Message::Text(json!({"type": "hello", "token": token}).to_string().into())).await.unwrap();
    let mut client = Client { socket, user: user.to_string(), room: Value::Null, coven: Value::Null };
    client.wait("a welcome and room", |room, _| room["members"].is_array()).await;
    client
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn four_people_deal_a_game_over_sockets() {
    let port = 18500 + (std::process::id() % 1000) as u16;
    let server = PartyServer::start(PartyConfig {
        bind: format!("127.0.0.1:{port}").parse().unwrap(),
        public_url: format!("ws://127.0.0.1:{port}/party/ws"),
    })
    .await
    .unwrap();
    let mut host = connect(&server, port, "u0").await;
    host.send(json!({"type": "choose_game", "key": "coven"})).await;
    host.wait("coven chosen", |room, _| room["minigame"] == "coven" && room["max_seats"] == 12).await;
    let mut others = Vec::new();
    for user in ["u1", "u2", "u3", "u4"] {
        others.push(connect(&server, port, user).await);
    }
    host.wait("five seated", |room, _| room["members"].as_array().map(|m| m.len()) == Some(5)).await;
    host.send(json!({"type": "start"})).await;
    host.wait("the deal", |room, coven| room["phase"]["name"] == "table" && coven["hand"].is_array()).await;
    for other in &mut others {
        other.wait("their own hand", |_, coven| coven["hand"].is_array()).await;
    }
    let mut all: Vec<&mut Client> = std::iter::once(&mut host).chain(others.iter_mut()).collect();
    let mut seats: Vec<i64> = all.iter().map(|c| c.coven["me"].as_i64().unwrap()).collect();
    seats.sort_unstable();
    assert_eq!(seats, vec![0, 1, 2, 3, 4]);
    for client in &all {
        assert_eq!(client.coven["hand"].as_array().unwrap().len(), crate::coven::HAND_AT_START);
        assert_eq!(client.coven["phase"]["kind"], "dawn");
        let mine = client.coven["me"].as_u64().unwrap() as usize;
        for (i, player) in client.coven["players"].as_array().unwrap().iter().enumerate() {
            for verdict in player["verdicts"].as_array().unwrap() {
                assert_eq!(verdict["card"].is_string(), i == mine, "only your own cards are known");
            }
        }
    }
    let witches: Vec<usize> = all
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.coven["witches"].as_array().unwrap().is_empty())
        .map(|(i, _)| i)
        .collect();
    let seen: Vec<String> = all.iter().map(|c| format!("{}={} witches={}", c.user, c.coven["me"], c.coven["witches"])).collect();
    assert!(!witches.is_empty(), "nobody knows they are a witch: {seen:?}");
    for i in witches {
        all[i].send(json!({"type": "table", "action": {"type": "place_familiar", "target": 0}})).await;
    }
    for client in all.iter_mut() {
        client.wait("play to begin", |_, coven| coven["phase"]["kind"] == "turn").await;
    }
    let turn = all[0].coven["phase"]["seat"].as_i64().unwrap();
    let mover = all.iter_mut().find(|c| c.coven["me"].as_i64() == Some(turn)).unwrap();
    mover.send(json!({"type": "table", "action": {"type": "draw"}})).await;
    mover.wait("the draw to land", |_, coven| coven["phase"]["kind"] != "turn" || coven["phase"]["seat"] != turn).await;
    let watching = all.iter_mut().find(|c| c.coven["me"].as_i64() != Some(turn)).unwrap();
    watching.wait("the turn to move on", |_, coven| coven["phase"]["kind"] != "turn" || coven["phase"]["seat"] != turn).await;
    let drew = watching.coven["players"][turn as usize]["hand_size"].as_u64().unwrap() > crate::coven::HAND_AT_START as u64;
    let interrupted = matches!(watching.coven["phase"]["kind"].as_str(), Some("conspiracy") | Some("reveal") | Some("night"));
    assert!(drew || interrupted, "the draw never showed: {}", watching.coven["phase"]);
}
