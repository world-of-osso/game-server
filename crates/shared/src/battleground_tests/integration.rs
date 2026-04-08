use super::*;

#[test]
fn wsg_full_pipeline_queue_to_rewards() {
    let mut q = BgQueue::default();
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }

    let bg_match = q.try_match(1).unwrap();
    assert!(q.is_empty());

    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    let mut flags = FlagObjective::default();
    for _ in 0..3 {
        flags.pickup(BgTeam::A, bg_match.team_a[0]);
        flags.capture(BgTeam::A, &mut inst, &tmpl);
    }

    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));

    let dist = distribute_rewards(&inst).unwrap();
    assert_eq!(dist.rewards.len(), 10);

    let winner = dist
        .rewards
        .iter()
        .find(|(p, _)| *p == bg_match.team_a[0])
        .unwrap()
        .1;
    let loser = dist
        .rewards
        .iter()
        .find(|(p, _)| *p == bg_match.team_b[0])
        .unwrap()
        .1;
    assert!(winner.honor > loser.honor);
    assert!(winner.marks > loser.marks);
}

#[test]
fn ab_full_pipeline_node_control_to_win() {
    let mut q = BgQueue::default();
    for i in 1..=16 {
        q.join_solo(i, 30, 2, 100).unwrap();
    }

    let bg_match = q.try_match(2).unwrap();

    let tmpl = arathi_basin();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    let mut nodes = NodeObjective::arathi_basin();
    for i in 0..5 {
        nodes.start_capture(i, BgTeam::A);
        nodes.finish_capture(i);
    }

    while inst.phase == BgPhase::InProgress {
        nodes.tick_resources(&mut inst, &tmpl);
    }

    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    assert!(inst.score_a.score >= 1600);

    let dist = distribute_rewards(&inst).unwrap();
    let winner = dist
        .rewards
        .iter()
        .find(|(p, _)| *p == bg_match.team_a[0])
        .unwrap()
        .1;
    assert_eq!(winner.honor, 150);
    assert_eq!(winner.marks, 3);
}

#[test]
fn av_full_pipeline_reinforcements_to_win() {
    let tmpl = alterac_valley();
    let bg_match = BgMatch {
        bg_id: 3,
        team_a: vec![1, 2],
        team_b: vec![3, 4],
    };
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    assert_eq!(inst.score_b.reinforcements, 600);

    for _ in 0..600 {
        on_kill_reinforcement(&mut inst, BgTeam::B);
    }

    assert_eq!(inst.phase, BgPhase::Ended);
    assert_eq!(inst.outcome, Some(BgOutcome::Winner(BgTeam::A)));
    assert_eq!(inst.score_b.reinforcements, 0);

    let dist = distribute_rewards(&inst).unwrap();
    let winner = dist.rewards.iter().find(|(p, _)| *p == 1).unwrap().1;
    let loser = dist.rewards.iter().find(|(p, _)| *p == 3).unwrap().1;
    assert_eq!(winner.honor, 150);
    assert_eq!(loser.honor, 50);
}

#[test]
fn deserter_lifecycle_leave_block_expire_rejoin() {
    let mut q = BgQueue::default();
    let mut deserters = BgDeserterTracker::default();

    for i in 1..=10 {
        q.join_solo(i, 30, 1, 1000).unwrap();
    }
    let bg_match = q.try_match(1).unwrap();
    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    leave_bg(&mut inst, &mut deserters, bg_match.team_a[0], 2000);
    assert!(deserters.is_deserter(bg_match.team_a[0], 2000));

    assert_eq!(
        q.join_solo_checked(bg_match.team_a[0], 30, 1, 2000, &deserters),
        Err(BgQueueError::Deserter)
    );

    let after_expiry = 2000 + 900;
    assert!(!deserters.is_deserter(bg_match.team_a[0], after_expiry));
    assert!(
        q.join_solo_checked(bg_match.team_a[0], 30, 1, after_expiry, &deserters)
            .is_ok()
    );
}

#[test]
fn timeout_draw_both_teams_get_draw_rewards() {
    let mut q = BgQueue::default();
    for i in 1..=10 {
        q.join_solo(i, 30, 1, 100).unwrap();
    }
    let bg_match = q.try_match(1).unwrap();
    let tmpl = warsong_gulch();
    let mut inst = BgInstance::from_match(&bg_match, &tmpl);
    inst.start();

    inst.tick(tmpl.max_duration_secs, &tmpl);
    assert_eq!(inst.outcome, Some(BgOutcome::Draw));

    let dist = distribute_rewards(&inst).unwrap();
    for (_, reward) in &dist.rewards {
        assert_eq!(reward.honor, 75);
        assert_eq!(reward.marks, 2);
    }
}
