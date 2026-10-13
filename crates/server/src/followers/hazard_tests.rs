//! Actual hostile Consecration and Stockade routes, production fixed-order ticks.
use super::navigation_tests::stockade_fixture;
use super::utility_tests::prepare_enemy_spec;
use crate::maps::InMap;
use crate::spell_cast::test_harness::spawn_npc;
use bevy::prelude::*;
use shared::components::Position;

#[test]
fn follower_hostile_field_without_a_bake_never_becomes_direct_escape() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    hostile_field(&mut h, unit);
    let missing = tempfile::tempdir().unwrap();
    h.app
        .insert_resource(crate::navigation::Navigation::new(missing.path()));
    for _ in 0..60 {
        h.advance(1);
    }
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), FROM);
}

#[test]
fn follower_does_not_flee_friendly_consecration() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    h.app.world_mut().entity_mut(h.caster).insert(FROM);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 26573, Some(unit), false).unwrap();
    for _ in 0..60 {
        h.advance(1);
    }
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), FROM);
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

const FROM: Position = Position {
    x: 116.0,
    y: -34.85623,
    z: -76.0,
};

fn hostile_field(h: &mut crate::spell_cast::test_harness::Harness, unit: Entity) {
    let from = FROM;
    let enemy = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((from, InMap::continent(34)));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((from, crate::networking::CombatTarget(Some(enemy))));
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(crate::networking::CombatTarget(Some(unit)));
    prepare_enemy_spec(h, enemy, 66);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 26573, Some(enemy), false).unwrap();
}

#[test]
fn follower_leaves_hostile_consecration_on_actual_stockade_navigation() {
    let from = FROM;
    let (mut h, unit, engine) = stockade_fixture(from);
    hostile_field(&mut h, unit);
    let mut los = los::LosMap::new(&engine.join("los/34"));
    let mut previous = from;
    let mut escaped = false;
    for tick in 0..200 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            los.clear(point(previous) + Vec3::Y, point(at) + Vec3::Y)
                .unwrap(),
            "flee tick{tick} crossed real collision"
        );
        if point(at).distance(point(from)) > 8.0 {
            escaped = true;
            break;
        }
        previous = at;
    }
    assert!(
        escaped,
        "companion must escape actual hostile Consecration before expiry"
    );
    for _ in 0..40 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            point(at).distance(point(from)) > 8.0,
            "combat must not pull the companion back into the active field"
        );
    }
}
