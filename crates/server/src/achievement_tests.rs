use super::*;

#[test]
fn level_achievement_snapshot_reports_progress_and_completed_ids() {
    let mut state = PlayerAchievementState::default();
    let completed = update_level_achievements(&mut state, 20);

    assert_eq!(completed.len(), 2);

    let snapshot = build_achievement_snapshot(20, &state);
    assert_eq!(snapshot.earned_ids, vec![1, 2]);
    assert_eq!(
        snapshot.progress,
        vec![
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 1,
                current: 10,
                required: 10,
                completed: true,
            },
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 2,
                current: 20,
                required: 20,
                completed: true,
            },
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 3,
                current: 20,
                required: 40,
                completed: false,
            },
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 4,
                current: 20,
                required: 60,
                completed: false,
            },
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 5,
                current: 20,
                required: 70,
                completed: false,
            },
            shared::protocol::AchievementProgressSnapshot {
                achievement_id: 6,
                current: 20,
                required: 80,
                completed: false,
            },
        ]
    );
}

#[test]
fn level_achievements_are_awarded_only_once() {
    let mut state = PlayerAchievementState::default();
    let first = update_level_achievements(&mut state, 10);
    let second = update_level_achievements(&mut state, 10);

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert_eq!(state.earned_ids, std::collections::BTreeSet::from([1]));
}

#[test]
fn achievement_state_round_trips_through_persistence() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);

    let state = PlayerAchievementState {
        earned_ids: std::collections::BTreeSet::from([1, 3, 6]),
    };
    crate::persistence::save_achievements(&db, 42, &state);

    let loaded = crate::persistence::load_achievements(&db, 42);
    assert_eq!(loaded, state);
}
