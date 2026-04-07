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

// -- Event Activation --

/// Admin override for an event's activation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventOverride {
    /// Follow the calendar date range.
    Auto,
    /// Force-enable regardless of date.
    ForceOn,
    /// Force-disable regardless of date.
    ForceOff,
}

/// Tracks which events are currently active on the server.
///
/// By default, events activate/deactivate based on the calendar.
/// Admins can override individual events with `ForceOn`/`ForceOff`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EventActivation {
    overrides: Vec<(u32, EventOverride)>,
}

impl EventActivation {
    /// Set an admin override for an event.
    pub fn set_override(&mut self, event_id: u32, state: EventOverride) {
        if let Some(entry) = self.overrides.iter_mut().find(|(id, _)| *id == event_id) {
            entry.1 = state;
        } else {
            self.overrides.push((event_id, state));
        }
    }

    /// Clear an override, returning the event to calendar-based activation.
    pub fn clear_override(&mut self, event_id: u32) {
        self.overrides.retain(|(id, _)| *id != event_id);
    }

    /// Get the override state for an event (None = no override).
    pub fn get_override(&self, event_id: u32) -> Option<EventOverride> {
        self.overrides
            .iter()
            .find(|(id, _)| *id == event_id)
            .map(|(_, state)| *state)
    }

    /// Whether an event is currently active, considering overrides and date.
    pub fn is_active(&self, event: &HolidayEvent, current_date: MonthDay) -> bool {
        match self.get_override(event.id) {
            Some(EventOverride::ForceOn) => true,
            Some(EventOverride::ForceOff) => false,
            Some(EventOverride::Auto) | None => event.date_range.contains(current_date),
        }
    }

    /// All currently active events, considering overrides.
    pub fn active_event_ids(&self, current_date: MonthDay) -> Vec<u32> {
        all_events()
            .iter()
            .filter(|e| self.is_active(e, current_date))
            .map(|e| e.id)
            .collect()
    }
}

// -- Event Quests --

/// A daily quest tied to a holiday event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventQuest {
    pub id: u32,
    pub name: String,
    /// Which holiday event this quest belongs to.
    pub event_id: u32,
    /// XP reward on completion.
    pub xp_reward: u32,
    /// Gold reward in copper.
    pub gold_reward: u32,
    /// Item reward IDs (if any).
    pub item_rewards: Vec<u32>,
}

/// Per-player daily quest completion tracking.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DailyQuestTracker {
    /// (player_id, quest_id, day_ordinal) — completed dailies.
    completions: Vec<(u64, u32, u16)>,
}

impl DailyQuestTracker {
    /// Whether a player has completed a daily quest today.
    pub fn is_completed(&self, player: u64, quest_id: u32, today: MonthDay) -> bool {
        let day = today.ordinal();
        self.completions
            .iter()
            .any(|&(p, q, d)| p == player && q == quest_id && d == day)
    }

    /// Mark a daily quest as completed. Returns false if already done today.
    pub fn complete(&mut self, player: u64, quest_id: u32, today: MonthDay) -> bool {
        if self.is_completed(player, quest_id, today) {
            return false;
        }
        self.completions.push((player, quest_id, today.ordinal()));
        true
    }

    /// Reset all completions for a new day (call on daily reset).
    pub fn daily_reset(&mut self, new_day: MonthDay) {
        let day = new_day.ordinal();
        self.completions.retain(|&(_, _, d)| d == day);
    }
}

