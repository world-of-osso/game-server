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
    let available = available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(9, 25));
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
    let available = available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(8, 15));
    assert!(available.is_empty());
}

#[test]
fn available_quests_forced_event() {
    let quests = vec![brewfest_quest()];
    let mut activation = EventActivation::default();
    activation.set_override(6, EventOverride::ForceOn);
    let tracker = DailyQuestTracker::default();
    // Force-active in August
    let available = available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(8, 15));
    assert_eq!(available.len(), 1);
}

// -- Boss tests --

#[test]
fn all_event_bosses_count() {
    assert_eq!(all_event_bosses().len(), 3);
}

#[test]
fn boss_ids_unique() {
    let bosses = all_event_bosses();
    let mut ids: Vec<u32> = bosses.iter().map(|b| b.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), bosses.len());
}

#[test]
fn lockout_record_and_check() {
    let mut tracker = BossLockoutTracker::default();
    let today = MonthDay::new(10, 25);
    assert!(!tracker.is_locked(1, 1, today));
    assert!(tracker.record_kill(1, 1, today));
    assert!(tracker.is_locked(1, 1, today));
}

#[test]
fn lockout_cannot_kill_twice() {
    let mut tracker = BossLockoutTracker::default();
    let today = MonthDay::new(10, 25);
    tracker.record_kill(1, 1, today);
    assert!(!tracker.record_kill(1, 1, today));
}

#[test]
fn lockout_different_day_resets() {
    let mut tracker = BossLockoutTracker::default();
    tracker.record_kill(1, 1, MonthDay::new(10, 25));
    assert!(!tracker.is_locked(1, 1, MonthDay::new(10, 26)));
}

#[test]
fn lockout_daily_reset() {
    let mut tracker = BossLockoutTracker::default();
    tracker.record_kill(1, 1, MonthDay::new(10, 25));
    tracker.daily_reset(MonthDay::new(10, 26));
    assert!(!tracker.is_locked(1, 1, MonthDay::new(10, 25)));
}

#[test]
fn available_bosses_during_event() {
    let bosses = all_event_bosses();
    let activation = EventActivation::default();
    let lockouts = BossLockoutTracker::default();
    // During Hallow's End
    let avail = available_bosses(
        &bosses,
        &activation,
        &lockouts,
        1,
        60,
        MonthDay::new(10, 25),
    );
    assert!(avail.iter().any(|b| b.name == "Headless Horseman"));
}

#[test]
fn available_bosses_excludes_inactive_event() {
    let bosses = all_event_bosses();
    let activation = EventActivation::default();
    let lockouts = BossLockoutTracker::default();
    // Mid-August: no events
    let avail = available_bosses(&bosses, &activation, &lockouts, 1, 60, MonthDay::new(8, 15));
    assert!(avail.is_empty());
}

#[test]
fn available_bosses_excludes_low_level() {
    let bosses = all_event_bosses();
    let activation = EventActivation::default();
    let lockouts = BossLockoutTracker::default();
    // Level 5 < min_level 15
    let avail = available_bosses(&bosses, &activation, &lockouts, 1, 5, MonthDay::new(10, 25));
    assert!(avail.is_empty());
}

#[test]
fn available_bosses_excludes_locked_out() {
    let bosses = all_event_bosses();
    let activation = EventActivation::default();
    let mut lockouts = BossLockoutTracker::default();
    let today = MonthDay::new(10, 25);
    lockouts.record_kill(1, 1, today); // killed Headless Horseman
    let avail = available_bosses(&bosses, &activation, &lockouts, 1, 60, today);
    assert!(
        !avail.iter().any(|b| b.id == 1),
        "locked boss should not appear"
    );
}

// -- Reward tests --

fn brewfest_mount() -> EventReward {
    EventReward {
        id: 1,
        name: "Brewfest Ram".into(),
        event_id: 6,
        category: RewardCategory::Mount,
        value: 9001,
    }
}

