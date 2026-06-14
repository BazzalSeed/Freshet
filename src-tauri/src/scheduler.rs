use chrono::{DateTime, FixedOffset};

use crate::model::{CadenceMode, StreamDescription, StreamState, StreamStatus};

// ---------------------------------------------------------------------------
// is_due
// ---------------------------------------------------------------------------

/// Returns `true` if the stream should be refreshed on the current tick.
///
/// Rules:
/// - `Interval`:  due if `last_checked_at` is `None`; otherwise due iff
///   `(now − last_checked_at) >= interval_minutes`.
///   If `interval_minutes` is `None`, treat as not due (false).
/// - `OnLaunch` / `Manual`: never due on a tick (the tick doesn't fire these).
///
/// Parse-error policy:
/// - Unparseable `now` → `false` (safe, don't fire).
/// - Unparseable `last_checked_at` → treat as "never checked" (due for Interval).
pub fn is_due(
    mode: &CadenceMode,
    interval_minutes: Option<u64>,
    last_checked_at: Option<&str>,
    now: &str,
) -> bool {
    match mode {
        CadenceMode::Manual | CadenceMode::OnLaunch => false,
        CadenceMode::Interval => {
            let Some(interval) = interval_minutes else {
                return false;
            };

            // If `now` can't be parsed there's nothing sensible to do — don't fire.
            let Ok(now_dt) = parse_dt(now) else {
                return false;
            };

            let Some(last_str) = last_checked_at else {
                // Never checked → due.
                return true;
            };

            match parse_dt(last_str) {
                Err(_) => {
                    // Unparseable last_checked_at → treat as never checked.
                    true
                }
                Ok(last_dt) => {
                    let elapsed_minutes = (now_dt - last_dt).num_minutes();
                    elapsed_minutes >= interval as i64
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// runs_at_startup
// ---------------------------------------------------------------------------

/// Returns `true` for modes that the startup pass should refresh once, shortly
/// after launch.
///
/// `OnLaunch` and `Interval` streams refresh at startup; `Manual` streams do not.
pub fn runs_at_startup(mode: &CadenceMode) -> bool {
    matches!(mode, CadenceMode::OnLaunch | CadenceMode::Interval)
}

// ---------------------------------------------------------------------------
// due_for_tick
// ---------------------------------------------------------------------------

/// Returns `true` iff the stream is `Active` and `is_due` returns true.
///
/// Paused, Retired, or non-interval streams are never due on a tick.
pub fn due_for_tick(desc: &StreamDescription, state: &StreamState, now: &str) -> bool {
    desc.status == StreamStatus::Active
        && is_due(
            &desc.cadence.mode,
            desc.cadence.interval_minutes,
            state.last_checked_at.as_deref(),
            now,
        )
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn parse_dt(s: &str) -> Result<DateTime<FixedOffset>, chrono::ParseError> {
    DateTime::parse_from_rfc3339(s)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Cadence, StreamDescription, StreamState, StreamStatus};

    // A fixed "now" used throughout the test suite.
    const NOW: &str = "2026-06-14T12:00:00Z";

    /// Return an ISO-8601 timestamp that is `offset_minutes` minutes before `NOW`.
    fn mins_ago(offset_minutes: i64) -> String {
        use chrono::Duration;
        let now_dt = DateTime::parse_from_rfc3339(NOW).unwrap();
        let past = now_dt - Duration::minutes(offset_minutes);
        past.to_rfc3339()
    }

    // -----------------------------------------------------------------------
    // is_due — Interval mode
    // -----------------------------------------------------------------------

    #[test]
    fn interval_61_min_ago_is_due_with_60_min_interval() {
        let last = mins_ago(61);
        assert!(is_due(&CadenceMode::Interval, Some(60), Some(&last), NOW));
    }

    #[test]
    fn interval_10_min_ago_is_not_due_with_60_min_interval() {
        let last = mins_ago(10);
        assert!(!is_due(&CadenceMode::Interval, Some(60), Some(&last), NOW));
    }

    #[test]
    fn interval_exactly_60_min_ago_is_due_with_60_min_interval() {
        // Boundary: elapsed == interval → due.
        let last = mins_ago(60);
        assert!(is_due(&CadenceMode::Interval, Some(60), Some(&last), NOW));
    }

    #[test]
    fn interval_none_last_checked_is_due() {
        // Never checked → always due.
        assert!(is_due(&CadenceMode::Interval, Some(60), None, NOW));
    }

    #[test]
    fn interval_none_interval_minutes_is_not_due() {
        // No interval configured → never due on tick.
        assert!(!is_due(&CadenceMode::Interval, None, None, NOW));
    }

    #[test]
    fn interval_unparseable_now_is_not_due() {
        assert!(!is_due(
            &CadenceMode::Interval,
            Some(60),
            None,
            "not-a-date"
        ));
    }

    #[test]
    fn interval_unparseable_last_checked_treated_as_never_checked() {
        // Bad last_checked_at → treat as "never checked" → due.
        assert!(is_due(
            &CadenceMode::Interval,
            Some(60),
            Some("garbage"),
            NOW
        ));
    }

    // -----------------------------------------------------------------------
    // is_due — OnLaunch and Manual modes
    // -----------------------------------------------------------------------

    #[test]
    fn on_launch_is_never_due_on_tick() {
        // Regardless of last_checked_at or interval.
        assert!(!is_due(&CadenceMode::OnLaunch, Some(60), None, NOW));
        assert!(!is_due(
            &CadenceMode::OnLaunch,
            Some(60),
            Some(&mins_ago(999)),
            NOW
        ));
    }

    #[test]
    fn manual_is_never_due_on_tick() {
        assert!(!is_due(&CadenceMode::Manual, Some(60), None, NOW));
        assert!(!is_due(
            &CadenceMode::Manual,
            Some(60),
            Some(&mins_ago(999)),
            NOW
        ));
    }

    // -----------------------------------------------------------------------
    // runs_at_startup
    // -----------------------------------------------------------------------

    #[test]
    fn on_launch_runs_at_startup() {
        assert!(runs_at_startup(&CadenceMode::OnLaunch));
    }

    #[test]
    fn interval_runs_at_startup() {
        assert!(runs_at_startup(&CadenceMode::Interval));
    }

    #[test]
    fn manual_does_not_run_at_startup() {
        assert!(!runs_at_startup(&CadenceMode::Manual));
    }

    // -----------------------------------------------------------------------
    // due_for_tick
    // -----------------------------------------------------------------------

    fn make_desc(mode: CadenceMode, status: StreamStatus) -> StreamDescription {
        StreamDescription {
            id: "test-stream".into(),
            title: "Test".into(),
            topic: "testing".into(),
            sources: vec![],
            cadence: Cadence {
                mode,
                interval_minutes: Some(60),
            },
            status,
            created_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn make_state(last_checked_at: Option<String>) -> StreamState {
        StreamState {
            last_checked_at,
            ..StreamState::default()
        }
    }

    #[test]
    fn active_interval_stream_past_interval_is_due() {
        let desc = make_desc(CadenceMode::Interval, StreamStatus::Active);
        let state = make_state(Some(mins_ago(61)));
        assert!(due_for_tick(&desc, &state, NOW));
    }

    #[test]
    fn active_interval_stream_within_interval_is_not_due() {
        let desc = make_desc(CadenceMode::Interval, StreamStatus::Active);
        let state = make_state(Some(mins_ago(10)));
        assert!(!due_for_tick(&desc, &state, NOW));
    }

    #[test]
    fn paused_interval_stream_is_not_due() {
        let desc = make_desc(CadenceMode::Interval, StreamStatus::Paused);
        // Even with last_checked long in the past, a paused stream is never due.
        let state = make_state(Some(mins_ago(999)));
        assert!(!due_for_tick(&desc, &state, NOW));
    }

    #[test]
    fn active_manual_stream_is_not_due() {
        let desc = make_desc(CadenceMode::Manual, StreamStatus::Active);
        let state = make_state(None);
        assert!(!due_for_tick(&desc, &state, NOW));
    }

    #[test]
    fn active_interval_stream_never_checked_is_due() {
        let desc = make_desc(CadenceMode::Interval, StreamStatus::Active);
        let state = make_state(None);
        assert!(due_for_tick(&desc, &state, NOW));
    }
}
