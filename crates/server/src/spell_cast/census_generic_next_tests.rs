//! TC a352b1fa SpellEffects.cpp1619; Unit.cpp3309,7503. Native70448 rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{CombatStatus, Health, Mana, PowerType};
use shared::protocol::CombatLogKind;

fn apply_native(h: &mut Harness, id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

#[test]
#[ignore = "RED census-next137: percent energize missing"]
fn census_next137_native_mana_uses_target_max_and_logs_overflow() {
    let mut h = harness(Vec::new(), &[], 8).unwrap();
    let spell = world_db().unwrap().store.get(5405).unwrap();
    assert_eq!(
        (spell.effects[0].effect, spell.effects[0].base_points),
        (137, 25.0)
    );
    let max = h.app.world().get::<Mana>(h.caster).unwrap().max;
    h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = max - 1.0;
    super::cast_triggered(h.app.world_mut(), h.caster, 5405, Some(h.caster), 1);
    assert_eq!(h.app.world().get::<Mana>(h.caster).unwrap().current, max);
    h.advance(1);
    let event = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(5405) && e.kind == CombatLogKind::Energize)
        .unwrap();
    let gain = (max * 0.25).trunc() as i32;
    assert_eq!((event.amount, event.overflow), (gain, gain - 1));
}

#[test]
#[ignore = "RED census-next137: percent energize missing"]
fn census_next137_native_energy_uses_raw_pool_and_dead_target_is_untouched() {
    let mut h = harness(Vec::new(), &[], 4).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(true));
    h.set_power(PowerType::Energy, 0.0);
    super::cast_triggered(h.app.world_mut(), h.caster, 55316, Some(h.caster), 1);
    let (_, max) = super::powers::power_value(h.app.world(), h.caster, PowerType::Energy).unwrap();
    assert_eq!(h.power(PowerType::Energy), max);
    h.set_power(PowerType::Energy, 0.0);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    super::cast_triggered(h.app.world_mut(), h.caster, 55316, Some(h.caster), 1);
    assert_eq!(h.power(PowerType::Energy), 0.0);
}

#[test]
#[ignore = "RED census-next307: label mobility missing"]
fn census_next307_native_label_allows_foreign_family_and_removal_interrupts() {
    let mut h = harness(Vec::new(), &[133], 8).unwrap();
    let aura = world_db().unwrap().store.get(343012).unwrap();
    assert_eq!(
        (aura.effects[0].aura, aura.effects[0].misc_values[0]),
        (307, 16)
    );
    assert!(
        world_db()
            .unwrap()
            .store
            .get(133)
            .unwrap()
            .labels
            .contains(&16)
    );
    h.cast_and_deliver(133, Some(h.npc));
    assert!(
        !h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    apply_native(&mut h, 343012, 1);
    assert!(
        h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    h.app
        .world_mut()
        .get_mut::<crate::networking::Moving>(h.caster)
        .unwrap()
        .0 = true;
    h.advance(2);
    assert!(h.app.world().get::<CastState>(h.caster).is_some());
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 343012,
    );
    assert!(
        !h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
    h.advance(1);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
}

#[test]
#[ignore = "RED census-next307: label mobility missing"]
fn census_next307_label_mismatch_does_not_grant_permission() {
    let mut h = harness(Vec::new(), &[133], 8).unwrap();
    // Native295263 selects661, unlike Fireball's native labels17/16/292.
    apply_native(&mut h, 295263, 1);
    assert!(crate::auras::take_unhandled_auras(h.app.world_mut()).is_empty());
    h.cast_and_deliver(133, Some(h.npc));
    assert!(
        !h.app
            .world()
            .get::<CastState>(h.caster)
            .unwrap()
            .cast_while_moving
    );
}

fn heal_fixture(h: &mut Harness, spell_id: u32, periodic: bool, health_pct: f32) -> f32 {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    let health = h
        .app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .into_inner();
    health.max = 10_000.0;
    health.current = 10_000.0 * health_pct;
    let before = health.current;
    // Fixed incoming heal fixture isolates the generic done multiplier shared by
    // direct heals and periodic ticks; native spell and aura selectors stay intact.
    super::effects::apply_heal(
        h.app.world_mut(),
        &super::effects::EffectCtx {
            caster: h.caster,
            spell,
            depth: 1,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.caster,
        100.0,
        false,
        periodic,
    );
    h.app.world().get::<Health>(h.caster).unwrap().current - before
}

#[test]
#[ignore = "RED census-next354: missing-health healing missing"]
fn census_next354_native_regenesis_scales_live_missing_health_and_removes() {
    let mut h = harness(Vec::new(), &[], 11).unwrap();
    apply_native(&mut h, 383191, 1);
    let baseline = heal_fixture(&mut h, 774, true, 0.5);
    assert_eq!(baseline, 105.0);
    assert_eq!(heal_fixture(&mut h, 774, false, 0.2), 108.0);
    assert_eq!(heal_fixture(&mut h, 774, true, 0.9), 101.0);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 383191,
    );
    assert_eq!(heal_fixture(&mut h, 774, true, 0.5), 100.0);
}

#[test]
#[ignore = "RED census-next354: missing-health healing missing"]
fn census_next354_native_class_mask_and_family_exclusions() {
    let mut h = harness(Vec::new(), &[], 11).unwrap();
    apply_native(&mut h, 383191, 1);
    assert!(crate::auras::take_unhandled_auras(h.app.world_mut()).is_empty());
    assert_eq!(
        heal_fixture(&mut h, 8936, true, 0.5),
        100.0,
        "Regrowth mask64 differs from Rejuvenation16"
    );
    assert_eq!(
        heal_fixture(&mut h, 8004, false, 0.5),
        100.0,
        "foreign Shaman family"
    );
}
