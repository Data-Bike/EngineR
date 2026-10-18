//! Time types: recorded (system history) and valid (business truth period).
//!
//! `RecordedTime` is the system history (append-only). `ValidPeriod` is the
//! business truth interval. Revocation creates a new record/status, never
//! erasing the past (docs/03 §3, ADR-004).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// System-recorded time (when the assertion was stored). Always UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordedTime(pub DateTime<Utc>);

impl RecordedTime {
    /// Now in UTC.
    #[must_use]
    pub fn now() -> Self {
        Self(Utc::now())
    }
}

/// Validity interval in the business domain. Open-ended upper bound means
/// "currently true".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidPeriod {
    pub valid_from: DateTime<Utc>,
    pub valid_to: Option<DateTime<Utc>>,
}

impl ValidPeriod {
    /// Build an open-ended period starting at `from`.
    #[must_use]
    pub fn open_from(from: DateTime<Utc>) -> Self {
        Self {
            valid_from: from,
            valid_to: None,
        }
    }

    /// Build a closed period `[from, to]`.
    #[must_use]
    pub fn closed(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        debug_assert!(to >= from, "valid_to must be >= valid_from");
        Self {
            valid_from: from,
            valid_to: Some(to),
        }
    }

    /// True if `instant` falls within this period.
    #[must_use]
    pub fn contains(&self, instant: DateTime<Utc>) -> bool {
        instant >= self.valid_from && self.valid_to.map_or(true, |to| instant <= to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn open_period_contains_future() {
        let p = ValidPeriod::open_from(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
        assert!(p.contains(Utc.with_ymd_and_hms(2026, 8, 3, 0, 0, 0).unwrap()));
    }

    #[test]
    fn closed_period_excludes_after() {
        let from = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let p = ValidPeriod::closed(from, to);
        assert!(!p.contains(Utc.with_ymd_and_hms(2026, 8, 3, 0, 0, 0).unwrap()));
    }
}
