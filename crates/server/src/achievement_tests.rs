use super::*;
use rusqlite::Connection;

#[test]
fn dungeonach_retail_level_ids_replace_placeholders() {
    let db = Connection::open(crate::test_data::world_db_path()).unwrap();
    let catalog = load_achievement_catalog(&db).unwrap();
    let mut state = PlayerAchievementState::default();
    let toasts = catalog.update(&mut state, CriteriaEvent::ReachLevel(20), None);
    assert!(
        state.earned_ids.contains(&6),
        "Retail Level 10 is achievement 6"
    );
    assert!(
        state.earned_ids.contains(&7),
        "Retail Level 20 is achievement 7"
    );
    assert_eq!(
        toasts
            .iter()
            .find(|toast| toast.achievement_id == 6)
            .unwrap()
            .points,
        10
    );
}

#[test]
fn dungeonach_scheduled_instance_lock_round_trip_restores_bosses_and_encounters() {
    use crate::maps::instances::{InstanceEntrant, Now};
    use crate::maps::locks::InstanceLocks;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let difficulty = maps.map_difficulty(670, 2).unwrap();
    assert!(difficulty.has_reset_schedule());
    let entrant = InstanceEntrant {
        character_id: 42,
        group: None,
        leader: 42,
        difficulty_id: 2,
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_790_000_000,
    };
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances.create_map(&maps, 670, entrant, now).unwrap().map;
    let bit = maps.encounters_for_map(670, 2)[0].bit;
    instances.update_instance_lock(copy, &[(Entity::from_bits(1), 42)], 1, Some(bit), now.unix);
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        crate::maps::persistence::save_instance_locks(&db, &instances.locks.saved());
    }
    let db = redb::Database::open(file.path()).unwrap();
    let saved = crate::maps::persistence::load_instance_locks(&db);
    let mut restored = crate::maps::InstanceManager::with_locks(InstanceLocks::from_saved(saved));
    let resumed = restored
        .create_map(
            &maps,
            670,
            entrant,
            Now {
                elapsed: 1.0,
                unix: now.unix + 1,
            },
        )
        .unwrap()
        .map;
    assert_eq!(resumed, copy);
    assert_eq!(restored.initial_done_bosses(resumed), 1);
    assert_eq!(restored.completed_encounters(resumed), 1 << bit);
}
