//! TC a352b1fa Spell.cpp1089–1214: nearest entry destination.
use super::test_harness::{harness, spawn_npc, world_db};
use super::*;
use shared::components::Position;

#[test]
fn target46_uses_nearest_matching_object_in_the_same_copy() {
    let mut h = harness(Vec::new(), &[381517], 1).unwrap();
    let mut spell = world_db().unwrap().store.get(381517).unwrap().clone();
    spell.misc.as_mut().unwrap().attributes[1] |= 0x0008_0000;
    let map = *h.app.world().get::<crate::maps::InMap>(h.caster).unwrap();
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let near = spawn_npc(&mut h.app, 0.0);
    let far = spawn_npc(&mut h.app, 0.0);
    let remote = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(near).insert((
        Position {
            x: origin.x + 2.0,
            ..origin
        },
        map,
    ));
    h.app.world_mut().entity_mut(far).insert((
        Position {
            x: origin.x + 4.0,
            ..origin
        },
        map,
    ));
    h.app.world_mut().entity_mut(remote).insert((
        Position {
            x: origin.x + 1.0,
            ..origin
        },
        crate::maps::InMap {
            instance_id: map.instance_id + 1,
            ..map
        },
    ));
    // The fixture's original target must not be closer than the concrete candidates.
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: origin.x + 8.0,
        ..origin
    });
    let dest =
        targets::effect_destination(h.app.world_mut(), h.caster, None, &spell, &spell.effects[0])
            .unwrap()
            .unwrap();
    assert!((dest.position.x - (origin.x + 2.0)).abs() < 0.001);
}
