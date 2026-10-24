use std::time::Duration;

use rand::rng;
use crate::types::UserResource;
use tokio_util::sync::CancellationToken;

pub mod activity;
pub mod bot;
pub mod notify;
pub mod sdk;
pub mod share;
pub mod services;
pub mod types;
pub mod web_client;

pub use activity::Activity;
pub use bot::*;
pub use sdk::*;
pub use workflow;

pub fn shutdown_token() -> CancellationToken {
    let token = CancellationToken::new();
    let t = token.clone();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        tokio::spawn(async move {
            use tokio::{select, signal};

            let mut sig_int = signal(SignalKind::interrupt()).expect("sigint handler");
            let mut sig_term = signal(SignalKind::terminate()).expect("sigterm handler");
            select! {
                _ = signal::ctrl_c() => {},
                _ = sig_int.recv() => {},
                _ = sig_term.recv() => {},
            }
            t.cancel();
        });
    }
    #[cfg(not(unix))]
    {
        tokio::spawn(async move {
            let _ = signal::ctrl_c().await;
            t.cancel();
        });
    }
    token
}

pub fn with_jitter(base: Duration) -> Duration {
    use rand::Rng;

    let mut rng = rng();
    let millis = base.as_millis() as u64;
    let jitter = ((millis as f64) * 0.1) as u64;
    let adj = rng.random_range(millis.saturating_sub(jitter)..=millis.saturating_add(jitter));
    Duration::from_millis(adj)
}

pub fn full_avatar_url(user: &UserResource) -> String {
    let ending = if user.avatar_url.starts_with("default/") {
        user.avatar_url.clone()
    } else {
        format!("{}/{}", user.id, user.avatar_url)
    };

    format!("https://cdn.tawks.app/avatars/{}", ending)
}
