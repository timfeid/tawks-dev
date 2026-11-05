use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use longtable::LongtableServer;
use longtable::store::{MemoryStore, Store};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn serve(store: Arc<dyn Store>) -> (LongtableServer, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/party/longtable/ws", listener.local_addr().unwrap());
    let server = LongtableServer::new(store, [7; 32], url.clone());
    let router = server.router();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (server, url)
}

async fn join(server: &LongtableServer, url: &str, user: &str) -> Socket {
    let (mut socket, _) = connect_async(url).await.unwrap();
    let token = server.issue_pass("lobby-1", user, user, "", false);
    socket
        .send(Message::text(json!({ "type": "hello", "token": token }).to_string()))
        .await
        .unwrap();
    socket
}

async fn next_table(socket: &mut Socket) -> Value {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let Message::Text(text) = message else { continue };
        let value: Value = serde_json::from_str(&text).unwrap();
        match value["type"].as_str() {
            Some("table") => return value["table"].clone(),
            Some("error") => panic!("error: {value}"),
            _ => {}
        }
    }
}

async fn act(socket: &mut Socket, action: Value) {
    socket
        .send(Message::text(json!({ "type": "act", "action": action }).to_string()))
        .await
        .unwrap();
}

/// Reads tables until one passes `done`.
async fn until(socket: &mut Socket, done: impl Fn(&Value) -> bool) -> Value {
    loop {
        let table = next_table(socket).await;
        if done(&table) {
            return table;
        }
    }
}

#[tokio::test]
async fn a_table_hides_gm_rolls_syncs_sheets_and_survives_a_restart() {
    let store: Arc<dyn Store> = Arc::new(MemoryStore::default());
    let (server, url) = serve(store.clone()).await;
    let mut gm = join(&server, &url, "gm").await;
    let first = next_table(&mut gm).await;
    assert_eq!(first["gm"], "gm");
    let mut pat = join(&server, &url, "pat").await;
    until(&mut pat, |t| t["people"].as_array().unwrap().len() == 2).await;

    act(
        &mut gm,
        json!({ "type": "roll", "formula": "1d20+7 ambush", "visibility": "gm" }),
    )
    .await;
    let gm_view = until(&mut gm, |t| !t["rolls"].as_array().unwrap().is_empty()).await;
    assert_eq!(gm_view["rolls"][0]["shown"], "open");
    let pat_view = until(&mut pat, |t| !t["rolls"].as_array().unwrap().is_empty()).await;
    assert_eq!(pat_view["rolls"][0]["shown"], "secret");
    assert!(!pat_view.to_string().contains("ambush"));

    act(&mut pat, json!({ "type": "sheet_create", "name": "Mira" })).await;
    let sheet = until(&mut gm, |t| !t["sheets"].as_array().unwrap().is_empty()).await["sheets"][0]["id"].clone();
    act(
        &mut gm,
        json!({ "type": "sheet_update", "id": sheet, "patch": { "hp": 3, "conditions": ["Prone"] } }),
    )
    .await;
    let seen = until(&mut pat, |t| t["sheets"][0]["hp"] == 3).await;
    assert_eq!(seen["sheets"][0]["conditions"], json!(["Prone"]));

    act(
        &mut gm,
        json!({ "type": "init_add_sheet", "sheet_id": sheet, "init": 14 }),
    )
    .await;
    act(&mut gm, json!({ "type": "init_add_npc", "name": "Wolf", "init": 9 })).await;
    act(&mut gm, json!({ "type": "init_sort" })).await;
    act(&mut gm, json!({ "type": "init_next" })).await;
    let order = until(&mut pat, |t| t["initiative"]["round"] == 1).await;
    assert_eq!(order["initiative"]["entries"][0]["name"], "Mira");
    assert_eq!(order["initiative"]["current"], order["initiative"]["entries"][0]["id"]);

    act(&mut pat, json!({ "type": "init_next" })).await;
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), pat.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let Message::Text(text) = message else { continue };
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["type"] == "error" {
            assert_eq!(value["message"], "Only the GM can do that");
            break;
        }
    }

    tokio::time::sleep(Duration::from_millis(1600)).await;
    let saved = store.load("lobby:lobby-1").await.unwrap().expect("the table was saved");
    assert_eq!(saved["gm"], "gm");

    let (restarted, url) = serve(store.clone()).await;
    let mut back = join(&restarted, &url, "pat").await;
    let table = next_table(&mut back).await;
    assert_eq!(table["gm"], "gm", "the GM seat outlives a restart");
    assert_eq!(table["sheets"][0]["hp"], 3);
    assert_eq!(table["initiative"]["round"], 1);
    assert_eq!(table["rolls"][0]["shown"], "secret");
}

#[tokio::test]
async fn a_forged_pass_is_turned_away() {
    let (_server, url) = serve(Arc::new(MemoryStore::default())).await;
    let other = LongtableServer::new(Arc::new(MemoryStore::default()), [9; 32], url.clone());
    let (mut socket, _) = connect_async(&url).await.unwrap();
    let token = other.issue_pass("lobby-1", "mallory", "Mallory", "", true);
    socket
        .send(Message::text(json!({ "type": "hello", "token": token }).to_string()))
        .await
        .unwrap();
    let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(message.to_text().unwrap().contains("pass_expired"));
}
