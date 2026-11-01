use std::fmt::Display;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// How long a bot may fail to reach tawks before it counts as an outage.
/// Deploys restart the API and events servers well within this.
pub const CONNECTION_GRACE: Duration = Duration::from_secs(120);

static CONNECTION: Mutex<Outage> = Mutex::new(Outage::new(CONNECTION_GRACE, 1));

/// Tracks a run of failures so retried, short-lived ones log as warnings
/// and only one that lasts reports as an error.
#[derive(Debug)]
pub struct Outage {
    grace: Duration,
    min_failures: u32,
    since: Option<Instant>,
    failures: u32,
    escalated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Transient,
    /// The failure that turned this run into an outage; reported once per outage.
    Persistent,
    /// Still failing after the outage was reported.
    Ongoing,
}

impl Outage {
    pub const fn new(grace: Duration, min_failures: u32) -> Self {
        Self {
            grace,
            min_failures,
            since: None,
            failures: 0,
            escalated: false,
        }
    }

    pub fn fail(&mut self, now: Instant) -> Severity {
        let since = *self.since.get_or_insert(now);
        self.failures += 1;
        if self.escalated {
            return Severity::Ongoing;
        }
        if self.failures >= self.min_failures && now.duration_since(since) >= self.grace {
            self.escalated = true;
            return Severity::Persistent;
        }
        Severity::Transient
    }

    /// Clears the run; returns how long it lasted and how many failures it
    /// had when there was one.
    pub fn recover(&mut self, now: Instant) -> Option<(Duration, u32)> {
        let since = self.since.take()?;
        let failures = std::mem::take(&mut self.failures);
        self.escalated = false;
        Some((now.duration_since(since), failures))
    }

    pub fn down_for(&self, now: Instant) -> Duration {
        self.since
            .map_or(Duration::ZERO, |since| now.duration_since(since))
    }
}

/// Logs a failed attempt to reach tawks (client setup, registration, the
/// events socket) that the caller is about to retry: a warning until the bot
/// has been unable to connect for [`CONNECTION_GRACE`], then one error.
pub fn connection_failed(error: &dyn Display, context: &str) {
    let now = Instant::now();
    let (severity, down_for) = {
        let mut outage = CONNECTION.lock().unwrap_or_else(|p| p.into_inner());
        (outage.fail(now), outage.down_for(now).as_secs())
    };
    match severity {
        Severity::Persistent => tracing::error!(
            %error,
            context,
            down_for_secs = down_for,
            "bot has been unable to connect to tawks for over two minutes"
        ),
        Severity::Transient | Severity::Ongoing => {
            tracing::warn!(%error, down_for_secs = down_for, "{context}; will retry")
        }
    }
}

pub(crate) fn connection_restored() {
    let recovered = CONNECTION
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .recover(Instant::now());
    if let Some((down_for, failures)) = recovered {
        tracing::info!(
            down_for_secs = down_for.as_secs(),
            failures,
            "reconnected to tawks"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalates_once_after_the_grace_period() {
        let start = Instant::now();
        let mut outage = Outage::new(Duration::from_secs(120), 1);
        assert_eq!(outage.fail(start), Severity::Transient);
        assert_eq!(
            outage.fail(start + Duration::from_secs(60)),
            Severity::Transient
        );
        assert_eq!(
            outage.fail(start + Duration::from_secs(121)),
            Severity::Persistent
        );
        assert_eq!(
            outage.fail(start + Duration::from_secs(150)),
            Severity::Ongoing
        );
        assert_eq!(
            outage.recover(start + Duration::from_secs(160)),
            Some((Duration::from_secs(160), 4))
        );
        assert_eq!(outage.recover(start + Duration::from_secs(170)), None);
        assert_eq!(
            outage.fail(start + Duration::from_secs(200)),
            Severity::Transient
        );
    }

    #[test]
    fn needs_enough_failures_as_well_as_time() {
        let start = Instant::now();
        let mut outage = Outage::new(Duration::from_secs(60), 3);
        assert_eq!(outage.fail(start), Severity::Transient);
        assert_eq!(
            outage.fail(start + Duration::from_secs(300)),
            Severity::Transient
        );
        assert_eq!(
            outage.fail(start + Duration::from_secs(301)),
            Severity::Persistent
        );
    }

    #[test]
    fn recovery_restarts_the_clock() {
        let start = Instant::now();
        let mut outage = Outage::new(Duration::from_secs(120), 1);
        outage.fail(start);
        outage.recover(start + Duration::from_secs(30));
        assert_eq!(
            outage.fail(start + Duration::from_secs(130)),
            Severity::Transient
        );
    }
}
