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

// -- Event Bosses --

/// A special dungeon boss available only during a holiday event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventBoss {
    pub id: u32,
    pub name: String,
    /// Which holiday event this boss belongs to.
    pub event_id: u32,
    /// Dungeon/instance map ID where the boss spawns.
    pub map_id: u32,
    /// Minimum player level to queue.
    pub min_level: u8,
    /// Loot item IDs dropped on kill.
    pub loot_table: Vec<u32>,
    /// Whether the boss can only be looted once per day.
    pub daily_lockout: bool,
}

/// Per-player daily boss kill tracking.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BossLockoutTracker {
    /// (player_id, boss_id, day_ordinal).
    kills: Vec<(u64, u32, u16)>,
}

impl BossLockoutTracker {
    /// Whether a player has already killed this boss today.
    pub fn is_locked(&self, player: u64, boss_id: u32, today: MonthDay) -> bool {
        let day = today.ordinal();
        self.kills
            .iter()
            .any(|&(p, b, d)| p == player && b == boss_id && d == day)
    }

    /// Record a boss kill. Returns false if already killed today.
    pub fn record_kill(&mut self, player: u64, boss_id: u32, today: MonthDay) -> bool {
        if self.is_locked(player, boss_id, today) {
            return false;
        }
        self.kills.push((player, boss_id, today.ordinal()));
        true
    }

    /// Reset all lockouts for a new day.
    pub fn daily_reset(&mut self, new_day: MonthDay) {
        let day = new_day.ordinal();
        self.kills.retain(|&(_, _, d)| d == day);
    }
}

/// Built-in event bosses.
pub fn headless_horseman() -> EventBoss {
    EventBoss {
        id: 1,
        name: "Headless Horseman".into(),
        event_id: 7, // Hallow's End
        map_id: 189,
        min_level: 15,
        loot_table: vec![5001, 5002, 5003],
        daily_lockout: true,
    }
}

pub fn coren_direbrew() -> EventBoss {
    EventBoss {
        id: 2,
        name: "Coren Direbrew".into(),
        event_id: 6, // Brewfest
        map_id: 230,
        min_level: 15,
        loot_table: vec![6001, 6002],
        daily_lockout: true,
    }
}

pub fn ahune() -> EventBoss {
    EventBoss {
        id: 3,
        name: "Ahune".into(),
        event_id: 5, // Midsummer Fire Festival
        map_id: 547,
        min_level: 15,
        loot_table: vec![7001, 7002],
        daily_lockout: true,
    }
}

pub fn all_event_bosses() -> Vec<EventBoss> {
    vec![headless_horseman(), coren_direbrew(), ahune()]
}

/// Get available event bosses for a player on a given date.
///
/// Filters by active event, player level, and daily lockout.
pub fn available_bosses<'a>(
    bosses: &'a [EventBoss],
    activation: &EventActivation,
    lockouts: &BossLockoutTracker,
    player: u64,
    player_level: u8,
    date: MonthDay,
) -> Vec<&'a EventBoss> {
    let active_ids = activation.active_event_ids(date);
    bosses
        .iter()
        .filter(|b| active_ids.contains(&b.event_id))
        .filter(|b| player_level >= b.min_level)
        .filter(|b| !b.daily_lockout || !lockouts.is_locked(player, b.id, date))
        .collect()
}

// -- Event Rewards --

/// Category of an event reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RewardCategory {
    /// Cosmetic armor/weapon appearance.
    Cosmetic,
    /// Toy (usable item with fun effect).
    Toy,
    /// Mount.
    Mount,
    /// Achievement.
    Achievement,
    /// Currency (e.g. event tokens).
    Currency,
}

/// A reward obtainable during a holiday event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventReward {
    pub id: u32,
    pub name: String,
    pub event_id: u32,
    pub category: RewardCategory,
    /// Item ID for Cosmetic/Toy/Mount, achievement ID for Achievement,
    /// currency amount for Currency.
    pub value: u32,
}

/// Tracks which event rewards a player has claimed (account-wide).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RewardTracker {
    /// (player_id, reward_id) pairs.
    claimed: Vec<(u64, u32)>,
}

impl RewardTracker {
    pub fn is_claimed(&self, player: u64, reward_id: u32) -> bool {
        self.claimed
            .iter()
            .any(|&(p, r)| p == player && r == reward_id)
    }

    /// Claim a reward. Returns false if already claimed.
    pub fn claim(&mut self, player: u64, reward_id: u32) -> bool {
        if self.is_claimed(player, reward_id) {
            return false;
        }
        self.claimed.push((player, reward_id));
        true
    }

    /// Count of rewards claimed by a player.
    pub fn claimed_count(&self, player: u64) -> usize {
        self.claimed.iter().filter(|&&(p, _)| p == player).count()
    }
}

/// Get unclaimed rewards available to a player during active events.
pub fn available_rewards<'a>(
    rewards: &'a [EventReward],
    activation: &EventActivation,
    tracker: &RewardTracker,
    player: u64,
    date: MonthDay,
) -> Vec<&'a EventReward> {
    let active_ids = activation.active_event_ids(date);
    rewards
        .iter()
        .filter(|r| active_ids.contains(&r.event_id))
        .filter(|r| !tracker.is_claimed(player, r.id))
        .collect()
}

#[cfg(test)]
#[path = "holiday_tests.rs"]
mod tests;
