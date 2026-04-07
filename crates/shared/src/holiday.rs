use serde::{Deserialize, Serialize};

/// A month-day pair for recurring annual events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonthDay {
    pub month: u8,
    pub day: u8,
}

impl MonthDay {
    pub const fn new(month: u8, day: u8) -> Self {
        Self { month, day }
    }

    /// Ordinal day of year (approximate, ignoring leap years).
    /// Used for range checks that may wrap around Dec→Jan.
    fn ordinal(self) -> u16 {
        const MONTH_OFFSETS: [u16; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
        let m = (self.month.saturating_sub(1) as usize).min(11);
        MONTH_OFFSETS[m] + self.day as u16
    }
}

/// A date range for a seasonal event (recurring annually).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventDateRange {
    pub start: MonthDay,
    pub end: MonthDay,
}

impl EventDateRange {
    pub const fn new(start: MonthDay, end: MonthDay) -> Self {
        Self { start, end }
    }

    /// Whether a given month/day falls within this event's date range.
    /// Handles year-wrapping (e.g. Dec 15 – Jan 2).
    pub fn contains(&self, date: MonthDay) -> bool {
        let d = date.ordinal();
        let s = self.start.ordinal();
        let e = self.end.ordinal();
        if s <= e {
            d >= s && d <= e
        } else {
            // Wraps around year boundary
            d >= s || d <= e
        }
    }
}

/// A holiday event definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HolidayEvent {
    pub id: u32,
    pub name: String,
    pub date_range: EventDateRange,
}

// -- Built-in Events --

pub fn lunar_festival() -> HolidayEvent {
    HolidayEvent {
        id: 1,
        name: "Lunar Festival".into(),
        date_range: EventDateRange::new(MonthDay::new(1, 23), MonthDay::new(2, 5)),
    }
}

pub fn love_is_in_the_air() -> HolidayEvent {
    HolidayEvent {
        id: 2,
        name: "Love is in the Air".into(),
        date_range: EventDateRange::new(MonthDay::new(2, 7), MonthDay::new(2, 21)),
    }
}

pub fn noblegarden() -> HolidayEvent {
    HolidayEvent {
        id: 3,
        name: "Noblegarden".into(),
        date_range: EventDateRange::new(MonthDay::new(4, 5), MonthDay::new(4, 11)),
    }
}

pub fn childrens_week() -> HolidayEvent {
    HolidayEvent {
        id: 4,
        name: "Children's Week".into(),
        date_range: EventDateRange::new(MonthDay::new(5, 1), MonthDay::new(5, 7)),
    }
}

pub fn midsummer_fire_festival() -> HolidayEvent {
    HolidayEvent {
        id: 5,
        name: "Midsummer Fire Festival".into(),
        date_range: EventDateRange::new(MonthDay::new(6, 21), MonthDay::new(7, 5)),
    }
}

pub fn brewfest() -> HolidayEvent {
    HolidayEvent {
        id: 6,
        name: "Brewfest".into(),
        date_range: EventDateRange::new(MonthDay::new(9, 20), MonthDay::new(10, 6)),
    }
}

pub fn hallows_end() -> HolidayEvent {
    HolidayEvent {
        id: 7,
        name: "Hallow's End".into(),
        date_range: EventDateRange::new(MonthDay::new(10, 18), MonthDay::new(11, 1)),
    }
}

pub fn pilgrims_bounty() -> HolidayEvent {
    HolidayEvent {
        id: 8,
        name: "Pilgrim's Bounty".into(),
        date_range: EventDateRange::new(MonthDay::new(11, 22), MonthDay::new(11, 28)),
    }
}

pub fn winter_veil() -> HolidayEvent {
    HolidayEvent {
        id: 9,
        name: "Winter Veil".into(),
        date_range: EventDateRange::new(MonthDay::new(12, 15), MonthDay::new(1, 2)),
    }
}

