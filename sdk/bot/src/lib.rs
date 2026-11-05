use std::time::Duration;

use rand::rng;
use crate::types::UserResource;
use tokio_util::sync::CancellationToken;

pub mod activity;
pub mod game;
pub mod channel_action;
pub mod command;
pub mod daily;
pub mod bot;
pub mod notify;
pub mod outage;
mod run;
pub mod sdk;
pub mod scoreboard;
pub mod share;
pub mod store;
pub mod ticker;
pub mod services;
pub mod types;
pub mod web_client;

pub use activity::Activity;
pub use command::Command;
pub use daily::DailyFinishes;
pub use store::Product;
pub use bot::*;
pub use outage::connection_failed;
pub use run::run;
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
            let _ = tokio::signal::ctrl_c().await;
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
    let stored = user.avatar_url.trim();
    let ending = if stored.is_empty() {
        "default/default-1.png".to_string()
    } else if stored.starts_with("http://") || stored.starts_with("https://") {
        return stored.to_string();
    } else if stored.starts_with("default/") {
        stored.to_string()
    } else {
        format!(
            "{}/{}",
            encode_path_segment(&user.id),
            encode_path_segment(stored)
        )
    };

    format!("https://cdn.tawks.app/avatars/{}", ending)
}

fn encode_path_segment(segment: &str) -> String {
    segment
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod avatar_url_tests {
    use super::*;

    fn user(avatar_url: &str) -> UserResource {
        UserResource {
            id: "USR-1".into(),
            username: "a".into(),
            display_name: "A".into(),
            avatar_url: avatar_url.into(),
            db_score: 0,
            is_bot: false,
        }
    }

    #[test]
    fn avatar_urls_never_point_at_a_broken_path() {
        let cdn = "https://cdn.tawks.app/avatars";
        assert_eq!(
            full_avatar_url(&user("abc.png")),
            format!("{cdn}/USR-1/abc.png")
        );
        assert_eq!(
            full_avatar_url(&user("my pic #2.png")),
            format!("{cdn}/USR-1/my%20pic%20%232.png")
        );
        assert_eq!(
            full_avatar_url(&user(" default/default-3.png ")),
            format!("{cdn}/default/default-3.png")
        );
        assert_eq!(
            full_avatar_url(&user("https://x.test/a.png")),
            "https://x.test/a.png"
        );
        assert_eq!(
            full_avatar_url(&user("")),
            format!("{cdn}/default/default-1.png")
        );
    }
}
