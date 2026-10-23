//! Approved2651 drop: actual native navigation, fall and follower movement.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use crate::ground::{GroundMap, GroundProbe};
use crate::maps::InMap;
use bevy::prelude::*;
use shared::components::{Health, Position};
use std::collections::HashMap;
use std::path::PathBuf;

const MAP: u32 = 2651;
const WDT: u32 = 5_217_425;
const UPPER: Vec3 = Vec3::new(535.1, 86.534424, 110.0);
const DEPARTURE: Vec3 = Vec3::new(533.88, 86.534424, 110.0);
const LANDING: Vec3 = Vec3::new(533.88, 56.281814, 110.0);

fn assets() -> PathBuf {
    PathBuf::from(
        std::env::var_os("FOLLOWER_COMPONENT_ASSETS").expect("native collision asset root"),
    )
}

fn navigation_root() -> PathBuf {
    PathBuf::from(std::env::var_os("FOLLOWER_NAV_CANONICAL").expect("native navigation root"))
}

fn position(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

#[test]
#[ignore = "requires actual2651 navigation/ground assets"]
fn darkflame_drop_native_navigation_connects_downward_but_never_upward() {
    let mut map =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    let path = map
        .complete_path(UPPER, LANDING)
        .expect("approved one-way drop connects native components");
    assert!(path.first().unwrap().distance(UPPER) < 0.4);
    assert!(path.last().unwrap().distance(LANDING) < 0.4);
    assert!(
        map.complete_path(LANDING, UPPER).is_err(),
        "down-only link must not create an upward route"
    );
}

#[test]
#[ignore = "requires actual2527/2526/2661 navigation assets"]
fn darkflame_drop_policy_preserves_the_three_unapproved_map_gaps() {
    for (map_id, from, to) in [
        (
            2527,
            Vec3::new(-0.370431, 20.8162, -228.269),
            Vec3::new(-353.12274, -67.9066, 155.7234),
        ),
        (
            2526,
            Vec3::new(1410.22, 955.637, 2771.71),
            Vec3::new(1815.2512, 904.01917, 2783.9756),
        ),
        (
            2661,
            Vec3::new(2650.88, 101.042, 4835.88),
            Vec3::new(2649.0535, 110.600006, 5013.1484),
        ),
    ] {
        let mut map =
            ::navigation::NavigationMap::open(&navigation_root().join(map_id.to_string())).unwrap();
        assert!(
            map.complete_path(from, to).is_err(),
            "map{map_id} has no approved drop"
        );
    }
}

#[test]
#[ignore = "requires actual2651 navigation/ground assets"]
fn darkflame_drop_native_move_fall_lands_continuously_on_target_polygon() {
    use crate::scripted_ai::fall_movement;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(GroundMap::new(&assets(), HashMap::from([(MAP, WDT)])));
    app.update();
    let unit = app
        .world_mut()
        .spawn((
            position(DEPARTURE),
            InMap::continent(MAP),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    fall_movement::move_fall(app.world_mut(), unit, 0);
    assert!(
        app.world()
            .get::<fall_movement::FallMovement>(unit)
            .is_some(),
        "native ground must resolve the actual lower floor"
    );
    let mut collision = los::LosMap::new(&assets().join(format!("los/{MAP}")));
    let mut previous = DEPARTURE;
    let mut samples = 0;
    for _ in 0..100 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        fall_movement::advance_falls(app.world_mut());
        let next = point(*app.world().get::<Position>(unit).unwrap());
        assert!((next.x - DEPARTURE.x).abs() < 0.001 && (next.z - DEPARTURE.z).abs() < 0.001);
        assert!(
            next.y <= previous.y && previous.y - next.y < 3.1,
            "continuous native gravity: {previous:?}->{next:?}"
        );
        assert!(
            collision.clear(previous + Vec3::Y, next + Vec3::Y).unwrap(),
            "fall crossed authored collision"
        );
        samples += usize::from(next.y < previous.y);
        previous = next;
    }
    assert!(
        samples > 20,
        "must fall over many native updates, not snap to landing"
    );
    assert!(
        (previous.y - LANDING.y).abs() < 0.05,
        "actual landing {previous:?}"
    );
    let mut ground = app.world_mut().resource_mut::<GroundMap>();
    assert!(
        matches!(ground.creature_ground_in(InMap::continent(MAP), previous), GroundProbe::Supported(y) if (y - LANDING.y).abs() < 0.05)
    );
    let mut map =
        ::navigation::NavigationMap::open(&navigation_root().join(MAP.to_string())).unwrap();
    assert!(
        map.complete_path(previous, LANDING)
            .unwrap()
            .last()
            .unwrap()
            .distance(LANDING)
            < 0.4
    );
    assert_eq!(
        app.world().get::<Health>(unit).unwrap().current,
        100.0,
        "FOLLOWER_NPC_FALL_DAMAGE_UNMODELED: do not add NPC damage"
    );
}

#[test]
#[ignore = "requires actual2651 navigation/ground and imported follower profile"]
fn darkflame_drop_follower_sim_traverses_only_after_upper_route_stage() {
    let (mut h, follower, _) = spawn_profile_fixture_with_plugins(209057, |app| {
        crate::creature_ai::install_follow_fixture(app);
        app.add_systems(
            Update,
            (
                crate::scripted_ai::movement::point_movement_system,
                crate::scripted_ai::fall_movement::advance_falls,
                crate::scripted_ai::fall_movement::release_finished_falls,
            )
                .chain(),
        );
    });
    h.app
        .insert_resource(GroundMap::new(&assets(), HashMap::from([(MAP, WDT)])));
    crate::navigation::install_fixture(
        &mut h.app,
        crate::navigation::Navigation::new(&navigation_root()),
    );
    h.app
        .world_mut()
        .entity_mut(follower)
        .insert((position(UPPER), InMap::continent(MAP)));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((position(UPPER), InMap::continent(MAP)));
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0 = None;
    h.advance(8);
    assert!(
        point(*h.app.world().get::<Position>(follower).unwrap()).distance(UPPER) < 0.1,
        "upper route stage must hold before leader proceeds"
    );
    // Actual lower-component boss placements: Blazikon order2000, Darkness order7000.
    let stages = [
        Vec3::new(406.7775, 56.68355, 155.27142),
        Vec3::new(188.47888, 59.8, 547.81714),
    ];
    let mut fall_samples = 0;
    for target in stages {
        *h.app.world_mut().get_mut::<Position>(h.caster).unwrap() = position(target);
        let mut arrived = false;
        for _ in 0..2400 {
            let before = point(*h.app.world().get::<Position>(follower).unwrap());
            h.advance(1);
            let after = point(*h.app.world().get::<Position>(follower).unwrap());
            assert!(
                after.distance(before) < 4.0,
                "follower teleported: {before:?}->{after:?}"
            );
            if before.y > 57.0 && before.y < 85.0 && after.y < before.y {
                fall_samples += 1;
            }
            if after.distance(target) < 15.0 {
                arrived = true;
                break;
            }
        }
        assert!(
            arrived,
            "follower failed authored lower stage {target:?}; feet {:?}",
            h.app.world().get::<Position>(follower)
        );
    }
    assert!(
        fall_samples > 15,
        "follower must traverse continuously rather than snap between floors"
    );
}
