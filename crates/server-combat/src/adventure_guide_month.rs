use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use shared::protocol::JournalMonth;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MonthlyCriteriaState {
    pub interval_start: i64,
    pub tracking_since: i64,
    pub counters: BTreeMap<u32, u64>,
    pub tracked: BTreeSet<u32>,
    pub ignored_login_achievements: BTreeSet<u32>,
}
impl MonthlyCriteriaState {
    pub(super) fn new(start: i64, now: i64) -> Self {
        Self {
            interval_start: start,
            tracking_since: now,
            counters: BTreeMap::new(),
            tracked: BTreeSet::new(),
            ignored_login_achievements: BTreeSet::new(),
        }
    }
    pub(super) fn reset_at(&mut self, start: i64, now: i64) {
        if self.interval_start != start {
            self.interval_start = start;
            self.tracking_since = now;
            self.counters.clear();
            self.ignored_login_achievements.clear();
        }
    }
}

/// World of Osso policy: UTC calendar months, starting at the existing daily reset hour.
pub(super) fn month_at(now: i64, hour: u8, tracking_since: i64) -> JournalMonth {
    assert!(hour < 24, "monthly reset hour must be 0-23");
    let shifted = chrono::DateTime::from_timestamp(now - i64::from(hour) * 3600, 0)
        .expect("valid realm clock");
    let year = shifted.year();
    let month = shifted.month();
    let start = Utc
        .with_ymd_and_hms(year, month, 1, u32::from(hour), 0, 0)
        .single()
        .expect("valid month start");
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let end = Utc
        .with_ymd_and_hms(next_year, next_month, 1, u32::from(hour), 0, 0)
        .single()
        .expect("valid month end");
    JournalMonth {
        year,
        month,
        name: start.format("%B").to_string(),
        start_unix: start.timestamp(),
        end_unix: end.timestamp(),
        now_unix: now,
        tracking_since,
    }
}
