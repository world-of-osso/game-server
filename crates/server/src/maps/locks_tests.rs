//! Instance locks on Grim Batol's `MapDifficulty.db2` rows (build 69933): Heroic (ID
//! 1 row: DifficultyID 2, LockID 0, daily, flags 0 → instance-id bound) and Mythic
//! (DifficultyID 23, LockID 251, weekly, flags 0x42 → encounter locks).

use super::*;
use crate::maps::store::{MapDifficulty, ResetInterval};

const GRIM_BATOL: u32 = 670;
/// 2026-09-27 12:00 UTC, a Sunday.
const SUNDAY_NOON: i64 = 1_790_510_400;
/// 2026-09-27 15:00 UTC: Sunday's daily reset.
const SUNDAY_RESET: i64 = 1_790_521_200;
/// 2026-09-28 15:00 UTC.
const MONDAY_RESET: i64 = 1_790_607_600;
/// 2026-09-29 15:00 UTC: the weekly reset.
const TUESDAY_RESET: i64 = 1_790_694_000;
/// 2026-10-06 15:00 UTC.
const NEXT_TUESDAY_RESET: i64 = 1_791_298_800;
/// General Umbriss: boss 0 of instance_grim_batol, DungeonEncounter 1051 Bit 3.
const UMBRISS_BOSS: u32 = 0;
const UMBRISS_BIT: u32 = 3;

fn heroic() -> MapDifficulty {
    MapDifficulty {
        id: 346,
        map_id: GRIM_BATOL,
        difficulty_id: 2,
        lock_id: 0,
        reset_interval: ResetInterval::Daily,
        max_players: 5,
        flags: 0,
        flexible_locking: false,
    }
}

fn mythic() -> MapDifficulty {
    MapDifficulty {
        id: 5475,
        map_id: GRIM_BATOL,
        difficulty_id: 23,
        lock_id: 251,
        reset_interval: ResetInterval::Weekly,
        max_players: 5,
        flags: 0x42,
        flexible_locking: false,
    }
}

fn normal() -> MapDifficulty {
    MapDifficulty {
        id: 345,
        map_id: GRIM_BATOL,
        difficulty_id: 1,
        lock_id: 0,
        reset_interval: ResetInterval::Anytime,
        max_players: 5,
        flags: 0,
        flexible_locking: false,
    }
}

fn umbriss_kill(instance_id: u32) -> LockUpdate {
    LockUpdate {
        instance_id,
        done_bosses: 1 << UMBRISS_BOSS,
        instance_completed_encounters: 1 << UMBRISS_BIT,
        completed_encounter_bit: Some(UMBRISS_BIT),
    }
}

#[test]
fn daily_and_weekly_resets_fall_on_retail_reset_hour() {
    assert_eq!(
        next_reset_time(ResetInterval::Daily, SUNDAY_NOON),
        SUNDAY_RESET
    );
    // From the reset hour on, the next day's.
    assert_eq!(
        next_reset_time(ResetInterval::Daily, SUNDAY_RESET),
        MONDAY_RESET
    );
    assert_eq!(
        next_reset_time(ResetInterval::Weekly, SUNDAY_NOON),
        TUESDAY_RESET
    );
    assert_eq!(
        next_reset_time(ResetInterval::Weekly, TUESDAY_RESET + 1800),
        NEXT_TUESDAY_RESET
    );
    assert_eq!(
        next_reset_time(ResetInterval::Weekly, TUESDAY_RESET - 1),
        TUESDAY_RESET
    );
}

#[test]
fn a_normal_dungeon_never_locks() {
    let mut locks = InstanceLocks::default();
    assert!(
        locks
            .create_for_new_instance(1, &normal(), 5, SUNDAY_NOON)
            .is_none()
    );
    assert!(locks.find_active_lock(1, &normal(), SUNDAY_NOON).is_none());
}

#[test]
fn a_new_heroic_copy_locks_its_owner_only_temporarily_until_a_boss_dies() {
    let mut locks = InstanceLocks::default();
    let lock = locks
        .create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON)
        .expect("heroic has a reset schedule");
    assert!(lock.is_new);
    assert_eq!(lock.expiry, SUNDAY_RESET);
    // A temporary lock neither shows in Raid Info nor keeps the player out of other copies.
    assert!(
        locks
            .raid_info(1, |_| Some(heroic()), SUNDAY_NOON)
            .is_empty()
    );
    assert_eq!(locks.can_join(1, &heroic(), 9, SUNDAY_NOON, 0), None);
    assert_eq!(locks.initialization_data(1, &heroic(), SUNDAY_NOON), None);
}

