//! Actual Stockade population and baked route: cost intervening aggro, not only endpoints.
use super::pull_forecast_tests::source_fixture;
use super::pull_route::{self, Request};
use crate::maps::InMap;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Level, ModelDisplay, Position, UnitLevel};
use std::collections::BTreeSet;
use std::path::PathBuf;

const TARGET_GUID: u32 = 20375682;
const INTERVENING_GUID: u32 = 20375688;

fn route_fixture() -> (Harness, Entity) {
    let (mut h, tank, previous) = source_fixture(46249);
    h.app.world_mut().despawn(previous);
    let data = crate::test_data::world_data();
    h.app.insert_resource(data.clone());
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert((Level(32), UnitLevel(32)));
    let profile = h
        .app
        .world()
        .get::<super::Follower>(tank)
        .unwrap()
        .profile
        .clone();
    super::spawning::initialize(h.app.world_mut(), owner, tank, profile).unwrap();
    let model = &data.templates[&209057].models[0];
    let (x, y, z) = crate::terrain_height::wow_to_bevy(103.0, 76.0, -34.85623);
    h.app.world_mut().entity_mut(tank).insert((
        Position { x, y, z },
        ModelDisplay {
            display_id: model.display_id,
        },
        shared::components::UnitScale(model.display_scale),
    ));
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let mut ground = crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(34, 791060)]),
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    let count = crate::zones::spawn_map_creatures(
        &mut commands,
        data,
        crate::zones::MapContent {
            map,
            difficulty_id: 1,
            dead_bosses: vec![],
        },
        false,
        Some(&mut ground),
    );
    assert!(count > 2, "actual full Normal Stockade population");
    queue.apply(h.app.world_mut());
    h.app.insert_resource(crate::navigation::Navigation::new(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh"),
    ));
    (h, tank)
}

fn read_route(h: &mut Harness, tank: Entity) -> Result<pull_route::PullRoute, String> {
    pull_route::read_route(
        h.app.world_mut(),
        &Request {
            tank,
            target_guid: TARGET_GUID,
            difficulty: 1,
            cleared: &BTreeSet::new(),
        },
    )
}

#[test]
fn tank_pull_route_includes_intervening_native_aggro_on_a_complete_stockade_corridor() {
    let (mut h, tank) = route_fixture();
    let before = *h.app.world().get::<Position>(tank).unwrap();
    let owner_target = h
        .app
        .world()
        .get::<crate::networking::CombatTarget>(h.caster)
        .unwrap()
        .0;
    let route = read_route(&mut h, tank).unwrap();
    assert!(route.spawns.contains(&TARGET_GUID));
    assert!(
        route.spawns.contains(&INTERVENING_GUID),
        "20375688 is by the route origin, not the target endpoint"
    );
    assert!(
        route.pack_count > 1,
        "authored ungrouped Stockade spawns are separate packs"
    );
    let data = crate::test_data::world_data();
    let target = data
        .spawns
        .iter()
        .find(|source| source.guid == TARGET_GUID)
        .unwrap();
    let intervening = data
        .spawns
        .iter()
        .find(|source| source.guid == INTERVENING_GUID)
        .unwrap();
    let endpoint_gap =
        Vec3::new(target.position_x, target.position_y, target.position_z).distance(Vec3::new(
            intervening.position_x,
            intervening.position_y,
            intervening.position_z,
        ));
    assert!(
        endpoint_gap > 24.0,
        "concrete off-endpoint source distance {endpoint_gap}"
    );
    assert!(route.points.len() >= 2);
    let mut collision = los::LosMap::new(&crate::ground::ground_data_dir().join("los/34"));
    for leg in route.points.windows(2) {
        assert!(
            collision.clear(leg[0] + Vec3::Y, leg[1] + Vec3::Y).unwrap(),
            "budgeted route crosses collision: {leg:?}"
        );
    }
    assert_eq!(*h.app.world().get::<Position>(tank).unwrap(), before);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(h.caster)
            .unwrap()
            .0,
        owner_target
    );
}

#[test]
fn tank_pull_route_refuses_missing_bake_without_direct_movement() {
    let (mut h, tank) = route_fixture();
    let before = *h.app.world().get::<Position>(tank).unwrap();
    h.app
        .world_mut()
        .remove_resource::<crate::navigation::Navigation>();
    assert!(read_route(&mut h, tank).is_err());
    let empty = tempfile::tempdir().unwrap();
    h.app
        .insert_resource(crate::navigation::Navigation::new(empty.path()));
    assert!(read_route(&mut h, tank).is_err());
    assert_eq!(*h.app.world().get::<Position>(tank).unwrap(), before);
}

#[test]
fn tank_pull_route_refuses_unavailable_native_radius_and_disappeared_source() {
    let (mut h, tank) = route_fixture();
    assert!(
        read_route(&mut h, tank).is_ok(),
        "establish native full-map fixture before refusal controls"
    );
    let models = h
        .app
        .world_mut()
        .remove_resource::<crate::scripted_ai::model_reach::ModelReaches>()
        .unwrap();
    assert!(read_route(&mut h, tank).is_err());
    h.app.insert_resource(models);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let intervening = h
        .app
        .world_mut()
        .query::<(Entity, &crate::zones::CreatureOrigin, &InMap)>()
        .iter(h.app.world())
        .find(|(_, origin, copy)| origin.guid == INTERVENING_GUID && **copy == map)
        .unwrap()
        .0;
    h.app.world_mut().despawn(intervening);
    assert!(
        read_route(&mut h, tank).is_err(),
        "disappearance is not a free route or a guessed radius"
    );
}
