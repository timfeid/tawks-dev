//! Actual Rust bot proofs -> prepared Python service -> existing club store.
//! No production account, bot token or service is used.
use pucklab_bot::client::{ApiError, PuckLab};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
};

struct Fixture(Child);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
#[ignore = "requires PUCKLAB_BOT_FIXTURE, PUCKLAB_SOURCE and NHL_PUCKLAB_CATALOG"]
async fn linked_player_and_bot_share_the_actual_pack_inventory_and_lineup() {
    let script = std::env::var("PUCKLAB_BOT_FIXTURE").expect("owned Python fixture path");
    let mut fixture = Fixture(
        Command::new("python3")
            .arg(script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut input = fixture.0.stdin.take().unwrap();
    let mut output = BufReader::new(fixture.0.stdout.take().unwrap());
    let read = |output: &mut BufReader<std::process::ChildStdout>| -> Value {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    };
    let start = read(&mut output);
    let api = PuckLab::new(
        start["url"].as_str().unwrap(),
        "isolated-bot-secret-not-a-player-token".into(),
        1,
        1,
    )
    .unwrap();
    let actor = "pucklab-alice";
    let account = api.get(actor, "/v1/account", &[], None).await.unwrap();
    assert_eq!(account["account_id"], start["account"]);
    let hash = api.roster_hash(actor, "nhl-legacy").await.unwrap();
    api.post(actor,"/v1/icelab/action",&json!({"roster":"nhl-legacy","action":"found","name":"One shared club","kit":17,"key":"rust-found"}),Some(&hash)).await.unwrap();
    let query = [("roster", "nhl-legacy")];
    let state = api
        .get(actor, "/v1/icelab/state", &query, Some(&hash))
        .await
        .unwrap();
    let pack = state["packs"][0]["id"].clone();
    api.post(
        actor,
        "/v1/icelab/action",
        &json!({"roster":"nhl-legacy","action":"open","pack":pack,"key":"rust-open"}),
        Some(&hash),
    )
    .await
    .unwrap();
    api.post(
        actor,
        "/v1/icelab/action",
        &json!({"roster":"nhl-legacy","action":"auto","key":"rust-auto"}),
        Some(&hash),
    )
    .await
    .unwrap();
    let state = api
        .get(actor, "/v1/icelab/state", &query, Some(&hash))
        .await
        .unwrap();
    assert!(!state["cards"].as_array().unwrap().is_empty());
    assert!(!state["lineup"].as_array().unwrap().is_empty());
    writeln!(input, "{}", json!({"op":"inventory"})).unwrap();
    input.flush().unwrap();
    let stored = read(&mut output);
    assert_eq!(stored["account"], account["account_id"]);
    assert_eq!(
        stored["cards"].as_u64().unwrap() as usize,
        state["cards"].as_array().unwrap().len()
    );
    assert_eq!(stored["coins"], state["club"]["coins"]);
    writeln!(
        input,
        "{}",
        json!({"op":"rename","name":"Changed in PuckLab"})
    )
    .unwrap();
    input.flush().unwrap();
    assert_eq!(read(&mut output)["ok"], true);
    let refreshed = api
        .get(actor, "/v1/icelab/state", &query, Some(&hash))
        .await
        .unwrap();
    assert_eq!(refreshed["club"]["name"], "Changed in PuckLab");
    assert_eq!(refreshed["cards"], state["cards"]);
    assert_eq!(refreshed["club"]["coins"], state["club"]["coins"]);
    let other = api
        .get("pucklab-bob", "/v1/account", &[], None)
        .await
        .unwrap_err();
    assert!(
        other
            .downcast_ref::<ApiError>()
            .unwrap()
            .pucklab_link_required
    );
    writeln!(input, "{}", json!({"op":"unlink"})).unwrap();
    input.flush().unwrap();
    assert_eq!(read(&mut output)["linked"], false);
    let revoked = api.get(actor, "/v1/account", &[], None).await.unwrap_err();
    assert!(
        revoked
            .downcast_ref::<ApiError>()
            .unwrap()
            .pucklab_link_required
    );
    // EOF lets the fixture stop its own HTTP service and remove its temp data.
    drop(input);
    assert!(fixture.0.wait().unwrap().success());
}
