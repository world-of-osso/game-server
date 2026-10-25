//! TC a352b1fa Spell.cpp9462–9483 / Unit.cpp712–720: actual native cone delivery.
use super::test_harness::{Harness, harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{ModelDisplay, Position, UnitScale};
use shared::protocol::CombatLogKind;

fn fixture(spell: u32, class: u8) -> Harness {
    let mut h = harness(Vec::new(), &[spell], class).unwrap();
    let front = h.npc;
    set_position(&mut h, front, Vec3::new(4.0, 0.0, 0.0));
    h
}

fn set_position(h: &mut Harness, unit: Entity, position: Vec3) {
    // Native Kobold6 identity/scale, not a manufactured bounding radius.
    let model = &crate::test_data::world_data().templates[&6].models[0];
    h.app.world_mut().entity_mut(unit).insert((
        Position {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        ModelDisplay {
            display_id: model.display_id,
        },
        UnitScale(model.display_scale),
    ));
}

fn target(h: &mut Harness, position: Vec3) -> Entity {
    let unit = spawn_npc(&mut h.app, position.x);
    set_position(h, unit, position);
    unit
}

fn attempt(h: &Harness, spell: u32, unit: Entity) -> bool {
    h.inbox.log.iter().any(|event| {
        event.source == Some(h.caster.to_bits())
            && event.target == Some(unit.to_bits())
            && event.spell_id == Some(spell)
            && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
    })
}

fn cast(h: &mut Harness, spell: u32) {
    // Ordinary authenticated spell intent; no FULL_MASK, stats, resource, or gate override.
    h.cast(spell, Some(h.npc));
    h.advance(6);
    assert!(
        h.inbox.failed.is_empty(),
        "native cast failed: {:?}",
        h.inbox.failed
    );
    assert!(
        attempt(h, spell, h.npc),
        "native front target must receive a real hit attempt"
    );
}

#[test]
fn cone_close_boundary_native_fof_hits_close_off_arc_enemy() {
    let spell = 117418;
    let mut h = fixture(spell, 10);
    // WoW(1.5,1,0): exact1.8027756yd/33.690°; outside the native60° cone.
    let close = target(&mut h, Vec3::new(1.5, 0.0, -1.0));
    cast(&mut h, spell);
    assert!(
        attempt(&h, spell, close),
        "native117418 must attempt the1.803yd/33.69° target inside TC's2yd boundary"
    );
}

#[test]
fn cone_close_boundary_native_fof_rejects_exact_radius_far_and_vertical_off_arc() {
    let spell = 117418;
    let mut h = fixture(spell, 10);
    let exact = target(&mut h, Vec3::new(0.0, 0.0, -2.0));
    let far = target(&mut h, Vec3::new(3.0, 0.0, -3.0));
    let vertical = target(&mut h, Vec3::new(1.5, 2.0, -1.0));
    cast(&mut h, spell);
    for unit in [exact, far, vertical] {
        assert!(
            !attempt(&h, spell, unit),
            "off-arc target outside the strict3D boundary must remain excluded: {unit:?}"
        );
    }
}

#[test]
fn cone_close_boundary_native_cone_of_cold_hits_close_rear_enemy() {
    let spell = 120;
    let mut h = fixture(spell, 8);
    let close = target(&mut h, Vec3::new(-1.5, 0.0, -1.0));
    let far = target(&mut h, Vec3::new(-3.0, 0.0, -3.0));
    cast(&mut h, spell);
    assert!(
        attempt(&h, spell, close),
        "native Mage120 must attempt a close rear target regardless of arc"
    );
    assert!(
        !attempt(&h, spell, far),
        "native Mage120 must not hit a distant rear target"
    );
}
