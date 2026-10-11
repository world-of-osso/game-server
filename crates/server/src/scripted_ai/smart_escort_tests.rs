//! Real TDB Erland quest-accept → timed-list → waypoint-start boundary.
use super::*;
use shared::components::{Npc, Position};
use shared::protocol::NpcFlags;

#[test]
fn smartai_coverage_erland_accept_starts_authored_path_and_removes_questgiver() {
    let source = crate::test_data::world_db_path();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("escort.sqlite");
    let fixture = Connection::open(&path).unwrap();
    fixture
        .execute("ATTACH DATABASE ?1 AS tdb", [source.to_str().unwrap()])
        .unwrap();
    fixture.execute_batch(
        "CREATE TABLE content_creature_template AS SELECT * FROM tdb.tdb_creature_template WHERE entry=1978;
         CREATE TABLE tdb_smart_scripts AS SELECT * FROM tdb.tdb_smart_scripts WHERE entryorguid=1978 OR entryorguid BETWEEN 197800 AND 197805;
         CREATE TABLE tdb_waypoint_path AS SELECT * FROM tdb.tdb_waypoint_path WHERE PathId=15825;
         CREATE TABLE tdb_waypoint_path_node AS SELECT * FROM tdb.tdb_waypoint_path_node WHERE PathId=15825;"
    ).unwrap();
    drop(fixture);
    let scripts = load_smart_scripts(&path).unwrap();
    let events = scripts.0[&1978].clone();
    let mut world = World::new();
    world.insert_resource(scripts);
    let creature = world
        .spawn((
            Npc {
                template_id: 1978,
                name: "Deathstalker Erland".into(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            NpcFlags(2),
        ))
        .id();
    let player = world.spawn_empty().id();
    let mut ai = SmartAi {
        states: vec![EventState::default(); events.len()],
        events,
        phase: 0,
        boss: None,
        stored_targets: HashMap::new(),
        timed_list: None,
        escort: None,
    };
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.reset(&mut ctx);
    ai.process_events_for(&mut ctx, 19, Some(player), 435);
    ai.update_ai(&mut ctx, 50);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0 & 2, 0);
    let movement = ctx
        .world
        .get::<super::super::movement::PointMovement>(creature)
        .expect("TDB action list 197800 must start waypoint path 15825");
    assert_eq!(movement.id, 1);
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let wow: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX, PositionY, PositionZ FROM tdb_waypoint_path_node WHERE PathId=15825 AND NodeId=1",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.0, wow.1, wow.2);
    assert_eq!(movement.target, Position { x, y, z });
}
