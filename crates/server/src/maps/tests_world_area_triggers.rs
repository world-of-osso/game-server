//! Ordinary MapTransfer/NewWorld/WorldPortAck with actual Orator barrier data.
//! The transfer destination is the authored barrier pose, not an asserted portal
//! entrance or a fabricated placement. No Orator AI/death hook in this slice.
use super::*;
use crate::world_area_triggers::WorldAreaTrigger;

const MAP_ID: u32 = 2669;
const BARRIER_SPAWN: u64 = 98;

fn source_barrier_location() -> WorldLocation {
    let connection = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    connection.query_row(
        "SELECT MapId,PosX,PosY,PosZ,Orientation FROM tdb_areatrigger WHERE SpawnId=98 AND AreaTriggerCreatePropertiesId=84 AND IsCustom=1",
        [], |row| Ok(WorldLocation { map_id: row.get(0)?, position: Vec3::new(row.get(1)?,row.get(2)?,row.get(3)?), orientation: row.get(4)? }),
    ).expect("authentic Orator bridge barrier spawn")
}

fn enter(h: &mut Harness, player: &Traveler, destination: WorldLocation) -> InMap {
    assert!(teleport_to(
        h.app.world_mut(),
        player.player,
        destination,
        TeleportOptions::default()
    ));
    let messages = h.new_worlds(player);
    assert_eq!(messages.len(), 1, "ordinary destination NewWorld");
    assert_eq!(messages[0].map_id, destination.map_id);
    h.ack(player);
    h.map_of(player).expect("actual ACK attaches the entrant")
}

fn barrier(world: &mut World, copy: InMap) -> Entity {
    let entities: Vec<_> = world
        .query::<(Entity, &InMap, &WorldAreaTrigger)>()
        .iter(world)
        .filter(|(_, map, area)| **map == copy && area.spawn_id == BARRIER_SPAWN)
        .map(|(entity, _, _)| entity)
        .collect();
    assert_eq!(
        entities.len(),
        1,
        "ordinary map entry must spawn actual98/84/35803 in {copy:?}"
    );
    entities[0]
}

fn exercise_barrier(difficulty: u32) {
    let mut h = harness().expect("real map/admission catalog");
    let source = source_barrier_location();
    assert_eq!(source.map_id, MAP_ID);
    let first = h.spawn(9101, STORMWIND, STOCKADE_EXIT_TARGET);
    let second = h.spawn(9102, STORMWIND, STOCKADE_EXIT_TARGET);
    for traveler in [&first, &second] {
        h.app
            .world_mut()
            .entity_mut(traveler.player)
            .insert(DungeonDifficulty(difficulty));
    }
    let a = enter(&mut h, &first, source);
    let b = enter(&mut h, &second, source);
    assert_ne!(a, b, "distinct ungrouped copies");
    let first_barrier = barrier(h.app.world_mut(), a);
    let second_barrier = barrier(h.app.world_mut(), b);
    assert_ne!(first_barrier, second_barrier);
    let area = h
        .app
        .world()
        .get::<WorldAreaTrigger>(first_barrier)
        .unwrap();
    assert_eq!(
        (area.create_properties_id, area.create_properties_custom),
        (84, true)
    );
    assert_eq!((area.template_id, area.template_custom), (35803, false));
    assert_eq!(
        (area.shape, area.shape_data),
        (0, [20.0, 20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    );
    assert_eq!(area.visual_spell, 449122);
    assert!(close(
        wow(h.app.world().get::<Position>(first_barrier).unwrap()),
        source.position
    ));
    assert!(
        (h.app.world().get::<Rotation>(first_barrier).unwrap().y
            - source.orientation
            - std::f32::consts::FRAC_PI_2)
            .abs()
            < 0.00001
    );
    // Leaving one copy exercises ordinary manager expiry, not direct despawn.
    let outside = WorldLocation {
        map_id: 0,
        position: STOCKADE_EXIT_TARGET,
        orientation: 0.0,
    };
    enter(&mut h, &first, outside);
    let delay = Duration::from_secs_f64(INSTANCE_UNLOAD_DELAY_SECS + 1.0);
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(delay);
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delay));
    h.advance(2);
    assert!(
        h.app.world().get_entity(first_barrier).is_err(),
        "empty map copy unload removes its barrier"
    );
    assert_eq!(
        barrier(h.app.world_mut(), b),
        second_barrier,
        "occupied copy retains its indefinite static barrier"
    );
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_millis(50),
        ));
    let fresh = enter(&mut h, &first, source);
    assert_ne!(fresh, a);
    assert_ne!(barrier(h.app.world_mut(), fresh), first_barrier);
    assert_eq!(barrier(h.app.world_mut(), b), second_barrier);
}

#[test]
#[ignore = "RED: world-static AreaTrigger spawns have no runtime map-entry path"]
fn world_static_areatrigger_orator_barrier_normal_entry_isolation_and_teardown() {
    exercise_barrier(1);
}
#[test]
#[ignore = "RED: world-static AreaTrigger spawns have no runtime map-entry path"]
fn world_static_areatrigger_orator_barrier_follower205_entry_isolation_and_teardown() {
    exercise_barrier(205);
}
