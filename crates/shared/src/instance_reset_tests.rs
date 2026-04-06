// Instance reset and rate limiting tests (extracted from instance_tests.rs)

// --- Instance reset ---

#[test]
fn leader_reset_empty_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    // Player leaves
    mgr.get_mut(id).unwrap().remove_player(100);

    let removed = mgr.leader_reset(id, 100).unwrap();
    assert_eq!(removed.instance_id, id);
    assert!(mgr.is_empty());
}

#[test]
fn leader_reset_fails_with_players_inside() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100, 101], 1000).unwrap();
    assert_eq!(
        mgr.leader_reset(id, 100),
        Err(InstanceError::PlayersStillInside)
    );
}

#[test]
fn leader_reset_fails_for_non_leader() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    mgr.get_mut(id).unwrap().remove_player(100);

    assert_eq!(
        mgr.leader_reset(id, 200),
        Err(InstanceError::NotGroupLeader)
    );
}

#[test]
fn leader_reset_unknown_instance() {
    let mut mgr = InstanceManager::new();
    assert_eq!(
        mgr.leader_reset(999, 100),
        Err(InstanceError::InstanceNotFound)
    );
}

#[test]
fn leader_reset_allows_new_instance() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id1 = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    mgr.get_mut(id1).unwrap().remove_player(100);
    mgr.leader_reset(id1, 100).unwrap();

    // Can create a fresh instance for the same map+difficulty
    let id2 = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 2000).unwrap();
    assert_ne!(id1, id2);
}

#[test]
fn instance_reset_at_heroic_daily() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    let inst = mgr.get(id).unwrap();

    // Heroic dungeon: daily reset
    assert_eq!(inst.reset_at(&reg), Some(DAY_SECS));
    assert!(!inst.is_expired(&reg, DAY_SECS - 1));
    assert!(inst.is_expired(&reg, DAY_SECS));
}

#[test]
fn instance_reset_at_normal_none() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();
    let inst = mgr.get(id).unwrap();

    // Normal dungeon: no scheduled reset
    assert_eq!(inst.reset_at(&reg), None);
    assert!(!inst.is_expired(&reg, u64::MAX));
}

#[test]
fn instance_reset_at_raid_weekly() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    let id = mgr.create_instance(&reg, 409, Difficulty::Mythic, 100, &[100], 1000).unwrap();
    let inst = mgr.get(id).unwrap();

    assert_eq!(inst.reset_at(&reg), Some(WEEK_SECS));
}

#[test]
fn scheduled_reset_destroys_expired() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    // Heroic dungeon (daily reset) and mythic raid (weekly reset)
    let heroic_id = mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    let mythic_id = mgr.create_instance(&reg, 409, Difficulty::Mythic, 200, &[200], 1000).unwrap();

    // After daily reset but before weekly
    let destroyed = mgr.scheduled_reset(&reg, DAY_SECS);
    assert_eq!(destroyed.len(), 1);
    assert_eq!(destroyed[0].instance_id, heroic_id);

    // Mythic raid still alive
    assert!(mgr.get(mythic_id).is_some());
    assert_eq!(mgr.len(), 1);
}

#[test]
fn scheduled_reset_destroys_all_expired() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    mgr.create_instance(&reg, 409, Difficulty::Mythic, 200, &[200], 1000).unwrap();

    // After weekly reset — both expired
    let destroyed = mgr.scheduled_reset(&reg, WEEK_SECS);
    assert_eq!(destroyed.len(), 2);
    assert!(mgr.is_empty());
}

#[test]
fn scheduled_reset_skips_normal_dungeons() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Normal, 100, &[100], 1000).unwrap();

    // Normal dungeons have no reset timer — should never be auto-destroyed
    let destroyed = mgr.scheduled_reset(&reg, u64::MAX);
    assert!(destroyed.is_empty());
    assert_eq!(mgr.len(), 1);
}

