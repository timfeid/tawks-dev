//! Serves a Longtable table on its own for working on the client:
//! `cargo run -p longtable --example dev -- [port]`, then open the printed
//! links (one per player) against the client dev server on port 5293.
//! Tables live in memory unless `LONGTABLE_DATABASE_URL` is set.

use std::sync::Arc;

use longtable::LongtableServer;
use longtable::store::{MemoryStore, PgStore, Store};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::args().nth(1).map(|p| p.parse()).transpose()?.unwrap_or(8791);
    let store: Arc<dyn Store> = match std::env::var("LONGTABLE_DATABASE_URL") {
        Ok(url) if !url.is_empty() => Arc::new(PgStore::connect(&url).await?),
        _ => Arc::new(MemoryStore::default()),
    };
    let ws = format!("ws://127.0.0.1:{port}/party/longtable/ws");
    let server = LongtableServer::new(store, *b"longtable-dev-secret-not-for-use", ws.clone());
    for (user, owner) in [("gm", true), ("ash", false), ("bea", false)] {
        let token = server.issue_pass("dev", user, user, "", owner);
        println!("{user}: http://localhost:5293/?ws={ws}&token={token}");
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, server.router()).await?;
    Ok(())
}
