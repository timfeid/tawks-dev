use chrono::{DateTime, Utc};

pub const DEFAULT_TOTAL_MS: i64 = 15_000;

pub fn parse_rfc3339_utc(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// (points_awarded, time_left_ms, time_taken_ms, speed_pct_0_100)
pub fn compute_time_scaled_points(
    max_points: i64,
    total_ms: i64,
    ends_at: Option<&str>,
    now: DateTime<Utc>,
) -> (i64, i64, i64, i64) {
    let mut time_left_ms = 0i64;
    if let Some(s) = ends_at {
        if let Some(end) = parse_rfc3339_utc(s) {
            time_left_ms = (end - now).num_milliseconds();
        }
    }

    let time_left_ms = time_left_ms.clamp(0, total_ms);
    let time_taken_ms = (total_ms - time_left_ms).clamp(0, total_ms);

    let ratio = (time_left_ms as f64) / (total_ms as f64);
    let points_awarded = ((max_points as f64) * ratio).ceil() as i64;
    let speed_pct = (ratio * 100.0).round().clamp(0.0, 100.0) as i64;

    (points_awarded, time_left_ms, time_taken_ms, speed_pct)
}

pub fn question_ends_at_ms(duration_ms: i64, now: DateTime<Utc>) -> String {
    (now + chrono::Duration::milliseconds(duration_ms)).to_rfc3339()
}