#[test]
fn scheduled_reset_cleans_group_index() {
    let reg = sample_registry();
    let mut mgr = InstanceManager::new();

    mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], 1000).unwrap();
    mgr.scheduled_reset(&reg, DAY_SECS);

    // Group can create a new instance after scheduled reset
    mgr.create_instance(&reg, 33, Difficulty::Heroic, 100, &[100], DAY_SECS + 1).unwrap();
    assert_eq!(mgr.len(), 1);
}

// --- Instance rate limiting ---

#[test]
fn rate_limiter_allows_up_to_five() {
    let mut limiter = InstanceRateLimiter::new();
    // First 4 entries: still under limit
    for i in 0..4 {
        limiter.record_entry(1, 1000 + i).unwrap();
    }
    assert!(limiter.can_enter(1, 1004));

    // 5th entry: succeeds but hits the limit
    limiter.record_entry(1, 1004).unwrap();
    assert_eq!(limiter.recent_count(1, 1005), 5);
    assert!(!limiter.can_enter(1, 1005));
}

#[test]
fn rate_limiter_rejects_sixth() {
    let mut limiter = InstanceRateLimiter::new();
    for i in 0..5 {
        limiter.record_entry(1, 1000 + i).unwrap();
    }
    assert_eq!(
        limiter.record_entry(1, 1005),
        Err(InstanceError::InstanceLimitReached)
    );
}

#[test]
fn rate_limiter_expires_after_one_hour() {
    let mut limiter = InstanceRateLimiter::new();
    for i in 0..5 {
        limiter.record_entry(1, 1000 + i).unwrap(); // t=1000..1004
    }
    // 1 hour after the *last* entry (1004 + 3600 + 1 = 4605)
    let after_hour = 4605;
    assert!(limiter.can_enter(1, after_hour));
    assert_eq!(limiter.recent_count(1, after_hour), 0);
    limiter.record_entry(1, after_hour).unwrap();
}

#[test]
fn rate_limiter_rolling_window() {
    let mut limiter = InstanceRateLimiter::new();
    // Enter 5 instances spread over 50 minutes
    for i in 0..5 {
        limiter.record_entry(1, i * 600).unwrap(); // 0, 600, 1200, 1800, 2400
    }
    // At t=3601, the first entry (t=0) has expired
    assert!(limiter.can_enter(1, 3601));
    assert_eq!(limiter.recent_count(1, 3601), 4);
}

#[test]
fn rate_limiter_independent_accounts() {
    let mut limiter = InstanceRateLimiter::new();
    for i in 0..5 {
        limiter.record_entry(1, 1000 + i).unwrap();
    }
    // Account 2 is unaffected
    assert!(limiter.can_enter(2, 1005));
    limiter.record_entry(2, 1005).unwrap();
}

#[test]
fn rate_limiter_cooldown_remaining() {
    let mut limiter = InstanceRateLimiter::new();
    for i in 0..5 {
        limiter.record_entry(1, 1000 + i).unwrap();
    }
    // Oldest entry at t=1000 expires at t=4600
    assert_eq!(limiter.cooldown_remaining(1, 2000), 2600);
    // Not at limit → 0
    assert_eq!(limiter.cooldown_remaining(2, 2000), 0);
}

#[test]
fn rate_limiter_cleanup() {
    let mut limiter = InstanceRateLimiter::new();
    limiter.record_entry(1, 1000).unwrap();
    limiter.record_entry(2, 2000).unwrap();

    // Well after both entries have expired (1h+ past the latest)
    limiter.cleanup(6000);
    assert_eq!(limiter.recent_count(1, 6000), 0);
    assert_eq!(limiter.recent_count(2, 6000), 0);
}

#[test]
fn rate_limiter_new_account_has_zero_count() {
    let limiter = InstanceRateLimiter::new();
    assert_eq!(limiter.recent_count(999, 1000), 0);
    assert!(limiter.can_enter(999, 1000));
    assert_eq!(limiter.cooldown_remaining(999, 1000), 0);
}
