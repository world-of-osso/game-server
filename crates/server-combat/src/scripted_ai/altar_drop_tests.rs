use super::*;
use crate::followers::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use shared::components::Health;
use std::collections::HashMap;
use std::path::PathBuf;

fn position(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

#[test]
#[ignore = "requires Altar native ground and navmesh"]
fn liadrin_protects_only_the_follower_using_the_drop() {
    let root = PathBuf::from("/home/osso/Projects/world-of-osso/game-engine/data");
    let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        app.insert_resource(crate::ground::GroundMap::new(
            &root,
            HashMap::from([(MAP, 7_213_439)]),
        ));
        crate::navigation::install_fixture(
            app,
            crate::navigation::Navigation::new(&root.join("navmesh")),
        );
        app.add_systems(
            Update,
            (
                advance,
                super::super::fall_movement::advance_falls,
                super::super::fall_movement::release_finished_falls,
            )
                .chain(),
        );
    });
    let map = InMap::continent(MAP);
    let liadrin = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(liadrin).insert((
        Npc {
            template_id: LIADRIN,
            name: "Lady Liadrin".into(),
        },
        map,
        position(SOURCE),
    ));
    h.app.world_mut().entity_mut(follower).insert((
        map,
        position(SOURCE),
        Health {
            current: 1000.0,
            max: 1000.0,
        },
    ));
    let other = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(other).insert((
        map,
        position(SOURCE),
        Health {
            current: 1000.0,
            max: 1000.0,
        },
    ));
    let floor = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_fall_ground_in(map, Vec3::new(LANDING.x, SOURCE.y, LANDING.z));
    assert!(
        matches!(floor, crate::ground::GroundProbe::Supported(y) if (y - LANDING.y).abs() < 0.01)
    );
    traverse(h.app.world_mut(), follower, liadrin).unwrap();
    let mut protected = false;
    for _ in 0..400 {
        h.advance(1);
        protected |= crate::auras::is_immune(h.app.world(), follower, 1);
        if h.app.world().get::<AltarDrop>(follower).is_none() {
            break;
        }
    }
    let world = h.app.world();
    let landed = world.get::<Position>(follower).unwrap();
    assert!(
        protected,
        "Liadrin must apply real physical immunity during the fall"
    );
    assert!(
        (landed.y - LANDING.y).abs() < 0.3,
        "follower did not land: {landed:?}"
    );
    assert_eq!(world.get::<Health>(follower).unwrap().current, 1000.0);
    assert!(!crate::auras::is_immune(world, other, 1));
    assert_eq!(*world.get::<Position>(other).unwrap(), position(SOURCE));
}
