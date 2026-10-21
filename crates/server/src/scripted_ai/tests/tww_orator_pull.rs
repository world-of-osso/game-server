//! TC a352b1fa Spell.cpp1710–1720 / Object.cpp2787–2832, native434712.
//! Scripted ordinary player cast with only IGNORE_CAST_IN_PROGRESS, no triggered
//! full mask, fake destination, movement component or ground plane.
use super::tww_orator::{cast_count, fixture};
use super::*;

const PULL: u32 = 434712;
const SOURCE_OFFSET: f32 = 5.0;
const SOURCE_MAX_HEIGHT_CHANGE: f32 = 6.0;

fn native_pull(difficulty: u32) {
    let (mut h, boss, map) = fixture(difficulty);
    h.app.insert_resource(crate::test_data::ground_map());
    let player = h.player();
    let target = *h.app.world().get::<Position>(boss).unwrap();
    let start = Position {
        x: target.x + 20.0,
        y: target.y + 20.0,
        z: target.z,
    };
    h.app.world_mut().entity_mut(player).insert(start);
    // Actual native148 destination is five horizontal yards from the supplied
    // boss destination toward the player, then real map height, not its center.
    let mut expected = Vec3::new(target.x + SOURCE_OFFSET, target.y, target.z);
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, expected);
    let crate::ground::GroundProbe::Supported(height) = ground else {
        panic!("native Orator five-yard destination floor prerequisite: {ground:?}");
    };
    assert!(
        (height - target.y).abs() <= SOURCE_MAX_HEIGHT_CHANGE,
        "source full five-yard MovePosition is admissible on this actual floor"
    );
    expected.y = height;
    crate::spell_cast::creature_cast_ignoring_in_progress(
        h.app.world_mut(),
        player,
        PULL,
        Some(boss),
    )
    .expect("native ordinary434712 cast admission");
    assert_eq!(
        *h.app.world().get::<Position>(player).unwrap(),
        start,
        "native254 does not teleport on launch"
    );
    h.advance(0.1);
    let partial = *h.app.world().get::<Position>(player).unwrap();
    assert_ne!(partial, start, "native35yd/s jump advances physically");
    h.advance(1.2);
    assert_eq!(cast_count(&h, player, PULL), 1);
    let actual = *h.app.world().get::<Position>(player).unwrap();
    let actual = Vec3::new(actual.x, actual.y, actual.z);
    assert!(
        actual.distance(expected) < 0.001,
        "native148 offset+real-floor endpoint: actual{actual:?}, expected{expected:?}, boss{target:?}"
    );
}

#[test]
#[ignore = "RED first: native148 currently uses an unmodified destination"]
fn encounter_coverage_tww_orator_normal_native148_pull_offset_and_floor() {
    native_pull(1);
}

#[test]
#[ignore = "RED first: native148 currently uses an unmodified destination"]
fn encounter_coverage_tww_orator_follower205_native148_pull_offset_and_floor() {
    native_pull(205);
}
