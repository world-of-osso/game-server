use super::*;

#[test]
fn queue_solo() {
    let mut q = DungeonQueue::default();
    assert!(q.queue_solo(1, GroupRole::Tank, vec![100], 1000).is_ok());
    assert_eq!(q.len(), 1);
    assert!(q.is_queued(1));
}

#[test]
fn queue_group() {
    let mut q = DungeonQueue::default();
    q.queue_group(
        1,
        vec![1, 2, 3],
        vec![GroupRole::Tank, GroupRole::Healer, GroupRole::Dps],
        vec![100],
        1000,
    )
    .unwrap();
    assert!(q.is_queued(1));
    assert!(q.is_queued(2));
    assert!(q.is_queued(3));
}

#[test]
fn queue_already_queued() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Dps, vec![100], 1000).unwrap();
    assert_eq!(
        q.queue_solo(1, GroupRole::Dps, vec![100], 1000),
        Err(QueueError::AlreadyQueued)
    );
}

#[test]
fn queue_no_dungeons() {
    let mut q = DungeonQueue::default();
    assert_eq!(
        q.queue_solo(1, GroupRole::Dps, vec![], 1000),
        Err(QueueError::NoDungeons)
    );
}

#[test]
fn dequeue() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Tank, vec![100], 1000).unwrap();
    assert!(q.dequeue(1));
    assert!(!q.is_queued(1));
    assert!(q.is_empty());
}

#[test]
fn dequeue_nonexistent() {
    let mut q = DungeonQueue::default();
    assert!(!q.dequeue(99));
}

#[test]
fn average_wait_time() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Dps, vec![100], 100).unwrap();
    q.queue_solo(2, GroupRole::Dps, vec![100], 200).unwrap();
    let avg = q.average_wait(400);
    // Player 1: 300s, Player 2: 200s → avg 250
    assert!((avg - 250.0).abs() < 0.01);
}

// --- Matchmaking tests ---

fn queue_5_players(q: &mut DungeonQueue) {
    q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
    q.queue_solo(2, GroupRole::Healer, vec![100], 0).unwrap();
    q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(5, GroupRole::Dps, vec![100], 0).unwrap();
}

#[test]
fn match_1t_1h_3d() {
    let mut q = DungeonQueue::default();
    queue_5_players(&mut q);
    let m = q.try_match().unwrap();
    assert_eq!(m.dungeon_id, 100);
    assert_eq!(m.tank, 1);
    assert_eq!(m.healer, 2);
    assert_eq!(m.dps.len(), 3);
    assert!(q.is_empty()); // all removed
}

#[test]
fn no_match_missing_tank() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Healer, vec![100], 0).unwrap();
    q.queue_solo(2, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
    assert!(q.try_match().is_none());
}

#[test]
fn no_match_missing_healer() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
    q.queue_solo(2, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
    assert!(q.try_match().is_none());
}

#[test]
fn no_match_different_dungeons() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
    q.queue_solo(2, GroupRole::Healer, vec![200], 0).unwrap(); // different dungeon
    q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(4, GroupRole::Dps, vec![100], 0).unwrap();
    q.queue_solo(5, GroupRole::Dps, vec![100], 0).unwrap();
    assert!(q.try_match().is_none());
}

#[test]
fn match_removes_from_queue() {
    let mut q = DungeonQueue::default();
    queue_5_players(&mut q);
    q.queue_solo(6, GroupRole::Dps, vec![200], 0).unwrap();
    q.try_match();
    assert_eq!(q.len(), 1); // player 6 remains
    assert!(q.is_queued(6));
}

#[test]
fn match_overlapping_dungeons() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Tank, vec![100, 200], 0).unwrap();
    q.queue_solo(2, GroupRole::Healer, vec![200, 300], 0)
        .unwrap();
    q.queue_solo(3, GroupRole::Dps, vec![100, 200], 0).unwrap();
    q.queue_solo(4, GroupRole::Dps, vec![200], 0).unwrap();
    q.queue_solo(5, GroupRole::Dps, vec![200, 300], 0).unwrap();
    let m = q.try_match().unwrap();
    // All overlap on dungeon 200
    assert_eq!(m.dungeon_id, 200);
}

// --- Teleport tests ---

fn test_dungeon() -> DungeonDef {
    DungeonDef {
        id: 100,
        name: "Deadmines".into(),
        map_id: 36,
        entrance_x: -11208.0,
        entrance_y: 1673.0,
        entrance_z: 24.0,
        min_level: 15,
        max_level: 21,
    }
}

#[test]
fn teleport_all_5_players() {
    let m = MatchResult {
        dungeon_id: 100,
        tank: 1,
        healer: 2,
        dps: vec![3, 4, 5],
    };
    let dungeon = test_dungeon();
    let orders = teleport_orders(&m, &dungeon);
    assert_eq!(orders.len(), 5);
    for order in &orders {
        assert_eq!(order.map_id, 36);
        assert_eq!(order.x, -11208.0);
    }
    let players: Vec<u64> = orders.iter().map(|o| o.player).collect();
    assert!(players.contains(&1));
    assert!(players.contains(&2));
    assert!(players.contains(&5));
}

#[test]
fn dungeon_def_level_range() {
    let d = test_dungeon();
    assert_eq!(d.min_level, 15);
    assert_eq!(d.max_level, 21);
}

// --- Deserter tests ---

