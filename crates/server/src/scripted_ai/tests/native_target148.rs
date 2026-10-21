//! Native434712 destination/movement on real Orator actors and terrain, independent
//! of the Orator AI port. TC Spell.cpp1710–1720,9286–9291 (flags select collision).
use super::*;

fn native_pull(difficulty: u32) {
    let (mut h, boss, map) = encounter_fixture(
        216619,
        2669,
        1,
        difficulty,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    h.app.insert_resource(crate::test_data::ground_map());
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .expect("native movement collision inputs"),
    );
    let player = h.player();
    let target = *h.app.world().get::<Position>(boss).unwrap();
    let start = Position {
        x: target.x + 20.0,
        y: target.y + 20.0,
        z: target.z,
    };
    h.app.world_mut().entity_mut(player).insert(start);
    let mut expected = Vec3::new(target.x + 5.0, target.y, target.z);
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .ground_in(map, expected);
    let crate::ground::GroundProbe::Supported(height) = ground else {
        panic!("native five-yard destination floor prerequisite: {ground:?}");
    };
    expected.y = height;
    crate::spell_cast::creature_cast_ignoring_in_progress(
        h.app.world_mut(),
        player,
        434712,
        Some(boss),
    )
    .expect("ordinary native434712 cast admission");
    assert_eq!(
        *h.app.world().get::<Position>(player).unwrap(),
        start,
        "native254 must not teleport on launch"
    );
    h.advance(0.1);
    assert_ne!(
        *h.app.world().get::<Position>(player).unwrap(),
        start,
        "real35yd/s motion"
    );
    h.advance(1.2);
    let casts = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == player.to_bits() && go.spell_id == 434712)
        .count();
    assert_eq!(casts, 1);
    let actual = *h.app.world().get::<Position>(player).unwrap();
    let actual = Vec3::new(actual.x, actual.y, actual.z);
    assert!(
        actual.distance(expected) < 0.001,
        "native148 offset/floor endpoint: actual{actual:?}, expected{expected:?}"
    );
}

#[test]
#[ignore = "target148 ordered native destination RED"]
fn native_target148_normal_real_actor_physical_pull_endpoint() {
    native_pull(1);
}

#[test]
#[ignore = "target148 ordered native destination RED"]
fn native_target148_follower205_real_actor_physical_pull_endpoint() {
    native_pull(205);
}
