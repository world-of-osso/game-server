//! Actual70448 mixed target lists on real City of Threads ground. These tests
//! qualify target selection, not unported arrival scripts of the sampled spells.
use super::test_harness::{Harness, harness, spawn_npc, world_db};
use super::*;
use crate::maps::InMap;
use shared::components::{Position, Rotation};

const MAP: InMap = InMap {
    map_id: 2669,
    instance_id: 1,
};
const IDS: &[u32] = &[330398, 196884, 1243638, 1281788, 1242814, 448480];

fn scene() -> (Harness, Position) {
    let mut h = harness(Vec::new(), IDS, 1).expect("native spell fixture");
    let origin = place_on_native_ground(&mut h);
    (h, origin)
}

pub(super) fn place_on_native_ground(h: &mut Harness) -> Position {
    let connection = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let origin = connection.query_row(
        "SELECT position_x,position_y,position_z FROM content_creature WHERE map=2669 AND id1=216619 ORDER BY guid LIMIT 1",
        [], |row| { let (x,y,z) = crate::terrain_height::wow_to_bevy(row.get(0)?,row.get(1)?,row.get(2)?); Ok(Position { x,y,z }) },
    ).expect("authored Orator arena input");
    h.app.insert_resource(crate::test_data::ground_map());
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .unwrap(),
    );
    h.app.world_mut().entity_mut(h.npc).insert((origin, MAP));
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: origin.x + 20.0,
            y: origin.y + 20.0,
            z: origin.z,
        },
        MAP,
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    origin
}

fn assert_endpoint(h: &mut Harness, actual: Dest, origin: Position, offset: f32) {
    let expected_x = origin.x + offset;
    let probe = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(MAP, Vec3::new(expected_x, origin.y, origin.z));
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("real native destination floor prerequisite: {probe:?}");
    };
    assert!(
        (actual.position.x - expected_x).abs() < 0.001,
        "native ordered X: {actual:?}, expected{expected_x}"
    );
    assert!((actual.position.z - origin.z).abs() < 0.001);
    assert!(
        (actual.position.y - height).abs() < 0.001,
        "native real floor: {actual:?}, expected{height}"
    );
}

fn destination(h: &mut Harness, spell_id: u32, effect: usize) -> Dest {
    let spell = world_db().unwrap().store.get(spell_id).unwrap().clone();
    plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false)
        .expect("native mixed selector plan")
        .into_iter()
        .find(|entry| entry.index == effect)
        .and_then(|entry| entry.dest)
        .expect("actual native selected destination")
}

#[test]
fn native_target148_target_enemy_then_towards_caster() {
    let (mut h, origin) = scene();
    // Face perpendicular to the destination/caster line:148 follows that line, not facing.
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::PI,
        z: 0.0,
    });
    let actual = destination(&mut h, 196884, 1); // Feral Lunge [53,148], native radius2.
    assert_endpoint(&mut h, actual, origin, 2.0);
}

#[test]
fn native_target148_then_ground_preserves_shifted_destination() {
    let (mut h, origin) = scene();
    let actual = destination(&mut h, 1243638, 0); // Harpoon [148,138], native radius3.
    assert_endpoint(&mut h, actual, origin, 3.0);
}

#[test]
fn native_target148_then_existing_destination_does_not_reinitialize() {
    let (mut h, origin) = scene();
    let actual = destination(&mut h, 1281788, 0); // Fast Jump [148,87], native radius3.
    assert_endpoint(&mut h, actual, origin, 3.0);
}

#[test]
fn native_target148_then_back_uses_caster_facing_and_source_order() {
    let (mut h, origin) = scene();
    let actual = destination(&mut h, 330398, 0); // Leap [148,79]: +5 toward caster then -2.5 relative to caster facing0.
    assert_endpoint(&mut h, actual, origin, 2.5);
    assert!(
        actual.orientation.abs() < 0.001,
        "source default MovePosition restores caster facing"
    );
}

#[test]
fn native_target148_area_enemy_selection_uses_modified_center() {
    let (mut h, origin) = scene();
    let near = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(near).insert((
        Position {
            x: origin.x + 22.0,
            ..origin
        },
        MAP,
    ));
    let spell = world_db().unwrap().store.get(1242814).unwrap().clone(); // Void Breath [148,16]: shift20, radius10.
    let units = targets::select_targets(
        h.app.world_mut(),
        h.caster,
        Some(h.npc),
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .expect("native destination-area target selection");
    assert_eq!(
        units,
        vec![near],
        "center must shift20 toward the caster; original target is outside10"
    );
}

#[test]
fn native_target148_force_location_has_named_geometry_gap() {
    let (mut h, _) = scene();
    let spell = world_db().unwrap().store.get(448480).unwrap().clone(); // Actual [138,148], force-location attribute.
    let result = plan_targets(h.app.world_mut(), h.caster, &spell, Some(h.npc), false);
    assert!(
        matches!(result, Err(CastFailure::Unsupported(ref reason))
        if reason.contains("SPELL_DEST_FORCE_LOCATION_MAX_HEIGHT_ABSENT")),
        "native force-location must not silently use direct-ray geometry: {result:?}"
    );
}
