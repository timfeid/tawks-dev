//! Serves Encore on its own for local client testing:
//! `cargo run -p encore --example dev -- /path/to/songs 8790`, then open the
//! printed client links (one per player) against the client dev server.

use encore::EncoreServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let songs = args.next().expect("usage: dev <songs dir> [port]");
    let port: u16 = args.next().map(|p| p.parse()).transpose()?.unwrap_or(8790);
    // SAFETY: set before any other thread reads the environment.
    unsafe {
        std::env::set_var("ENCORE_SONGS_DIR", &songs);
        std::env::set_var("ENCORE_LIBRARY_USERS", "dev1,dev2,dev3");
        std::env::set_var("ENCORE_SECRET", "encore-dev");
    }
    let server = EncoreServer::from_env(&format!("ws://127.0.0.1:{port}/party/ws"))
        .await?
        .ok_or_else(|| anyhow::anyhow!("no songs in {songs}"))?;
    for user in ["dev1", "dev2", "dev3"] {
        let token = server.issue_token("dev", user, user, "", false);
        println!(
            "{user}: ?ws={}&api={}&token={token}",
            server.ws_url(),
            server.api_url()
        );
    }
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    axum::serve(listener, server.router()).await?;
    Ok(())
}
