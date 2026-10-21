//! Concrete native max-level selection and Voltaic Blaze boundaries, not full APL parity.
use super::kit_tests::{at_level, uses_spell};
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, ThreatTable};
use shared::protocol::CombatLogKind;

fn prepare(entry: u32, count: usize) -> (Harness, Entity, Vec<Entity>) {
    prepare_at_level(entry, count, 90)
}

fn prepare_at_level(entry: u32, count: usize, level: u8) -> (Harness, Entity, Vec<Entity>) {
    let (mut h, caster) = at_level(entry, level, |_| {});
    let mut targets = vec![h.npc];
    targets.extend((1..count).map(|index| spawn_npc(&mut h.app, index as f32)));
    for &target in &targets {
        let mut threat = ThreatTable::default();
        threat.add_threat(h.caster.to_bits(), 1.0);
        h.app.world_mut().entity_mut(target).insert((
            Health {
                current: 1_000_000_000.0,
                max: 1_000_000_000.0,
            },
            threat,
        ));
    }
    (h, caster, targets)
}

#[test]
fn rotation_selection_63_native_book_reaches_fireball() {
    let (mut h, caster, _) = prepare(209059, 1);
    uses_spell(&mut h, caster, 133);
}

#[test]
fn rotation_selection_262_native_book_applies_flame_shock() {
    let (mut h, caster, _) = prepare(209053, 1);
    uses_spell(&mut h, caster, 470057);
    h.advance(20);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(h.npc)
            .is_some_and(|auras| auras
                .iter()
                .any(|aura| aura.spell_id == 188389 && aura.caster == caster))
    );
}

fn kybel_actions(level: u8) -> (Harness, Entity, Vec<u32>) {
    let (mut h, caster, _) = prepare_at_level(215240, 3, level);
    for _ in 0..600 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
    }
    let actions: Vec<_> = h
        .inbox
        .go
        .iter()
        .filter(|go| go.caster == caster.to_bits())
        .map(|go| go.spell_id)
        .collect();
    (h, caster, actions)
}

#[test]
fn rotation_selection_1467_three_targets_use_empower_and_spender() {
    let (h, caster, actions) = kybel_actions(90);
    assert!(crate::auras::has_aura(h.app.world(), caster, 411212));
    // Native411212 effects1077606/1077607 replace both empower roots.
    assert!(
        actions.contains(&382266) && actions.contains(&382411),
        "learned Font of Magic must cast native empowered variants: {actions:?}"
    );
    assert!(!actions.contains(&357208) && !actions.contains(&359073));
    assert!(
        actions.iter().any(|spell| matches!(spell, 356995 | 357211)),
        "Azure filler must not starve native Essence spenders: {actions:?}"
    );
}

#[test]
fn rotation_selection_1467_three_targets_before_font_use_native_roots() {
    let (h, caster, actions) = kybel_actions(56);
    assert!(!crate::auras::has_aura(h.app.world(), caster, 411212));
    assert!(
        actions.contains(&357208) && actions.contains(&359073),
        "before native Font of Magic57, both base empower roots must cast: {actions:?}"
    );
    assert!(!actions.contains(&382266) && !actions.contains(&382411));
}

#[test]
fn rotation_selection_262_voltaic_blaze_damage_precedes_delayed_flame_shock() {
    let (mut h, caster, targets) = prepare(209053, 3);
    let begin = aura_now(h.app.world());
    let first = h.inbox.log.len();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 470057, Some(h.npc), false)
        .unwrap();
    h.advance(1);
    let hits: Vec<_> = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1259101)
                && event.source == Some(caster.to_bits())
        })
        .cloned()
        .collect();
    assert!(
        !hits.is_empty(),
        "native Dummy root must cast its actual1259101 damage child"
    );
    assert!(hits.iter().all(|event| event.crit));
    h.advance(8);
    assert!(aura_now(h.app.world()) - begin < 0.5);
    assert!(
        targets
            .iter()
            .all(|target| !crate::auras::has_aura(h.app.world(), *target, 188389)),
        "TC's500ms Flame Shock callback must not execute early"
    );
    h.advance(2);
    for hit in hits {
        let target = Entity::try_from_bits(hit.target.unwrap()).unwrap();
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(target)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == 188389 && aura.caster == caster)),
            "actual Voltaic hit must receive its delayed native Flame Shock"
        );
    }
}
