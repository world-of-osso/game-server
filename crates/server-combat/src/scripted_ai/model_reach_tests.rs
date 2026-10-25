use super::*;
use crate::creature_aggro::AggroDistance;
use bevy::ecs::system::SystemState;

fn read_both_reaches(world: &mut World, unit: Entity) -> (Option<f32>, Option<f32>) {
    let scripted = combat_reach(world, unit);
    let mut state = SystemState::<AggroDistance>::new(world);
    let sight = state.get(world).unwrap().combat_reach(unit);
    (scripted, sight)
}

#[test]
fn missing_native_model_info_uses_scaled_player_default_reach() {
    let mut world = World::new();
    // Source Henry215011 display121275 has no encounter model-info row.
    world.insert_resource(ModelReaches(HashMap::new()));
    for (scale, expected) in [(0.5, 0.75), (1.0, 1.5), (2.0, 3.0)] {
        let unit = world
            .spawn((ModelDisplay { display_id: 121275 }, UnitScale(scale)))
            .id();
        assert_eq!(
            read_both_reaches(&mut world, unit),
            (Some(expected), Some(expected))
        );
    }
}

#[test]
fn native_model_geometry_preserves_source_radius_scale_and_unavailable_rows() {
    let mut world = World::new();
    // Native Glasshide Basilisk5419/display141, radius3.0042/reach2.
    let basilisk = world
        .spawn((ModelDisplay { display_id: 141 }, UnitScale(2.0)))
        .id();
    assert_eq!(bounding_radius(&world, basilisk), None);
    let conn = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    world.insert_resource(load_model_reaches(&conn).unwrap());
    assert_eq!(bounding_radius(&world, basilisk), Some(3.0042 * 2.0));
    assert_eq!(combat_reach(&world, basilisk), Some(4.0));
    let kobold = world
        .spawn((ModelDisplay { display_id: 10913 }, UnitScale(1.0)))
        .id();
    assert_eq!(bounding_radius(&world, kobold), Some(0.550618));
    assert_eq!(combat_reach(&world, kobold), Some(0.0));
    // Actual Henry215011 display has no source row: no radius substitution.
    let henry = world
        .spawn((ModelDisplay { display_id: 121275 }, UnitScale(1.0)))
        .id();
    assert_eq!(bounding_radius(&world, henry), None);
    let unidentified = world.spawn(UnitScale(1.0)).id();
    assert_eq!(bounding_radius(&world, unidentified), None);
}

#[test]
fn sourced_model_info_preserves_scaled_native_reach() {
    let mut world = World::new();
    // Source Crenna209072 display113757 has native combat reach3yd.
    world.insert_resource(ModelReaches(
        [(
            113757,
            ModelDimensions {
                combat_reach: 3.0,
                bounding_radius: 0.698,
            },
        )]
        .into(),
    ));
    for (scale, expected) in [(0.5, 1.5), (1.0, 3.0), (2.0, 6.0)] {
        let unit = world
            .spawn((ModelDisplay { display_id: 113757 }, UnitScale(scale)))
            .id();
        assert_eq!(
            read_both_reaches(&mut world, unit),
            (Some(expected), Some(expected))
        );
    }
}
