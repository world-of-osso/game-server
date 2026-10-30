//! Follower flight uses production movement and a real landing navmesh.
use super::navigation_tests::stockade_fixture;
use crate::ground::GroundMap;
use bevy::prelude::*;
use shared::components::{Mounted, PlayerMotion, Position};

fn areas(flags: u32) -> crate::area_table::AreaTable {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE area_table (ID INTEGER, ParentAreaID INTEGER, ContinentID INTEGER, Flags_0 INTEGER, FactionGroupMask INTEGER, Flags_1 INTEGER, AreaName_lang TEXT, MountFlags INTEGER);").unwrap();
    db.execute(
        "INSERT INTO area_table VALUES (9001,0,34,0,0,0,'Flight fixture',?1)",
        [flags],
    )
    .unwrap();
    crate::area_table::load_area_table(&db).unwrap()
}

#[test]
fn flying_follower_requires_area_permission_and_lands_on_leaders_navmesh() {
    let from = Position {
        x: 150.557,
        y: -34.85624,
        z: -45.8164,
    };
    let (mut h, follower, _) = stockade_fixture(from);
    let owner = h.caster;
    let landing = Position {
        x: 103.0,
        y: -34.85623,
        z: -76.0,
    };
    let in_air = Position {
        y: landing.y + 20.0,
        ..landing
    };
    h.app.insert_resource(GroundMap::with_area_samples(&[(
        34,
        Vec3::new(in_air.x, in_air.y, in_air.z),
        9001,
    )]));
    h.app.insert_resource(areas(0));
    h.app.world_mut().entity_mut(owner).insert((
        in_air,
        PlayerMotion(PlayerMotion::FLYING),
        Mounted {
            mount_display_id: 42,
            ..default()
        },
    ));
    h.advance(5);
    assert!(h.app.world().get::<Mounted>(follower).is_none());
    h.app.insert_resource(areas(2));
    for _ in 0..500 {
        h.advance(1);
    }
    let airborne = *h.app.world().get::<Position>(follower).unwrap();
    assert!(
        (airborne.y - in_air.y).abs() < 1.0,
        "follower did not follow in three dimensions: {airborne:?}"
    );
    assert_eq!(
        h.app
            .world()
            .get::<Mounted>(follower)
            .unwrap()
            .mount_display_id,
        42
    );
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((landing, PlayerMotion::default()));
    for _ in 0..500 {
        h.advance(1);
        if h.app.world().get::<Mounted>(follower).is_none() {
            break;
        }
        h.app
            .world_mut()
            .resource_mut::<crate::navigation::Navigation>()
            .drain_until_idle();
    }
    let landed = *h.app.world().get::<Position>(follower).unwrap();
    assert!(
        h.app.world().get::<Mounted>(follower).is_none(),
        "follower did not complete navmesh landing: {landed:?}"
    );
    assert!((landed.y - landing.y).abs() < navigation::AGENT_CLIMB);
    assert!((landed.x - landing.x).hypot(landed.z - landing.z) < 1.0);
}
