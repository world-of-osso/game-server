use super::*;
use rusqlite::Connection;

fn catalog() -> AchievementCatalog {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

#[test]
fn dungeonach_real_level_rows_grant_at_thresholds() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    for (level, id) in [
        (10, 6),
        (20, 7),
        (40, 9),
        (60, 14783),
        (70, 15805),
        (80, 19459),
    ] {
        catalog.update(&mut state, CriteriaEvent::ReachLevel(level - 1), None);
        assert!(!state.earned_ids.contains(&id));
        let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(level), None);
        assert!(toasts.iter().any(|toast| toast.achievement_id == id));
        assert!(state.earned_ids.contains(&id));
        assert!(
            catalog
                .update(&mut state, CriteriaEvent::ReachLevel(level), None)
                .is_empty()
        );
    }
    assert_eq!(state.criteria_progress[&5212], 80);
    let snapshot = catalog.snapshot(&state);
    let progress = snapshot
        .progress
        .iter()
        .find(|progress| progress.achievement_id == 19459)
        .unwrap();
    assert_eq!(
        (progress.current, progress.required, progress.completed),
        (80, 80, true)
    );
}

#[test]
fn dungeonach_real_encounter_criteria_match_only_their_asset() {
    let catalog = catalog();
    let criterion = &catalog.criteria[&39291];
    assert_eq!((criterion.kind, criterion.asset), (165, 1141));
    assert!(criterion.supported);
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1140), None);
    assert!(!state.criteria_progress.contains_key(&39291));
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1141), None);
    assert_eq!(state.criteria_progress[&39291], 1);
}

#[test]
fn dungeonach_tree_all_any_and_count_operators() {
    let mut catalog = catalog();
    // Concrete Retail Hogger kill and reach-level leaves, reused under fixture parents.
    assert_eq!(catalog.trees[&3386].criteria_id, 18527);
    assert_eq!(catalog.trees[&2051].criteria_id, 5212);
    let leaves = vec![3386, 2051];
    for (id, operator, amount) in [
        (900001, TreeOperator::All, 0),
        (900002, TreeOperator::Any, 1),
        (900003, TreeOperator::CountDirectChildren, 2),
    ] {
        catalog.trees.insert(
            id,
            CriteriaTree {
                criteria_id: 0,
                amount,
                operator,
            },
        );
        catalog.children.insert(id, leaves.clone());
    }
    let mut state = PlayerAchievementState::default();
    assert!(!catalog.tree_progress(900002, &state).completed);
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None);
    assert!(catalog.tree_progress(900002, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    assert!(!catalog.tree_progress(900003, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None);
    // Count counts direct children with positive progress, not completed level thresholds.
    assert!(catalog.tree_progress(900003, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(10), None);
    assert!(catalog.tree_progress(900001, &state).completed);
}

#[test]
fn dungeonach_criteria_progress_persists_across_database_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::ReachLevel(20), None);
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None);
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        crate::persistence::save_achievements(&db, 42, &state);
    }
    let db = redb::Database::open(file.path()).unwrap();
    assert_eq!(crate::persistence::load_achievements(&db, 42), state);
    // Existing placeholder rows migrate once, retaining the actual earned levels.
    let (old, migrated) =
        crate::achievement::decode_saved_achievement_state(br#"{"earned_ids":[1,6]}"#);
    assert!(migrated);
    assert_eq!(old.earned_ids, BTreeSet::from([6, 19459]));
    assert!(old.criteria_progress.is_empty());
    let bytes = serde_json::to_vec(&old).unwrap();
    let (again, migrated) = crate::achievement::decode_saved_achievement_state(&bytes);
    assert!(!migrated);
    assert_eq!(again, old);
}