fn brewfest_toy() -> EventReward {
    EventReward {
        id: 2,
        name: "Brewfest Pony Keg".into(),
        event_id: 6,
        category: RewardCategory::Toy,
        value: 9002,
    }
}

fn hallows_achievement() -> EventReward {
    EventReward {
        id: 3,
        name: "Hallowed Be Thy Name".into(),
        event_id: 7,
        category: RewardCategory::Achievement,
        value: 500,
    }
}

#[test]
fn reward_claim_and_check() {
    let mut tracker = RewardTracker::default();
    assert!(!tracker.is_claimed(1, 1));
    assert!(tracker.claim(1, 1));
    assert!(tracker.is_claimed(1, 1));
}

#[test]
fn reward_cannot_claim_twice() {
    let mut tracker = RewardTracker::default();
    tracker.claim(1, 1);
    assert!(!tracker.claim(1, 1));
}

#[test]
fn reward_different_player_independent() {
    let mut tracker = RewardTracker::default();
    tracker.claim(1, 1);
    assert!(!tracker.is_claimed(2, 1));
}

#[test]
fn reward_claimed_count() {
    let mut tracker = RewardTracker::default();
    tracker.claim(1, 1);
    tracker.claim(1, 2);
    tracker.claim(2, 1);
    assert_eq!(tracker.claimed_count(1), 2);
    assert_eq!(tracker.claimed_count(2), 1);
    assert_eq!(tracker.claimed_count(3), 0);
}

#[test]
fn available_rewards_active_event() {
    let rewards = vec![brewfest_mount(), brewfest_toy(), hallows_achievement()];
    let activation = EventActivation::default();
    let tracker = RewardTracker::default();
    // During Brewfest
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(9, 25));
    assert_eq!(avail.len(), 2);
    assert!(avail.iter().all(|r| r.event_id == 6));
}

#[test]
fn available_rewards_excludes_claimed() {
    let rewards = vec![brewfest_mount(), brewfest_toy()];
    let activation = EventActivation::default();
    let mut tracker = RewardTracker::default();
    tracker.claim(1, 1); // claimed mount
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(9, 25));
    assert_eq!(avail.len(), 1);
    assert_eq!(avail[0].id, 2);
}

#[test]
fn available_rewards_inactive_event_empty() {
    let rewards = vec![brewfest_mount()];
    let activation = EventActivation::default();
    let tracker = RewardTracker::default();
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(8, 15));
    assert!(avail.is_empty());
}

#[test]
fn available_rewards_multiple_categories() {
    let rewards = vec![brewfest_mount(), brewfest_toy(), hallows_achievement()];
    let activation = EventActivation::default();
    let tracker = RewardTracker::default();
    // During Hallow's End
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(10, 25));
    assert!(
        avail
            .iter()
            .any(|r| r.category == RewardCategory::Achievement)
    );
}

// -- Integration tests: full pipeline --

#[test]
fn full_pipeline_date_activates_quests_bosses_rewards() {
    let quests = vec![brewfest_quest(), hallows_quest()];
    let bosses = all_event_bosses();
    let rewards = vec![brewfest_mount(), hallows_achievement()];
    let activation = EventActivation::default();
    let quest_tracker = DailyQuestTracker::default();
    let lockouts = BossLockoutTracker::default();
    let reward_tracker = RewardTracker::default();

    // During Brewfest (Sep 25)
    let date = MonthDay::new(9, 25);

    let avail_quests = available_event_quests(&quests, &activation, &quest_tracker, 1, date);
    assert_eq!(avail_quests.len(), 1);
    assert_eq!(avail_quests[0].name, "Bark for the Barkers");

    let avail_bosses = available_bosses(&bosses, &activation, &lockouts, 1, 60, date);
    assert!(avail_bosses.iter().any(|b| b.name == "Coren Direbrew"));
    assert!(!avail_bosses.iter().any(|b| b.name == "Headless Horseman"));

    let avail_rewards = available_rewards(&rewards, &activation, &reward_tracker, 1, date);
    assert_eq!(avail_rewards.len(), 1);
    assert_eq!(avail_rewards[0].name, "Brewfest Ram");
}

