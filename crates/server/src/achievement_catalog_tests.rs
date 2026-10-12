use super::*;
use rusqlite::Connection;

fn catalog() -> AchievementCatalog {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    load_achievement_catalog(&db).unwrap()
}

#[test]
fn achievements2_other_character_earned_dependencies_do_not_grant_character_meta() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    let mut account = PlayerAchievementState::default();
    account.earned_ids.extend([4788, 4789]);
    state.account = Some(Box::new(account));
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    assert!(!state.earned_ids.contains(&1205));
}

#[test]
fn achievements2_holiday_and_missing_spell_assets_remain_untrackable() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::CastSpell(62049), None, 1);
    assert!(!state.criteria_progress.contains_key(&11118));
    catalog.update(&mut state, CriteriaEvent::BeSpellTarget(132022), None, 1);
    assert!(!state.criteria_progress.contains_key(&533));
}

#[test]
fn achievements2_pinned_snapshot_trackable_union_matches_census() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    state.account = Some(Box::new(PlayerAchievementState::default()));
    let union: BTreeSet<_> = [1, 2]
        .into_iter()
        .flat_map(|race| {
            catalog
                .snapshot(&state, race)
                .progress
                .into_iter()
                .map(|row| row.achievement_id)
        })
        .collect();
    assert_eq!(union.len(), 3706);
}

#[test]
fn achievements_dependencies_complete_from_historical_earned_ids_once() {
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::ReachLevel(1), None, 1)
            .is_empty()
    );
    // Retail Hero of Shattrath references these historical earned IDs.
    state.earned_ids.extend([4788, 4789]);
    let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    for id in [1205] {
        assert!(
            state.earned_ids.contains(&id),
            "missing dependency achievement {id}"
        );
        assert_eq!(
            toasts
                .iter()
                .filter(|toast| toast.achievement_id == id)
                .count(),
            1
        );
    }
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::ReachLevel(1), None, 1)
            .is_empty()
    );
}

#[test]
fn achievements_quest_asset_and_repeat_credit_preserve_boolean_progress() {
    let catalog = catalog();
    println!(
        "trackable achievement snapshot rows: {}",
        catalog
            .snapshot(&PlayerAchievementState::default(), 1)
            .progress
            .len()
    );
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::CompleteQuest(7630), None, 1);
    assert!(!state.earned_ids.contains(&2357));
    let toasts = catalog.update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1);
    assert!(toasts.iter().any(|toast| toast.achievement_id == 2357));
    assert_eq!(state.criteria_progress[&8905], 1);
    assert!(
        catalog
            .update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1)
            .is_empty()
    );
    assert_eq!(state.criteria_progress[&8905], 1);
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
        catalog.update(&mut state, CriteriaEvent::ReachLevel(level - 1), None, 1);
        assert!(!state.earned_ids.contains(&id));
        let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(level), None, 1);
        assert!(toasts.iter().any(|toast| toast.achievement_id == id));
        assert!(state.earned_ids.contains(&id));
        assert!(
            catalog
                .update(&mut state, CriteriaEvent::ReachLevel(level), None, 1)
                .is_empty()
        );
    }
    assert_eq!(state.criteria_progress[&5212], 80);
    let snapshot = catalog.snapshot(&state, 1);
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
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1140), None, 1);
    assert!(!state.criteria_progress.contains_key(&39291));
    catalog.update(&mut state, CriteriaEvent::DefeatEncounter(1141), None, 1);
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
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None, 1);
    assert!(catalog.tree_progress(900002, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    assert!(!catalog.tree_progress(900003, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(1), None, 1);
    // Count counts direct children with positive progress, not completed level thresholds.
    assert!(catalog.tree_progress(900003, &state).completed);
    assert!(!catalog.tree_progress(900001, &state).completed);
    catalog.update(&mut state, CriteriaEvent::ReachLevel(10), None, 1);
    assert!(catalog.tree_progress(900001, &state).completed);
}

#[test]
fn dungeonach_placeholder_ids_migrate_once_in_redb() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let table = redb::TableDefinition::<u64, &[u8]>::new("achievements");
    let txn = db.begin_write().unwrap();
    {
        let mut table = txn.open_table(table).unwrap();
        table
            .insert(42, br#"{"earned_ids":[1,6]}"#.as_slice())
            .unwrap();
    }
    txn.commit().unwrap();
    let first = crate::persistence::load_achievements(&db, 42);
    assert_eq!(first.earned_ids, BTreeSet::from([6, 19459]));
    let second = crate::persistence::load_achievements(&db, 42);
    assert_eq!(first, second);
}

#[test]
fn dungeonach_criteria_progress_persists_across_database_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let catalog = catalog();
    let mut state = PlayerAchievementState::default();
    catalog.update(&mut state, CriteriaEvent::ReachLevel(20), None, 1);
    catalog.update(&mut state, CriteriaEvent::KillCreature(46254), None, 1);
    catalog.update(&mut state, CriteriaEvent::CompleteQuest(7631), None, 1);
    assert!(state.earned_ids.contains(&2357));
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
