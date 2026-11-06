//! Serves an Outroads drive on its own for working on the client:
//! `cargo run -p outroads --example dev -- [port]`, then open the printed
//! links (one per driver) against the client dev server on port 5296.

use outroads::OutroadsServer;
use outroads::geocode::Geocoder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::args().nth(1).map(|p| p.parse()).transpose()?.unwrap_or(8796);
    let ws = format!("ws://127.0.0.1:{port}/party/outroads/ws");
    let server = OutroadsServer::new(*b"outroads-dev-secret-not-for-use!", ws.clone(), Geocoder::from_env());
    for (user, owner) in [("ash", true), ("bea", false), ("cal", false)] {
        let token = server.issue_pass("dev", user, user, "", owner);
        println!("{user}: http://localhost:5296/?ws={ws}&token={token}");
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, server.router()).await?;
    Ok(())
}
