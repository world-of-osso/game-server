use super::*;
use crate::maps::instances::{InstanceEntrant, InstanceResetResult, Now};
use crate::maps::locks::InstanceLocks;

#[test]
fn neltharus_follower_checkpoint_restores_difficulty_and_position_on_login() {
    use bevy::ecs::system::RunSystemOnce;
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    let entrant = InstanceEntrant {
        character_id: 37,
        leader: 37,
        difficulty_id: 205,
        ..Default::default()
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_790_000_000,
    };
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances.create_map(&maps, 2519, entrant, now).unwrap().map;
    instances.update_instance_lock(copy, &[], 12, Some(2), now.unix);
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        let normal: Vec<_> = instances.normal_locks.values().cloned().collect();
        crate::maps::persistence::save_instance_locks(&db, &instances.locks.saved(), &normal);
    }
    let db = redb::Database::open(file.path()).unwrap();
    let locks = InstanceLocks::from_saved(crate::maps::persistence::load_instance_locks(&db));
    let normal = crate::maps::persistence::load_normal_instance_locks(&db);
    let mut app = App::new();
    app.insert_resource(maps)
        .insert_resource(crate::maps::InstanceManager::with_locks_and_normal(
            locks, normal,
        ))
        .insert_resource(Time::<()>::default());
    let position = shared::components::Position {
        x: -248.251,
        y: -468.9084,
        z: 123.114,
    };
    let place = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands, mut login: crate::maps::LoginMaps| {
                login.resolve(
                    &mut commands,
                    37,
                    1,
                    crate::maps::RaidDifficulty::default(),
                    2519,
                    position,
                )
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(place.map, copy);
    assert_eq!(place.position, position);
    let instances = app.world().resource::<crate::maps::InstanceManager>();
    assert_eq!(instances.copy_difficulty(copy).unwrap().difficulty_id, 205);
    assert_eq!(instances.initial_done_bosses(copy), 12);
    assert_eq!(instances.completed_encounters(copy), 4);
}

#[test]
fn dungeonach_normal_stockade_checkpoint_reopens_and_manual_reset_clears_it() {
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    assert!(!maps.map_difficulty(34, 1).unwrap().has_reset_schedule());
    assert!(
        maps.dungeon_encounters
            .values()
            .any(|encounter| encounter.order_index < 0)
    );
    let entrant = InstanceEntrant {
        character_id: 42,
        group: None,
        leader: 42,
        difficulty_id: 1,
        ..Default::default()
    };
    let now = Now {
        elapsed: 0.0,
        unix: 1_790_000_000,
    };
    let mut instances = crate::maps::InstanceManager::default();
    let copy = instances.create_map(&maps, 34, entrant, now).unwrap().map;
    let player = World::new().spawn_empty().id();
    instances.update_instance_lock(copy, &[(player, 42)], 1 << 2, Some(3), now.unix);
    assert!(
        instances
            .locks
            .find_active_lock(42, maps.map_difficulty(34, 1).unwrap(), now.unix)
            .is_none()
    );
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let db = redb::Database::create(file.path()).unwrap();
        crate::persistence::create_tables(&db);
        let normal: Vec<_> = instances.normal_locks.values().cloned().collect();
        crate::maps::persistence::save_instance_locks(&db, &instances.locks.saved(), &normal);
    }
    let db = redb::Database::open(file.path()).unwrap();
    let locks = InstanceLocks::from_saved(crate::maps::persistence::load_instance_locks(&db));
    let normal = crate::maps::persistence::load_normal_instance_locks(&db);
    let mut restored = crate::maps::InstanceManager::with_locks_and_normal(locks, normal);
    let resumed = restored.create_map(&maps, 34, entrant, now).unwrap().map;
    assert_eq!(resumed, copy);
    assert_eq!(restored.initial_done_bosses(resumed), 1 << 2);
    assert_eq!(restored.completed_encounters(resumed), 1 << 3);
    // Occupied normal copies cannot reset, but a completed boss never creates a scheduled bind.
    restored.add_player(resumed, player);
    assert_eq!(
        restored.reset_instances(entrant, false, now),
        vec![(34, InstanceResetResult::NotEmpty)]
    );
    restored.remove_player(resumed, player, now);
    assert_eq!(
        restored.reset_instances(entrant, false, now),
        vec![(34, InstanceResetResult::Success)]
    );
    assert!(restored.normal_locks.is_empty());
    let normal: Vec<_> = restored.normal_locks.values().cloned().collect();
    crate::maps::persistence::save_instance_locks(&db, &restored.locks.saved(), &normal);
    assert!(crate::maps::persistence::load_normal_instance_locks(&db).is_empty());
    restored.take_expired(2.0);
    let fresh = restored
        .create_map(
            &maps,
            34,
            entrant,
            Now {
                elapsed: 2.0,
                unix: now.unix + 2,
            },
        )
        .unwrap()
        .map;
    assert_ne!(fresh, copy);
    assert_eq!(restored.completed_encounters(fresh), 0);
}
