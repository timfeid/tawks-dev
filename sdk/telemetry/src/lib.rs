//! Errors and log lines to GlitchTip (any Sentry-protocol server).
//!
//! Every message is scrubbed of secrets before it leaves the process, and
//! each process sends at most [`LOGS_PER_MINUTE`] log lines and
//! [`EVENTS_PER_MINUTE`] error events a minute.
//!
//! Environment, read by [`init_from_env`]:
//! - `GLITCHTIP_DSN`: nothing is sent without it.
//! - `GLITCHTIP_LOG_LEVEL`: lowest level sent as a log line (`info` by default).
//! - `GLITCHTIP_ENVIRONMENT`: `production` by default.
//! - `TAWKS_REPLICA`: copy number, reported as `<service>-<replica>`.
//! - `BUILD_SHA`: release, falling back to the commit the binary was built from.

use std::borrow::Cow;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

use regex::Regex;
use sentry::protocol::{Event, Log, LogAttribute, Value};

pub use sentry;

pub const LOGS_PER_MINUTE: u32 = 120;
pub const EVENTS_PER_MINUTE: u32 = 30;
const MAX_TEXT: usize = 2000;
/// Chatty targets whose lines would use up the per-minute budget.
const QUIET_TARGETS: [&str; 6] = [
    "sentry",
    "sqlx::query",
    "hyper",
    "h2",
    "rustls",
    "tungstenite",
];

static SECRETS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    [
        (r"eyJ[\w-]{8,}\.[\w-]{8,}\.[\w-]*", "[jwt]"),
        (r"(?i)\b(Bearer|Bot|Basic)\s+[\w.~+/=-]{8,}", "$1 [redacted]"),
        (
            r#"(?i)\b([\w-]*(?:token|secret|password|passwd|api[_-]?key|signature|credential|session|code|auth))(["']?\s*[:=]\s*["']?)[^\s"'&,;)}\]]+"#,
            "$1$2[redacted]",
        ),
        (r#"(/hooks/[\w-]+/)[^\s/?#"']+"#, "$1[redacted]"),
        (r"https://[0-9a-f]{32}@", "https://[dsn]@"),
        (r"(?i)\b(postgres(?:ql)?|nats|redis|amqp)://[^\s/@]+@", "$1://[redacted]@"),
        (r"[\w.+-]+@[\w-]+\.[\w.-]+", "[email]"),
    ]
    .into_iter()
    .map(|(pattern, replacement)| (Regex::new(pattern).expect("valid redaction pattern"), replacement))
    .collect()
});

pub struct Config {
    pub dsn: String,
    /// Shown as the log service and the `service` tag, e.g. `api` or `cards-bot`.
    pub service: String,
    pub release: Option<String>,
    pub environment: String,
    /// Reported host name; `None` keeps the SDK's default for events and
    /// leaves it off log lines (the desktop app runs on people's own machines).
    pub server_name: Option<String>,
    pub replica: Option<String>,
}

/// Starts reporting from `GLITCHTIP_DSN`; without it, returns `None` and
/// nothing changes. Keep the guard alive for the life of the process.
pub fn init_from_env(service: &str) -> Option<sentry::ClientInitGuard> {
    let dsn = env("GLITCHTIP_DSN")?;
    let replica = env("TAWKS_REPLICA");
    let server_name = Some(match &replica {
        Some(replica) => format!("{service}-{replica}"),
        None => service.to_owned(),
    });
    Some(init(Config {
        dsn,
        service: service.to_owned(),
        release: env("BUILD_SHA").or_else(|| option_env!("TAWKS_GIT_SHA").map(str::to_owned)),
        environment: env("GLITCHTIP_ENVIRONMENT").unwrap_or_else(|| "production".into()),
        server_name,
        replica,
    }))
}

pub fn init(config: Config) -> sentry::ClientInitGuard {
    // sentry pulls in aws-lc-rs next to the ring provider other crates enable, and rustls panics when it has to pick between them.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let keep_host = config.server_name.is_some();
    let service = config.service.clone();
    let mut options = sentry::ClientOptions::default();
    options.release = config.release.map(Cow::Owned);
    options.environment = Some(Cow::Owned(config.environment));
    options.server_name = config.server_name.map(Cow::Owned);
    options.send_default_pii = false;
    let events = Mutex::new(Budget::new(EVENTS_PER_MINUTE, Instant::now()));
    options.before_send = Some(Arc::new(move |event| {
        events
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .admit(Instant::now())?;
        Some(scrub_event(event))
    }));
    let logs = Mutex::new(Budget::new(LOGS_PER_MINUTE, Instant::now()));
    options.before_send_log = Some(Arc::new(move |log| {
        let dropped_before = logs
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .admit(Instant::now())?;
        Some(scrub_log(log, &service, keep_host, dropped_before))
    }));
    let guard = sentry::init((config.dsn, options));
    sentry::configure_scope(|scope| {
        scope.set_tag("service", &config.service);
        if let Some(replica) = &config.replica {
            scope.set_tag("replica", replica);
        }
    });
    guard
}

