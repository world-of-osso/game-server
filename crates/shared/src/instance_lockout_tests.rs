use crate::instance::{DAY_SECS, Difficulty, ResetTimer, WEEK_SECS};

use super::*;

// --- InstanceSave ---

#[test]
fn instance_save_expires_daily() {
    let save = InstanceSave {
        map_id: 33,
        difficulty: Difficulty::Heroic,
        instance_id: 1,
        completed_encounters: 0b0000_0001,
        acquired_at: 1000,
        reset_timer: ResetTimer::Daily,
    };
    assert_eq!(save.expires_at(), Some(DAY_SECS));
    assert!(!save.is_expired(1000));
    assert!(!save.is_expired(DAY_SECS - 1));
    assert!(save.is_expired(DAY_SECS));
}

#[test]
fn instance_save_expires_weekly() {
    let save = InstanceSave {
        map_id: 409,
        difficulty: Difficulty::Normal,
        instance_id: 2,
        completed_encounters: 0,
        acquired_at: 1000,
        reset_timer: ResetTimer::Weekly,
    };
    assert_eq!(save.expires_at(), Some(WEEK_SECS));
    assert!(!save.is_expired(WEEK_SECS - 1));
    assert!(save.is_expired(WEEK_SECS));
}

#[test]
fn instance_save_no_lockout_never_expires() {
    let save = InstanceSave {
        map_id: 33,
        difficulty: Difficulty::Normal,
        instance_id: 1,
        completed_encounters: 0,
        acquired_at: 1000,
        reset_timer: ResetTimer::None,
    };
    assert_eq!(save.expires_at(), None);
    assert!(!save.is_expired(u64::MAX));
}

#[test]
fn encounter_tracking() {
    let mut save = InstanceSave {
        map_id: 409,
        difficulty: Difficulty::Normal,
        instance_id: 1,
        completed_encounters: 0,
        acquired_at: 1000,
        reset_timer: ResetTimer::Weekly,
    };
    assert_eq!(save.completed_count(), 0);

    save.complete_encounter(0);
    save.complete_encounter(3);
    save.complete_encounter(7);

    assert!(save.is_encounter_done(0));
    assert!(!save.is_encounter_done(1));
    assert!(save.is_encounter_done(3));
    assert!(save.is_encounter_done(7));
    assert_eq!(save.completed_count(), 3);
    assert_eq!(save.completed_encounters, 0b1000_1001);
}

// --- CharacterLockouts ---

#[test]
fn bind_creates_lockout() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);

    assert!(lockouts.is_locked(33, Difficulty::Heroic, 1000));
    assert!(!lockouts.is_locked(33, Difficulty::Normal, 1000));
    assert_eq!(lockouts.len(), 1);
}

#[test]
fn bind_updates_existing_encounters() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(409, Difficulty::Normal, 1, 0, ResetTimer::Weekly, 1000);
    lockouts.bind(409, Difficulty::Normal, 1, 2, ResetTimer::Weekly, 2000);

    // Should update existing save, not create a second
    assert_eq!(lockouts.len(), 1);
    let save = lockouts.find(409, Difficulty::Normal).unwrap();
    assert!(save.is_encounter_done(0));
    assert!(save.is_encounter_done(2));
    assert_eq!(save.completed_count(), 2);
}

#[test]
fn heroic_daily_lockout() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);

    // Locked before daily reset
    assert!(lockouts.is_locked(33, Difficulty::Heroic, DAY_SECS - 1));
    // Unlocked after daily reset
    assert!(!lockouts.is_locked(33, Difficulty::Heroic, DAY_SECS));
}

#[test]
fn raid_weekly_lockout() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(409, Difficulty::Mythic, 5, 0, ResetTimer::Weekly, 1000);

    assert!(lockouts.is_locked(409, Difficulty::Mythic, WEEK_SECS - 1));
    assert!(!lockouts.is_locked(409, Difficulty::Mythic, WEEK_SECS));
}

#[test]
fn different_difficulties_independent() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);

    // Heroic locked, normal not
    assert!(lockouts.is_locked(33, Difficulty::Heroic, 1000));
    assert!(!lockouts.is_locked(33, Difficulty::Normal, 1000));
    assert!(!lockouts.is_locked(33, Difficulty::Mythic, 1000));
}

#[test]
fn bound_instance_for_rejoin() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 42, 0, ResetTimer::Daily, 1000);

    assert_eq!(
        lockouts.bound_instance(33, Difficulty::Heroic, 1000),
        Some(42)
    );
    assert_eq!(lockouts.bound_instance(33, Difficulty::Normal, 1000), None);
    // Expired lockout returns None
    assert_eq!(
        lockouts.bound_instance(33, Difficulty::Heroic, DAY_SECS),
        None
    );
}

#[test]
fn cleanup_removes_expired() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);
    lockouts.bind(409, Difficulty::Mythic, 2, 0, ResetTimer::Weekly, 1000);

    // After daily reset but before weekly
    lockouts.cleanup(DAY_SECS);
    assert_eq!(lockouts.len(), 1);
    assert!(lockouts.find(409, Difficulty::Mythic).is_some());
}

#[test]
fn active_lockouts_filters_expired() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);
    lockouts.bind(409, Difficulty::Normal, 2, 0, ResetTimer::Weekly, 1000);

    let active = lockouts.active_lockouts(DAY_SECS);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].map_id, 409);
}

#[test]
fn next_reset_returns_earliest() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);
    lockouts.bind(409, Difficulty::Normal, 2, 0, ResetTimer::Weekly, 1000);

    // Daily reset comes first
    assert_eq!(lockouts.next_reset(1000), Some(DAY_SECS));
}

#[test]
fn next_reset_none_when_empty() {
    let lockouts = CharacterLockouts::default();
    assert_eq!(lockouts.next_reset(1000), None);
}

#[test]
fn multiple_maps_tracked_independently() {
    let mut lockouts = CharacterLockouts::default();
    lockouts.bind(33, Difficulty::Heroic, 1, 0, ResetTimer::Daily, 1000);
    lockouts.bind(36, Difficulty::Heroic, 2, 0, ResetTimer::Daily, 2000);
    lockouts.bind(409, Difficulty::Normal, 3, 0, ResetTimer::Weekly, 3000);

    assert_eq!(lockouts.len(), 3);
    assert!(lockouts.is_locked(33, Difficulty::Heroic, 5000));
    assert!(lockouts.is_locked(36, Difficulty::Heroic, 5000));
    assert!(lockouts.is_locked(409, Difficulty::Normal, 5000));
}
