#![cfg(feature = "tracing")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use tracing_subscriber::layer::SubscriberExt;

fn collector() -> (String, Arc<Mutex<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let seen = Arc::new(Mutex::new(String::new()));
    let sink = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = vec![0u8; 1 << 20];
            let mut total = 0;
            stream
                .set_read_timeout(Some(std::time::Duration::from_millis(300)))
                .ok();
            while let Ok(n) = stream.read(&mut buf[total..]) {
                if n == 0 {
                    break;
                }
                total += n;
            }
            sink.lock()
                .unwrap()
                .push_str(&String::from_utf8_lossy(&buf[..total]));
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}");
        }
    });
    (
        format!("http://0123456789abcdef0123456789abcdef@127.0.0.1:{port}/7"),
        seen,
    )
}

#[test]
fn sends_scrubbed_errors_and_logs() {
    assert!(tawks_telemetry::init_from_env("unset").is_none());

    let (dsn, seen) = collector();
    let guard = tawks_telemetry::init(tawks_telemetry::Config {
        dsn,
        service: "api".into(),
        release: Some("abc1234".into()),
        environment: "test".into(),
        server_name: Some("api-2".into()),
        replica: Some("2".into()),
    });
    let subscriber = tracing_subscriber::registry().with(tawks_telemetry::layer());
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!("session renewed with token=abc123secret");
        tracing::debug!("not sent at info");
        tracing::error!("lookup failed for tim@example.com");
    });
    drop(guard);

    let seen = seen.lock().unwrap().clone();
    assert!(seen.contains(r#""type":"log""#), "no log item in {seen}");
    assert!(seen.contains(r#""type":"event""#), "no event in {seen}");
    assert!(seen.contains("token=[redacted]"));
    assert!(seen.contains("lookup failed for [email]"));
    assert!(seen.contains(r#""service.name""#));
    assert!(seen.contains(r#""replica":"2""#));
    assert!(!seen.contains("abc123secret"));
    assert!(!seen.contains("tim@example.com"));
    assert!(!seen.contains("not sent at info"));
}
