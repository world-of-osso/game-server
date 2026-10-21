//! FOLLOWER_TANK_DEFENSIVE_SEQUENCING: explicit project timing, native spells.
use super::kit::{Conditions, Entry, Slot, Target};
use super::kit_tests::at_level;
use super::{FollowerCatalog, kit_ai, update_ai};
use crate::auras::has_aura;
use crate::spell_cast::{creature_cast, test_harness::Harness};
use bevy::prelude::*;
use shared::components::Health;

fn wounded_garrick() -> (Harness, Entity) {
    let (mut h, unit) = at_level(209057, 50, |_| {});
    let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
    health.current = health.max * 0.45;
    h.app.world_mut().entity_mut(unit).insert((
        crate::combat::AutoAttack::new(h.npc),
        crate::networking::CombatTarget(Some(h.npc)),
    ));
    (h, unit)
}

fn major(spell_id: u32) -> Entry {
    Entry {
        spec_id: 66,
        spell_id,
        min_level: 1,
        max_level: 90,
        slot: Slot::Defensive,
        conditions: Conditions {
            target: Target::SelfUnit,
            health_below: Some(0.5),
            missing_aura: Some(spell_id),
            tank_defensive_sequence: true,
            ..Default::default()
        },
    }
}

#[test]
fn rotation_fidelity_tank_major_never_overlaps_another_native_major() {
    let (mut h, unit) = wounded_garrick();
    let guardian = major(86659);
    assert!(kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        unit,
        &[],
        &[],
        &guardian
    ));
    creature_cast(h.app.world_mut(), unit, 31850, Some(unit), false).unwrap();
    h.advance(3);
    assert!(
        has_aura(h.app.world(), unit, 31850),
        "actual Ardent Defender application"
    );
    assert!(
        !kit_ai::conditions_met(h.app.world_mut(), unit, unit, &[], &[], &guardian),
        "named sequencing policy must preserve Guardian while Ardent Defender is active"
    );
    h.advance(250);
    assert!(!has_aura(h.app.world(), unit, 31850));
    assert!(kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        unit,
        &[],
        &[],
        &guardian
    ));
}

#[test]
fn rotation_fidelity_tank_prefers_shorter_native_cooldown_not_catalog_order() {
    let (mut h, unit) = wounded_garrick();
    h.app.world_mut().resource_mut::<FollowerCatalog>().kits = vec![major(86659), major(31850)];
    update_ai(h.app.world_mut());
    h.advance(3);
    let first = h
        .inbox
        .go
        .iter()
        .find(|event| event.caster == unit.to_bits() && [31850, 86659].contains(&event.spell_id))
        .expect("a real native major must be selected");
    assert_eq!(
        first.spell_id, 31850,
        "Ardent Defender's native cooldown is shorter than Guardian's even in reversed input order"
    );
}

#[test]
fn rotation_fidelity_last_stand_is_only_the_native_learned_shield_wall_consequence() {
    for (level, learned) in [(46, false), (47, true)] {
        let (mut h, unit) = at_level(215517, level, |_| {});
        let before = h.app.world().get::<Health>(unit).unwrap().max;
        creature_cast(h.app.world_mut(), unit, 871, Some(unit), false).unwrap();
        h.advance(3);
        assert!(has_aura(h.app.world(), unit, 871), "real Shield Wall");
        assert_eq!(
            has_aura(h.app.world(), unit, 12975),
            learned,
            "native1243659 first learned at47, no invented manual Last Stand"
        );
        let after = h.app.world().get::<Health>(unit).unwrap().max;
        assert_eq!(
            after > before,
            learned,
            "actual Last Stand max-health consequence"
        );
    }
}
