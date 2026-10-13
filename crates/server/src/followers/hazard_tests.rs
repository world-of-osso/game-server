//! Actual hostile Consecration and Stockade routes, production fixed-order ticks.
use super::navigation_tests::stockade_fixture;
use super::utility_tests::prepare_enemy_spec;
use crate::maps::InMap;
use crate::spell_cast::test_harness::spawn_npc;
use bevy::prelude::*;
use shared::components::Position;

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

#[test]
fn follower_leaves_hostile_consecration_on_actual_stockade_navigation() {
    let from = Position {
        x: 116.0,
        y: -34.85623,
        z: -76.0,
    };
    let (mut h, unit, engine) = stockade_fixture(from);
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
    prepare_enemy_spec(&mut h, enemy, 66);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 26573, Some(enemy), false).unwrap();
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
}