#[test]
fn a_heroic_boss_kill_saves_the_player_to_that_copy_until_the_daily_reset() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON);
    let kill = umbriss_kill(5);
    locks.update_shared(&kill);
    assert!(locks.update_for_player(1, &heroic(), &kill, SUNDAY_NOON));
    // A second kill in the same lock is not a new save.
    assert!(!locks.update_for_player(1, &heroic(), &kill, SUNDAY_NOON + 60));

    let lock = locks.lock(1, (GRIM_BATOL, 0)).expect("permanent lock");
    assert!(!lock.is_new);
    assert_eq!(lock.instance_id, 5);
    assert_eq!(lock.data.completed_encounters, 1 << UMBRISS_BIT);
    assert_eq!(
        locks.can_join(1, &heroic(), 9, SUNDAY_NOON, 0),
        Some(TransferAbortReason::LockedToDifferentInstance)
    );
    assert_eq!(locks.can_join(1, &heroic(), 5, SUNDAY_NOON, 0), None);
    // A recreated copy 5 resumes with Umbriss done.
    assert_eq!(
        locks.initialization_data(1, &heroic(), SUNDAY_NOON),
        Some(LockData {
            done_bosses: 1,
            completed_encounters: 1 << UMBRISS_BIT,
        })
    );
    assert_eq!(
        locks.raid_info(1, |_| Some(heroic()), SUNDAY_NOON),
        vec![InstanceLockInfo {
            map_id: GRIM_BATOL,
            difficulty_id: 2,
            instance_id: 5,
            time_remaining_secs: 3 * 3600,
            completed_mask: 1 << UMBRISS_BIT,
            locked: true,
            extended: false,
        }]
    );

    // After the reset the lock is expired: listed, no longer binding.
    let after = SUNDAY_RESET + 1;
    assert_eq!(locks.can_join(1, &heroic(), 9, after, 0), None);
    assert!(locks.find_active_lock(1, &heroic(), after).is_none());
    let info = locks.raid_info(1, |_| Some(heroic()), after);
    assert_eq!((info[0].locked, info[0].time_remaining_secs), (false, 0));
}

#[test]
fn a_kill_after_the_reset_starts_a_fresh_lock() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON);
    locks.update_for_player(1, &heroic(), &umbriss_kill(5), SUNDAY_NOON);
    // Next day, copy 8: Forgemaster Throngus (boss 1, Bit 2) only.
    let monday = SUNDAY_RESET + 3600;
    locks.create_for_new_instance(1, &heroic(), 8, monday);
    let throngus = LockUpdate {
        instance_id: 8,
        done_bosses: 1 << 1,
        instance_completed_encounters: 1 << 2,
        completed_encounter_bit: Some(2),
    };
    assert!(locks.update_for_player(1, &heroic(), &throngus, monday));
    let lock = locks.lock(1, (GRIM_BATOL, 0)).unwrap();
    assert_eq!(
        (
            lock.instance_id,
            lock.data.completed_encounters,
            lock.expiry
        ),
        (8, 1 << 2, MONDAY_RESET)
    );
}

#[test]
fn an_extended_lock_lasts_one_more_period_and_survives_its_reset() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON);
    locks.update_for_player(1, &heroic(), &umbriss_kill(5), SUNDAY_NOON);
    assert_eq!(
        locks.set_extended(1, &heroic(), true, SUNDAY_NOON),
        Some((SUNDAY_RESET, MONDAY_RESET))
    );
    let after = SUNDAY_RESET + 1;
    assert_eq!(
        locks
            .find_active_lock(1, &heroic(), after)
            .map(|lock| lock.instance_id),
        Some(5)
    );
    assert_eq!(
        locks.raid_info(1, |_| Some(heroic()), after)[0].time_remaining_secs,
        (MONDAY_RESET - after) as u32
    );
}

#[test]
fn mythic_encounter_locks_record_kills_without_binding_to_a_copy() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &mythic(), 5, SUNDAY_NOON);
    assert!(locks.update_for_player(1, &mythic(), &umbriss_kill(5), SUNDAY_NOON));
    let lock = locks.lock(1, (GRIM_BATOL, 251)).expect("mythic lock");
    assert_eq!(lock.expiry, TUESDAY_RESET);
    assert_eq!(locks.can_join(1, &mythic(), 9, SUNDAY_NOON, 0), None);
    // Heroic (LockID 0) is a separate lock.
    assert!(locks.lock(1, (GRIM_BATOL, 0)).is_none());
}

#[test]
fn gm_reset_expires_unused_locks_at_the_last_reset() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON);
    locks.update_for_player(1, &heroic(), &umbriss_kill(5), SUNDAY_NOON);
    assert!(
        locks
            .reset_for_player(
                1,
                None,
                |lock| lock.instance_id == 5,
                |_| Some(heroic()),
                SUNDAY_NOON
            )
            .is_empty()
    );
    let reset = locks.reset_for_player(
        1,
        Some(GRIM_BATOL),
        |_| false,
        |_| Some(heroic()),
        SUNDAY_NOON,
    );
    assert_eq!(reset, vec![(GRIM_BATOL, 0)]);
    assert_eq!(
        locks.lock(1, (GRIM_BATOL, 0)).unwrap().expiry,
        SUNDAY_RESET - 86_400
    );
    assert!(locks.find_active_lock(1, &heroic(), SUNDAY_NOON).is_none());
}

#[test]
fn saved_locks_round_trip_without_temporary_locks() {
    let mut locks = InstanceLocks::default();
    locks.create_for_new_instance(1, &heroic(), 5, SUNDAY_NOON);
    let kill = umbriss_kill(5);
    locks.update_shared(&kill);
    locks.update_for_player(1, &heroic(), &kill, SUNDAY_NOON);
    locks.create_for_new_instance(2, &mythic(), 6, SUNDAY_NOON);

    let bytes = bitcode::encode(&locks.saved());
    let restored = InstanceLocks::from_saved(bitcode::decode(&bytes).unwrap());
    assert_eq!(
        restored.lock(1, (GRIM_BATOL, 0)),
        locks.lock(1, (GRIM_BATOL, 0))
    );
    assert_eq!(restored.instance_ids().collect::<Vec<_>>(), vec![5]);
    assert!(
        restored
            .find_active_lock(2, &mythic(), SUNDAY_NOON)
            .is_none()
    );
    assert_eq!(
        restored
            .initialization_data(1, &heroic(), SUNDAY_NOON)
            .map(|data| data.done_bosses),
        Some(1)
    );
}