/// All built-in holiday events.
pub fn all_events() -> Vec<HolidayEvent> {
    vec![
        lunar_festival(),
        love_is_in_the_air(),
        noblegarden(),
        childrens_week(),
        midsummer_fire_festival(),
        brewfest(),
        hallows_end(),
        pilgrims_bounty(),
        winter_veil(),
    ]
}

/// Find an event by ID.
pub fn event_by_id(id: u32) -> Option<HolidayEvent> {
    all_events().into_iter().find(|e| e.id == id)
}

/// Find all events active on a given date.
pub fn active_events(date: MonthDay) -> Vec<HolidayEvent> {
    all_events()
        .into_iter()
        .filter(|e| e.date_range.contains(date))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_day_ordinal() {
        assert_eq!(MonthDay::new(1, 1).ordinal(), 1);
        assert_eq!(MonthDay::new(2, 1).ordinal(), 32);
        assert_eq!(MonthDay::new(12, 31).ordinal(), 365);
    }

    #[test]
    fn range_contains_simple() {
        let range = EventDateRange::new(MonthDay::new(3, 1), MonthDay::new(3, 31));
        assert!(range.contains(MonthDay::new(3, 15)));
        assert!(range.contains(MonthDay::new(3, 1)));
        assert!(range.contains(MonthDay::new(3, 31)));
        assert!(!range.contains(MonthDay::new(2, 28)));
        assert!(!range.contains(MonthDay::new(4, 1)));
    }

    #[test]
    fn range_wraps_year_boundary() {
        // Winter Veil: Dec 15 – Jan 2
        let range = EventDateRange::new(MonthDay::new(12, 15), MonthDay::new(1, 2));
        assert!(range.contains(MonthDay::new(12, 25)));
        assert!(range.contains(MonthDay::new(12, 31)));
        assert!(range.contains(MonthDay::new(1, 1)));
        assert!(range.contains(MonthDay::new(1, 2)));
        assert!(!range.contains(MonthDay::new(1, 3)));
        assert!(!range.contains(MonthDay::new(12, 14)));
        assert!(!range.contains(MonthDay::new(6, 15)));
    }

    #[test]
    fn all_events_count() {
        assert_eq!(all_events().len(), 9);
    }

    #[test]
    fn event_ids_unique() {
        let events = all_events();
        let mut ids: Vec<u32> = events.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), events.len());
    }

    #[test]
    fn lookup_by_id() {
        assert_eq!(event_by_id(6).unwrap().name, "Brewfest");
        assert_eq!(event_by_id(9).unwrap().name, "Winter Veil");
        assert!(event_by_id(999).is_none());
    }

    #[test]
    fn active_events_brewfest() {
        let active = active_events(MonthDay::new(9, 25));
        assert!(active.iter().any(|e| e.name == "Brewfest"));
    }

    #[test]
    fn active_events_winter_veil_dec() {
        let active = active_events(MonthDay::new(12, 25));
        assert!(active.iter().any(|e| e.name == "Winter Veil"));
    }

    #[test]
    fn active_events_winter_veil_jan() {
        let active = active_events(MonthDay::new(1, 1));
        assert!(active.iter().any(|e| e.name == "Winter Veil"));
    }

    #[test]
    fn active_events_mid_august_none() {
        let active = active_events(MonthDay::new(8, 15));
        assert!(active.is_empty(), "no events in mid-August");
    }

    #[test]
    fn hallows_end_dates() {
        let event = hallows_end();
        assert!(event.date_range.contains(MonthDay::new(10, 18)));
        assert!(event.date_range.contains(MonthDay::new(10, 31)));
        assert!(event.date_range.contains(MonthDay::new(11, 1)));
        assert!(!event.date_range.contains(MonthDay::new(10, 17)));
        assert!(!event.date_range.contains(MonthDay::new(11, 2)));
    }

    #[test]
    fn serialization_round_trip() {
        let event = brewfest();
        let json = serde_json::to_string(&event).unwrap();
        let restored: HolidayEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, restored);
    }
}