/// `GLITCHTIP_LOG_LEVEL`, `info` when unset or unreadable.
pub fn log_level() -> log::LevelFilter {
    env("GLITCHTIP_LOG_LEVEL")
        .and_then(|level| level.parse().ok())
        .unwrap_or(log::LevelFilter::Info)
}

pub fn is_active() -> bool {
    sentry::Hub::current()
        .client()
        .is_some_and(|client| client.is_enabled())
}

pub fn is_quiet_target(target: &str) -> bool {
    QUIET_TARGETS
        .iter()
        .any(|quiet| target == *quiet || target.starts_with(&format!("{quiet}::")))
}

pub fn redact(text: &str) -> String {
    let mut out = match text.char_indices().nth(MAX_TEXT) {
        Some((cut, _)) => format!("{}...", &text[..cut]),
        None => text.to_owned(),
    };
    for (pattern, replacement) in SECRETS.iter() {
        out = pattern.replace_all(&out, *replacement).into_owned();
    }
    out
}

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn scrub_event(mut event: Event<'static>) -> Event<'static> {
    if let Some(message) = &mut event.message {
        *message = redact(message);
    }
    if let Some(entry) = &mut event.logentry {
        entry.message = redact(&entry.message);
        for param in &mut entry.params {
            scrub_value(param);
        }
    }
    for exception in &mut event.exception.values {
        if let Some(value) = &mut exception.value {
            *value = redact(value);
        }
    }
    for crumb in &mut event.breadcrumbs.values {
        if let Some(message) = &mut crumb.message {
            *message = redact(message);
        }
        for value in crumb.data.values_mut() {
            scrub_value(value);
        }
    }
    for value in event.extra.values_mut() {
        scrub_value(value);
    }
    event
}

fn scrub_log(mut log: Log, service: &str, keep_host: bool, dropped_before: u32) -> Log {
    log.body = redact(&log.body);
    log.attributes.remove("user.name");
    log.attributes.remove("user.email");
    if !keep_host {
        log.attributes.remove("server.address");
    }
    for (key, LogAttribute(value)) in log.attributes.iter_mut() {
        if !key.starts_with("sentry.") || key.starts_with("sentry.message.parameter") {
            scrub_value(value);
        }
    }
    log.attributes
        .insert("service.name".into(), LogAttribute(Value::from(service)));
    if dropped_before > 0 {
        log.attributes.insert(
            "tawks.dropped_before".into(),
            LogAttribute(Value::from(dropped_before)),
        );
    }
    log
}

fn scrub_value(value: &mut Value) {
    match value {
        Value::String(text) => *text = redact(text),
        Value::Array(items) => items.iter_mut().for_each(scrub_value),
        Value::Object(map) => map.values_mut().for_each(scrub_value),
        _ => {}
    }
}

struct Budget {
    per_minute: u32,
    window_start: Instant,
    sent: u32,
    dropped: u32,
}

impl Budget {
    fn new(per_minute: u32, now: Instant) -> Self {
        Self {
            per_minute,
            window_start: now,
            sent: 0,
            dropped: 0,
        }
    }

    /// Admits one item and returns how many were dropped in the previous minute.
    fn admit(&mut self, now: Instant) -> Option<u32> {
        let mut dropped_before = 0;
        if now.duration_since(self.window_start) >= Duration::from_secs(60) {
            dropped_before = self.dropped;
            *self = Self::new(self.per_minute, now);
        }
        if self.sent >= self.per_minute {
            self.dropped += 1;
            return None;
        }
        self.sent += 1;
        Some(dropped_before)
    }
}

#[cfg(feature = "tracing")]
pub use tracing_layer::layer;

#[cfg(feature = "tracing")]
mod tracing_layer {
    use sentry::integrations::tracing::{EventFilter, SentryLayer};
    use tracing_core::{Level, Subscriber};
    use tracing_subscriber::registry::LookupSpan;

    /// Errors become issues and log lines, lines at [`super::log_level`] and up
    /// become log lines. `None` when reporting is off, so it costs nothing.
    pub fn layer<S>() -> Option<SentryLayer<S>>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        if !super::is_active() {
            return None;
        }
        let min = super::log_level();
        Some(
            sentry::integrations::tracing::layer()
                .span_filter(|_| false)
                .event_filter(move |metadata| {
                    if super::is_quiet_target(metadata.target()) {
                        return EventFilter::Ignore;
                    }
                    let level = match *metadata.level() {
                        Level::ERROR => log::Level::Error,
                        Level::WARN => log::Level::Warn,
                        Level::INFO => log::Level::Info,
                        Level::DEBUG => log::Level::Debug,
                        Level::TRACE => log::Level::Trace,
                    };
                    let mut filter = match level {
                        log::Level::Error => EventFilter::Event,
                        log::Level::Warn | log::Level::Info => EventFilter::Breadcrumb,
                        _ => EventFilter::Ignore,
                    };
                    if level <= min {
                        filter |= EventFilter::Log;
                    }
                    filter
                }),
        )
    }
}