/// Get available event quests for a player on a given date.
///
/// Returns quests whose event is active and the player hasn't completed today.
pub fn available_event_quests<'a>(
    all_quests: &'a [EventQuest],
    activation: &EventActivation,
    tracker: &DailyQuestTracker,
    player: u64,
    date: MonthDay,
) -> Vec<&'a EventQuest> {
    let active_ids = activation.active_event_ids(date);
    all_quests
        .iter()
        .filter(|q| active_ids.contains(&q.event_id))
        .filter(|q| !tracker.is_completed(player, q.id, date))
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

    // -- Activation tests --

    #[test]
    fn activation_default_follows_calendar() {
        let activation = EventActivation::default();
        let bf = brewfest();
        assert!(activation.is_active(&bf, MonthDay::new(9, 25)));
        assert!(!activation.is_active(&bf, MonthDay::new(8, 15)));
    }

    #[test]
    fn activation_force_on_overrides_date() {
        let mut activation = EventActivation::default();
        let bf = brewfest();
        activation.set_override(bf.id, EventOverride::ForceOn);
        // Active even outside date range
        assert!(activation.is_active(&bf, MonthDay::new(3, 1)));
    }

    #[test]
    fn activation_force_off_overrides_date() {
        let mut activation = EventActivation::default();
        let bf = brewfest();
        activation.set_override(bf.id, EventOverride::ForceOff);
        // Inactive even during date range
        assert!(!activation.is_active(&bf, MonthDay::new(9, 25)));
    }

    #[test]
    fn activation_auto_override_same_as_no_override() {
        let mut activation = EventActivation::default();
        let bf = brewfest();
        activation.set_override(bf.id, EventOverride::Auto);
        assert!(activation.is_active(&bf, MonthDay::new(9, 25)));
        assert!(!activation.is_active(&bf, MonthDay::new(8, 15)));
    }

    #[test]
    fn activation_clear_override() {
        let mut activation = EventActivation::default();
        let bf = brewfest();
        activation.set_override(bf.id, EventOverride::ForceOff);
        assert!(!activation.is_active(&bf, MonthDay::new(9, 25)));

        activation.clear_override(bf.id);
        assert!(activation.is_active(&bf, MonthDay::new(9, 25)));
    }

    #[test]
    fn activation_override_update() {
        let mut activation = EventActivation::default();
        activation.set_override(6, EventOverride::ForceOn);
        activation.set_override(6, EventOverride::ForceOff);
        assert_eq!(activation.get_override(6), Some(EventOverride::ForceOff));
    }

    #[test]
    fn activation_active_event_ids() {
        let mut activation = EventActivation::default();
        // Mid-August: no events naturally active
        let date = MonthDay::new(8, 15);
        assert!(activation.active_event_ids(date).is_empty());

        // Force-enable Brewfest
        activation.set_override(6, EventOverride::ForceOn);
        let ids = activation.active_event_ids(date);
        assert_eq!(ids, vec![6]);
    }

    #[test]
    fn activation_multiple_overrides_independent() {
        let mut activation = EventActivation::default();
        activation.set_override(6, EventOverride::ForceOn);
        activation.set_override(7, EventOverride::ForceOff);

        let bf = brewfest();
        let he = hallows_end();
        // During Hallow's End date range
        let date = MonthDay::new(10, 25);
        assert!(activation.is_active(&bf, date)); // forced on
        assert!(!activation.is_active(&he, date)); // forced off despite in range
    }

    // -- Quest tests --

    fn brewfest_quest() -> EventQuest {
        EventQuest {
            id: 100,
            name: "Bark for the Barkers".into(),
            event_id: 6, // Brewfest
            xp_reward: 500,
            gold_reward: 1000,
            item_rewards: vec![],
        }
    }

    fn brewfest_quest_2() -> EventQuest {
        EventQuest {
            id: 101,
            name: "This One Time".into(),
            event_id: 6,
            xp_reward: 300,
            gold_reward: 500,
            item_rewards: vec![42],
        }
    }

    fn hallows_quest() -> EventQuest {
        EventQuest {
            id: 200,
            name: "Trick or Treat".into(),
            event_id: 7, // Hallow's End
            xp_reward: 400,
            gold_reward: 800,
            item_rewards: vec![],
        }
    }

    #[test]
    fn daily_complete_and_check() {
        let mut tracker = DailyQuestTracker::default();
        let today = MonthDay::new(9, 25);
        assert!(!tracker.is_completed(1, 100, today));
        assert!(tracker.complete(1, 100, today));
        assert!(tracker.is_completed(1, 100, today));
    }

    #[test]
    fn daily_cannot_complete_twice() {
        let mut tracker = DailyQuestTracker::default();
        let today = MonthDay::new(9, 25);
        tracker.complete(1, 100, today);
        assert!(!tracker.complete(1, 100, today));
    }

    #[test]
    fn daily_different_player_independent() {
        let mut tracker = DailyQuestTracker::default();
        let today = MonthDay::new(9, 25);
        tracker.complete(1, 100, today);
        assert!(!tracker.is_completed(2, 100, today));
    }

    #[test]
    fn daily_different_quest_independent() {
        let mut tracker = DailyQuestTracker::default();
        let today = MonthDay::new(9, 25);
        tracker.complete(1, 100, today);
        assert!(!tracker.is_completed(1, 101, today));
    }

    #[test]
    fn daily_different_day_resets() {
        let mut tracker = DailyQuestTracker::default();
        tracker.complete(1, 100, MonthDay::new(9, 25));
        // Next day — not completed
        assert!(!tracker.is_completed(1, 100, MonthDay::new(9, 26)));
    }

    #[test]
    fn daily_reset_clears_old() {
        let mut tracker = DailyQuestTracker::default();
        tracker.complete(1, 100, MonthDay::new(9, 25));
        tracker.daily_reset(MonthDay::new(9, 26));
        assert!(!tracker.is_completed(1, 100, MonthDay::new(9, 25)));
    }

    #[test]
    fn available_quests_active_event() {
        let quests = vec![brewfest_quest(), brewfest_quest_2(), hallows_quest()];
        let activation = EventActivation::default();
        let tracker = DailyQuestTracker::default();
        // During Brewfest (Sep 25): only Brewfest quests available
        let available = available_event_quests(
            &quests,
            &activation,
            &tracker,
            1,
            MonthDay::new(9, 25),
        );
        assert_eq!(available.len(), 2);
        assert!(available.iter().all(|q| q.event_id == 6));
    }

    #[test]
    fn available_quests_excludes_completed() {
        let quests = vec![brewfest_quest(), brewfest_quest_2()];
        let activation = EventActivation::default();
        let mut tracker = DailyQuestTracker::default();
        let today = MonthDay::new(9, 25);
        tracker.complete(1, 100, today);

        let available = available_event_quests(&quests, &activation, &tracker, 1, today);
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].id, 101);
    }

    #[test]
    fn available_quests_inactive_event_empty() {
        let quests = vec![brewfest_quest()];
        let activation = EventActivation::default();
        let tracker = DailyQuestTracker::default();
        // Mid-August: Brewfest not active
        let available = available_event_quests(
            &quests,
            &activation,
            &tracker,
            1,
            MonthDay::new(8, 15),
        );
        assert!(available.is_empty());
    }

    #[test]
    fn available_quests_forced_event() {
        let quests = vec![brewfest_quest()];
        let mut activation = EventActivation::default();
        activation.set_override(6, EventOverride::ForceOn);
        let tracker = DailyQuestTracker::default();
        // Force-active in August
        let available = available_event_quests(
            &quests,
            &activation,
            &tracker,
            1,
            MonthDay::new(8, 15),
        );
        assert_eq!(available.len(), 1);
    }
}
