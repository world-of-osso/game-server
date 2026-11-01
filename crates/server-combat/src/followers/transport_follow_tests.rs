//! Real navigation and production follower movement across a registered pyre.
use super::*;
use bevy::prelude::*;
use shared::components::Position;

#[test]
fn follower_walks_to_and_uses_pyre_after_owner_uses_it() {
    let from = Position {
        x: 4474.46,
        y: 13.525905,
        z: 4918.17,
    };
    let (mut h, follower, engine) = super::navigation_tests::stockade_fixture(from);
    let owner = h.caster;
    let map = crate::maps::InMap::continent(2825);
    let source = Position {
        x: 4480.69,
        y: 13.37,
        z: 4912.04,
    };
    h.app.world_mut().entity_mut(owner).insert((map, source));
    h.app.world_mut().entity_mut(follower).insert((map, from));
    let maps = crate::maps::store::load_map_store(
        &rusqlite::Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap(),
    )
    .unwrap();
    h.app.insert_resource(crate::ground::GroundMap::new(
        &engine,
        std::collections::HashMap::from([(2825, maps.map(2825).unwrap().wdt_file_data_id)]),
    ));
    let pyre = h
        .app
        .world_mut()
        .spawn((
            map,
            source,
            shared::protocol::GameObjectInfo {
                entry: 616377,
                go_type: 10,
                display_id: 0,
                name: "Ethereal Pyre".into(),
                scale: 1.0,
            },
        ))
        .id();
    crate::npc_interaction::nalorakk_pyre::traverse(h.app.world_mut(), owner, pyre).unwrap();
    assert_eq!(
        *h.app.world().get::<Position>(follower).unwrap(),
        from,
        "another unit must not teleport the follower"
    );
    let mut walked = false;
    for _ in 0..1600 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(follower).unwrap();
        if at.y < 30.0
            && Vec3::new(at.x, at.y, at.z).distance(Vec3::new(from.x, from.y, from.z)) > 0.2
        {
            walked = true;
        }
        if at.y > 90.0 {
            break;
        }
    }
    let at = *h.app.world().get::<Position>(follower).unwrap();
    assert!(walked, "follower must walk to the pyre");
    assert!(
        (at.x - 4339.33).abs() < 0.1
            && (at.y - 104.31152).abs() < 0.1
            && (at.z - 4947.34).abs() < 0.1,
        "follower did not use its own pyre interaction: {at:?}"
    );
}