#[cfg(feature = "log")]
pub use log_logger::install_logger;

#[cfg(feature = "log")]
mod log_logger {
    use sentry::integrations::log::{LogFilter, SentryLogger};

    /// Installs `inner` as the global logger, wrapped so errors become issues
    /// and lines at [`super::log_level`] and up become log lines.
    pub fn install_logger(
        inner: Box<dyn log::Log>,
        inner_level: log::LevelFilter,
    ) -> Result<(), log::SetLoggerError> {
        if !super::is_active() {
            log::set_boxed_logger(inner)?;
            log::set_max_level(inner_level);
            return Ok(());
        }
        let min = super::log_level();
        let logger = SentryLogger::with_dest(inner).filter(move |metadata| {
            if super::is_quiet_target(metadata.target()) {
                return LogFilter::Ignore;
            }
            let mut filter = match metadata.level() {
                log::Level::Error => LogFilter::Exception,
                log::Level::Warn | log::Level::Info => LogFilter::Breadcrumb,
                _ => LogFilter::Ignore,
            };
            if metadata.level() <= min {
                filter |= LogFilter::Log;
            }
            filter
        });
        log::set_boxed_logger(Box::new(logger))?;
        log::set_max_level(inner_level.max(min));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets() {
        let jwt = "eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJVU1ItMSJ9.c2lnbmF0dXJlLXNpZ25hdHVyZQ";
        assert_eq!(redact(&format!("refresh with {jwt}")), "refresh with [jwt]");
        assert_eq!(
            redact("Authorization: Bearer abcdefgh12345"),
            "Authorization: Bearer [redacted]"
        );
        assert_eq!(
            redact("GET /file?X-Amz-Signature=deadbeef&x=1 token=abc123"),
            "GET /file?X-Amz-Signature=[redacted]&x=1 token=[redacted]"
        );
        assert_eq!(
            redact("POST /hooks/glitchtip/s3cr3t ok"),
            "POST /hooks/glitchtip/[redacted] ok"
        );
        assert_eq!(
            redact(concat!("connect postgres", "://postgres:hunter2@tawks-postgres:5432/talky")),
            "connect postgres://[redacted]@tawks-postgres:5432/talky"
        );
        assert_eq!(redact("mail tim@example.com"), "mail [email]");
        assert_eq!(
            redact("installer finished: exit status 0"),
            "installer finished: exit status 0"
        );
    }

    #[test]
    fn truncates_long_text() {
        let long = "é".repeat(MAX_TEXT + 10);
        assert_eq!(redact(&long).chars().count(), MAX_TEXT + 3);
    }

    #[test]
    fn caps_items_per_minute() {
        let start = Instant::now();
        let mut budget = Budget::new(3, start);
        for _ in 0..3 {
            assert_eq!(budget.admit(start), Some(0));
        }
        assert_eq!(budget.admit(start), None);
        assert_eq!(budget.admit(start), None);
        assert_eq!(budget.admit(start + Duration::from_secs(61)), Some(2));
    }

    #[test]
    fn scrubs_log_lines() {
        let mut log = Log {
            level: sentry::protocol::LogLevel::Info,
            body: "token=abc123".into(),
            trace_id: None,
            timestamp: std::time::SystemTime::now(),
            severity_number: None,
            attributes: Default::default(),
        };
        log.attributes
            .insert("user.name".into(), LogAttribute(Value::from("dazed")));
        log.attributes
            .insert("server.address".into(), LogAttribute(Value::from("pc")));
        log.attributes
            .insert("url".into(), LogAttribute(Value::from("/x?code=xyz")));
        let log = scrub_log(log, "api", false, 4);
        assert_eq!(log.body, "token=[redacted]");
        assert!(!log.attributes.contains_key("user.name"));
        assert!(!log.attributes.contains_key("server.address"));
        assert_eq!(log.attributes["url"].0, Value::from("/x?code=[redacted]"));
        assert_eq!(log.attributes["service.name"].0, Value::from("api"));
        assert_eq!(log.attributes["tawks.dropped_before"].0, Value::from(4));
    }

    #[test]
    fn quiet_targets_match_whole_segments() {
        assert!(is_quiet_target("sqlx::query"));
        assert!(is_quiet_target("hyper::proto::h1"));
        assert!(!is_quiet_target("hyperlink"));
        assert!(!is_quiet_target("tawks_api::router"));
    }
}