#[test]
fn deserter_applied_and_checked() {
    let mut tracker = DeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(tracker.is_deserter(1, 10000));
    assert!(tracker.is_deserter(1, 11799)); // 1799s later, still active
    assert!(!tracker.is_deserter(1, 11800)); // 1800s = expired
}

#[test]
fn deserter_remaining() {
    let mut tracker = DeserterTracker::default();
    tracker.apply(1, 10000);
    assert_eq!(tracker.remaining(1, 10000), 1800);
    assert_eq!(tracker.remaining(1, 10500), 1300);
    assert_eq!(tracker.remaining(1, 12000), 0);
}

#[test]
fn deserter_not_active_for_others() {
    let mut tracker = DeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(!tracker.is_deserter(2, 10000));
}

#[test]
fn deserter_cleanup() {
    let mut tracker = DeserterTracker::default();
    tracker.apply(1, 10000);
    tracker.apply(2, 11000);
    tracker.cleanup(11800); // player 1 expired, player 2 still active
    assert!(!tracker.is_deserter(1, 11800));
    assert!(tracker.is_deserter(2, 11800));
}

#[test]
fn deserter_refresh_extends() {
    let mut tracker = DeserterTracker::default();
    tracker.apply(1, 10000); // expires 11800
    tracker.apply(1, 11000); // refreshes to 12800
    assert!(tracker.is_deserter(1, 12000));
}

// --- Lockout tests ---

#[test]
fn heroic_daily_lockout() {
    let mut lockouts = PlayerLockouts::default();
    let now = 100_000; // mid-day
    lockouts.add(100, LockoutType::HeroicDaily, now);
    assert!(lockouts.is_locked(100, now));
    assert!(lockouts.is_locked(100, now + 1000)); // still same day
}

#[test]
fn heroic_resets_at_day_boundary() {
    let mut lockouts = PlayerLockouts::default();
    let now = 100_000;
    lockouts.add(100, LockoutType::HeroicDaily, now);
    let reset = lockouts.reset_time(100, now).unwrap();
    // Next day boundary after 100000: ceil(100000/86400)*86400 = 2*86400 = 172800
    assert_eq!(reset, 172_800);
    assert!(!lockouts.is_locked(100, 172_800)); // expired at reset
}

#[test]
fn mythic_weekly_lockout() {
    let mut lockouts = PlayerLockouts::default();
    let now = 100_000;
    lockouts.add(200, LockoutType::MythicWeekly, now);
    assert!(lockouts.is_locked(200, now + 3 * DAY_SECS)); // 3 days later still locked
}

#[test]
fn mythic_resets_at_week_boundary() {
    let mut lockouts = PlayerLockouts::default();
    let now = 100_000;
    lockouts.add(200, LockoutType::MythicWeekly, now);
    let reset = lockouts.reset_time(200, now).unwrap();
    assert_eq!(reset, WEEK_SECS); // first week boundary
    assert!(!lockouts.is_locked(200, WEEK_SECS));
}

#[test]
fn different_dungeons_independent() {
    let mut lockouts = PlayerLockouts::default();
    lockouts.add(100, LockoutType::HeroicDaily, 50_000);
    assert!(lockouts.is_locked(100, 50_000));
    assert!(!lockouts.is_locked(200, 50_000)); // different dungeon
}

#[test]
fn cleanup_removes_expired() {
    let mut lockouts = PlayerLockouts::default();
    lockouts.add(100, LockoutType::HeroicDaily, 10_000);
    lockouts.add(200, LockoutType::MythicWeekly, 10_000);
    lockouts.cleanup(200_000); // heroic expired, mythic still active
    assert_eq!(lockouts.lockouts.len(), 1);
    assert_eq!(lockouts.lockouts[0].dungeon_id, 200);
}

// --- Satchel tests ---

#[test]
fn tank_in_demand_with_many_dps() {
    let mut q = DungeonQueue::default();
    for i in 1..=10 {
        q.queue_solo(i, GroupRole::Dps, vec![100], 0).unwrap();
    }
    let demand = in_demand_roles(&q);
    assert!(demand.contains(&GroupRole::Tank));
    assert!(demand.contains(&GroupRole::Healer));
}

#[test]
fn no_demand_when_balanced() {
    let mut q = DungeonQueue::default();
    q.queue_solo(1, GroupRole::Tank, vec![100], 0).unwrap();
    q.queue_solo(2, GroupRole::Healer, vec![100], 0).unwrap();
    q.queue_solo(3, GroupRole::Dps, vec![100], 0).unwrap();
    let demand = in_demand_roles(&q);
    assert!(!demand.contains(&GroupRole::Dps));
}

#[test]
fn empty_queue_both_in_demand() {
    let q = DungeonQueue::default();
    let demand = in_demand_roles(&q);
    assert!(demand.contains(&GroupRole::Tank));
    assert!(demand.contains(&GroupRole::Healer));
}

#[test]
fn satchel_for_in_demand_role() {
    let demand = vec![GroupRole::Tank];
    assert!(satchel_for_role(GroupRole::Tank, &demand).is_some());
    assert!(satchel_for_role(GroupRole::Dps, &demand).is_none());
}

#[test]
fn satchel_reward_values() {
    let reward = satchel_for_role(GroupRole::Tank, &[GroupRole::Tank]).unwrap();
    assert_eq!(reward.gold, 500_000);
    assert_eq!(reward.item_id, 54516);
}
