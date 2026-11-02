//! Keystone pull admission follows the instance copy's actual difficulty.
use super::*;
use crate::maps::InstanceManager;
use crate::maps::instances::{InstanceEntrant, Now};
use std::collections::BTreeSet;

fn pull_at_difficulty(difficulty: u32) -> Result<(bool, String), String> {
    let mut f = fixture();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = InstanceManager::default();
    let entrant = InstanceEntrant {
        character_id: 1,
        leader: 1,
        difficulty_id: if difficulty == 8 { 23 } else { 1 },
        ..Default::default()
    };
    let map = instances
        .create_map(
            &store,
            2993,
            entrant,
            Now {
                elapsed: 0.0,
                unix: 1_790_510_400,
            },
        )
        .unwrap()
        .map;
    if difficulty == 8 {
        instances
            .activate_challenge_difficulty(map, *store.map_difficulty(2993, 8).unwrap())
            .unwrap();
    }
    let guid =
        f.h.app
            .world()
            .get::<crate::zones::CreatureOrigin>(f.second)
            .unwrap()
            .guid;
    {
        let mut data = f.h.app.world_mut().resource_mut::<WorldData>();
        let spawn = data
            .spawns
            .iter_mut()
            .find(|spawn| spawn.guid == guid)
            .unwrap();
        spawn.map = 2993;
        spawn.spawn_difficulties = vec![8];
        data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    }
    let owner = f.h.caster;
    for unit in f.members.iter().copied().chain([owner, f.first, f.second]) {
        f.h.app.world_mut().entity_mut(unit).insert(map);
    }
    f.h.app
        .world_mut()
        .get_mut::<super::super::stockade_diagnostic::DiagnosticParty>(owner)
        .unwrap()
        .copy = map;
    f.h.app.insert_resource(instances);
    super::super::pull_history::sample_all(f.h.app.world_mut());
    super::super::simulator::pull(f.h.app.world_mut(), f.tank, &BTreeSet::from([guid]))
}

#[test]
fn difficulty_eight_source_is_admitted_only_in_the_keystone_copy() {
    let result = pull_at_difficulty(8).expect("the Keystone source must be present");
    assert!(
        result.0,
        "the ready Keystone party must admit its first source: {result:?}"
    );
    let error = pull_at_difficulty(1).unwrap_err();
    assert!(
        error.contains("Projected source GUID") && error.contains("is not present"),
        "{error}"
    );
}
