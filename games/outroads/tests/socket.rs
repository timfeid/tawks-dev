use std::time::Duration;

use futures::{SinkExt, StreamExt};
use outroads::OutroadsServer;
use outroads::geocode::Geocoder;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn serve() -> (OutroadsServer, String) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}/party/outroads/ws", listener.local_addr().unwrap());
    let server = OutroadsServer::new([7; 32], url.clone(), Geocoder::new("http://127.0.0.1:9/".into()));
    let router = server.router();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (server, url)
}

async fn join(server: &OutroadsServer, url: &str, user: &str, owner: bool) -> Socket {
    let (mut socket, _) = connect_async(url).await.unwrap();
    let token = server.issue_pass("lobby-1", user, user, "", owner);
    send(&mut socket, json!({ "type": "hello", "token": token })).await;
    socket
}

async fn send(socket: &mut Socket, value: Value) {
    socket.send(Message::text(value.to_string())).await.unwrap();
}

async fn until(socket: &mut Socket, kind: &str, done: impl Fn(&Value) -> bool) -> Value {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("a message in time")
            .unwrap()
            .unwrap();
        let Message::Text(text) = message else { continue };
        let value: Value = serde_json::from_str(&text).unwrap();
        if value["type"] == kind && done(&value) {
            return value;
        }
    }
}

#[tokio::test]
async fn two_drivers_share_a_place_and_see_each_other_move() {
    let (server, url) = serve().await;
    let mut ash = join(&server, &url, "ash", true).await;
    let mut bea = join(&server, &url, "bea", false).await;
    until(&mut ash, "room", |v| {
        v["room"]["drivers"].as_array().unwrap().len() == 2
    })
    .await;

    send(
        &mut bea,
        json!({ "type": "act", "action": { "kind": "set_place", "place": { "name": "Nope", "lat": 1, "lon": 1 } } }),
    )
    .await;
    until(&mut bea, "error", |_| true).await;

    send(
        &mut ash,
        json!({ "type": "act", "action": { "kind": "set_place", "place": { "name": "Exchange Place", "lat": 40.7163, "lon": -74.0327 } } }),
    )
    .await;
    let room = until(&mut bea, "room", |v| v["room"]["epoch"] == 1).await;
    assert_eq!(room["room"]["place"]["name"], "Exchange Place");
    assert_eq!(room["room"]["host"], "ash");

    send(
        &mut ash,
        json!({ "type": "pose", "pose": { "x": 12.5, "z": -40.0, "h": 1.0, "s": 9.0, "st": 0.2, "f": 1 } }),
    )
    .await;
    let poses = until(&mut bea, "poses", |v| !v["cars"].as_array().unwrap().is_empty()).await;
    assert_eq!(poses["cars"][0]["id"], "ash");
    assert_eq!(poses["cars"][0]["x"], 12.5);

    send(&mut ash, json!({ "type": "honk" })).await;
    let honk = until(&mut bea, "honk", |_| true).await;
    assert_eq!(honk["id"], "ash");
}

#[tokio::test]
async fn a_forged_pass_is_turned_away() {
    let (_server, url) = serve().await;
    let (mut socket, _) = connect_async(&url).await.unwrap();
    send(&mut socket, json!({ "type": "hello", "token": "bad.token" })).await;
    let error = until(&mut socket, "error", |_| true).await;
    assert_eq!(error["message"], "pass_expired");
}