#[test]
fn date_change_swaps_available_content() {
    let quests = vec![brewfest_quest(), hallows_quest()];
    let activation = EventActivation::default();
    let tracker = DailyQuestTracker::default();

    // Brewfest active
    let bf_quests = available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(9, 25));
    assert_eq!(bf_quests.len(), 1);
    assert_eq!(bf_quests[0].event_id, 6);

    // Switch to Hallow's End
    let he_quests =
        available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(10, 25));
    assert_eq!(he_quests.len(), 1);
    assert_eq!(he_quests[0].event_id, 7);

    // Neither active
    let none = available_event_quests(&quests, &activation, &tracker, 1, MonthDay::new(8, 15));
    assert!(none.is_empty());
}

#[test]
fn quest_completion_then_daily_reset_cycle() {
    let quests = vec![brewfest_quest(), brewfest_quest_2()];
    let activation = EventActivation::default();
    let mut tracker = DailyQuestTracker::default();
    let day1 = MonthDay::new(9, 25);

    // Day 1: complete both quests
    tracker.complete(1, 100, day1);
    tracker.complete(1, 101, day1);
    let avail = available_event_quests(&quests, &activation, &tracker, 1, day1);
    assert!(avail.is_empty(), "all done for today");

    // Day 2: reset, quests available again
    let day2 = MonthDay::new(9, 26);
    tracker.daily_reset(day2);
    let avail = available_event_quests(&quests, &activation, &tracker, 1, day2);
    assert_eq!(avail.len(), 2, "quests reset for new day");
}

#[test]
fn boss_kill_lockout_then_reset() {
    let bosses = all_event_bosses();
    let activation = EventActivation::default();
    let mut lockouts = BossLockoutTracker::default();
    let day1 = MonthDay::new(10, 25);

    // Kill Headless Horseman
    lockouts.record_kill(1, 1, day1);
    let avail = available_bosses(&bosses, &activation, &lockouts, 1, 60, day1);
    assert!(!avail.iter().any(|b| b.id == 1));

    // Next day: reset, boss available again
    let day2 = MonthDay::new(10, 26);
    lockouts.daily_reset(day2);
    let avail = available_bosses(&bosses, &activation, &lockouts, 1, 60, day2);
    assert!(avail.iter().any(|b| b.id == 1));
}

#[test]
fn reward_claim_persists_across_days() {
    let rewards = vec![brewfest_mount()];
    let activation = EventActivation::default();
    let mut tracker = RewardTracker::default();

    // Claim mount on day 1
    tracker.claim(1, 1);
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(9, 25));
    assert!(avail.is_empty());

    // Still claimed on day 2 (rewards don't reset daily)
    let avail = available_rewards(&rewards, &activation, &tracker, 1, MonthDay::new(9, 26));
    assert!(avail.is_empty());
}

#[test]
fn force_override_enables_off_season_content() {
    let quests = vec![brewfest_quest()];
    let bosses = all_event_bosses();
    let rewards = vec![brewfest_mount()];
    let mut activation = EventActivation::default();
    let quest_tracker = DailyQuestTracker::default();
    let lockouts = BossLockoutTracker::default();
    let reward_tracker = RewardTracker::default();

    // August: nothing active
    let date = MonthDay::new(8, 15);
    assert!(available_event_quests(&quests, &activation, &quest_tracker, 1, date).is_empty());

    // Admin forces Brewfest on
    activation.set_override(6, EventOverride::ForceOn);
    assert_eq!(
        available_event_quests(&quests, &activation, &quest_tracker, 1, date).len(),
        1
    );
    assert!(
        available_bosses(&bosses, &activation, &lockouts, 1, 60, date)
            .iter()
            .any(|b| b.name == "Coren Direbrew")
    );
    assert_eq!(
        available_rewards(&rewards, &activation, &reward_tracker, 1, date).len(),
        1
    );
}
