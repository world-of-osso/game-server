use super::*;

#[test]
fn rewards_none_when_not_ended() {
    let (inst, _) = wsg_instance();
    assert!(distribute_rewards(&inst).is_none());
}

#[test]
fn rewards_winner_gets_more() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 150);
    assert_eq!(a_reward.marks, 3);
    assert_eq!(b_reward.honor, 50);
    assert_eq!(b_reward.marks, 1);
}

#[test]
fn rewards_team_b_wins() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::B, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 50);
    assert_eq!(b_reward.honor, 150);
}

#[test]
fn rewards_draw() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.tick(tmpl.max_duration_secs, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let a_reward = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let b_reward = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;

    assert_eq!(a_reward.honor, 75);
    assert_eq!(a_reward.marks, 2);
    assert_eq!(b_reward.honor, 75);
    assert_eq!(b_reward.marks, 2);
}

#[test]
fn rewards_all_players_included() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    assert_eq!(dist.rewards.len(), 10);
}

#[test]
fn rewards_loser_still_gets_marks() {
    let (mut inst, tmpl) = wsg_instance();
    inst.start();
    inst.add_score(BgTeam::A, 3, &tmpl);

    let dist = distribute_rewards(&inst).unwrap();
    let loser = dist.rewards.iter().find(|(p, _)| *p == 6).unwrap().1;
    assert!(loser.marks > 0, "losers should still get marks");
    assert!(loser.honor > 0, "losers should still get honor");
}

#[test]
fn deserter_applied_and_checked() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(tracker.is_deserter(1, 10000));
    assert!(tracker.is_deserter(1, 10899));
    assert!(!tracker.is_deserter(1, 10900));
}

#[test]
fn deserter_remaining_time() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert_eq!(tracker.remaining(1, 10000), 900);
    assert_eq!(tracker.remaining(1, 10500), 400);
    assert_eq!(tracker.remaining(1, 10900), 0);
    assert_eq!(tracker.remaining(2, 10000), 0);
}

#[test]
fn deserter_not_active_for_others() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    assert!(!tracker.is_deserter(2, 10000));
}

#[test]
fn deserter_cleanup_removes_expired() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    tracker.apply(2, 10500);
    tracker.cleanup(10900);
    assert!(!tracker.is_deserter(1, 10900));
    assert!(tracker.is_deserter(2, 10900));
}

#[test]
fn deserter_refresh_extends() {
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);
    tracker.apply(1, 10500);
    assert!(tracker.is_deserter(1, 11000));
    assert!(!tracker.is_deserter(1, 11400));
}

#[test]
fn leave_bg_applies_deserter() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(leave_bg(&mut inst, &mut tracker, 1, 5000));
    assert!(tracker.is_deserter(1, 5000));
    assert!(!inst.team_a_players.contains(&1));
    assert_eq!(inst.team_a_players.len(), 4);
}

#[test]
fn leave_bg_team_b_player() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(leave_bg(&mut inst, &mut tracker, 6, 5000));
    assert!(tracker.is_deserter(6, 5000));
    assert!(!inst.team_b_players.contains(&6));
}

#[test]
fn leave_bg_not_in_match() {
    let (mut inst, _) = wsg_instance();
    inst.start();
    let mut tracker = BgDeserterTracker::default();

    assert!(!leave_bg(&mut inst, &mut tracker, 99, 5000));
    assert!(!tracker.is_deserter(99, 5000));
}

#[test]
fn leave_bg_not_in_progress() {
    let (mut inst, _) = wsg_instance();
    let mut tracker = BgDeserterTracker::default();

    assert!(!leave_bg(&mut inst, &mut tracker, 1, 5000));
    assert!(!tracker.is_deserter(1, 5000));
}

#[test]
fn deserter_blocks_queue_join() {
    let mut q = BgQueue::default();
    let mut tracker = BgDeserterTracker::default();
    tracker.apply(1, 10000);

    assert_eq!(
        q.join_solo_checked(1, 30, 1, 10000, &tracker),
        Err(BgQueueError::Deserter)
    );

    assert!(q.join_solo_checked(1, 30, 1, 10900, &tracker).is_ok());
}
