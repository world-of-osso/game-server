//! Native effect8 execution: unchanged imported rows, real costs and creature pools.
use bevy::prelude::*;
use shared::components::{Health, Mana, PowerType};

use super::test_harness::*;

const SOUL_REND: u32 = 3405;
const DRAIN_MANA: u32 = 46453;
const FROST_BREATH: u32 = 16094;
const ARCANE_INTELLECT: u32 = 1459;
const SKUL: u32 = 10393;
const NATURAL_PLAYER_MANA: f32 = 10_000.0;
const INTELLECT_COST: f32 = 100.0;
const NATIVE_DRAIN_WITH_PLAYER_SP: f32 = 117.0;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[SOUL_REND, DRAIN_MANA, FROST_BREATH, ARCANE_INTELLECT],
        8,
    )
    .unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    // Native Skul: class8, level58, HealthModifier6.5/ManaModifier2.
    let caster = h.caster;
    let target =
        crate::zones::summon_creature(h.app.world_mut(), caster, SKUL, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(mana(&h, caster), NATURAL_PLAYER_MANA);
    assert_eq!(mana(&h, target), 10_922.0);
    (h, target)
}

fn mana(h: &Harness, unit: Entity) -> f32 {
    h.app.world().get::<Mana>(unit).unwrap().current
}

/// The real completion boundary, without changing cast times or advancing regen.
fn execute(h: &mut Harness, spell_id: u32, target: Entity) {
    let spell = world_db().unwrap().store.get(spell_id).unwrap();
    super::execute_cast(h.app.world_mut(), h.caster, spell, Some(target)).unwrap();
}

fn spend_intellect_mana(h: &mut Harness, casts: usize) {
    let caster = h.caster;
    let before = mana(h, caster);
    for _ in 0..casts {
        execute(h, ARCANE_INTELLECT, caster);
    }
    assert_eq!(mana(h, caster), before - casts as f32 * INTELLECT_COST);
}

#[test]
fn native_power_drain_soul_rend_removes_ten_without_caster_credit() {
    let (mut h, target) = fixture();
    spend_intellect_mana(&mut h, 2);
    let target_before = mana(&h, target);
    let caster_before = mana(&h, h.caster);
    let health_before = h.health(target);
    execute(&mut h, SOUL_REND, target);
    assert_eq!(mana(&h, target), target_before - 10.0);
    assert_eq!(
        mana(&h, h.caster),
        caster_before,
        "native amplitude is zero"
    );
    assert_eq!(h.health(target), health_before);
}

#[test]
fn native_power_drain_mana_uses_integer_base_spell_power_and_credits_caster() {
    let (mut h, target) = fixture();
    spend_intellect_mana(&mut h, 2);
    let target_before = mana(&h, target);
    let caster_before = mana(&h, h.caster);
    execute(&mut h, DRAIN_MANA, target);
    // GetEffectValueAsInt truncates17.7514801, then native coefficient1 adds100SP.
    assert_eq!(
        mana(&h, target),
        target_before - NATIVE_DRAIN_WITH_PLAYER_SP
    );
    assert_eq!(
        mana(&h, h.caster),
        caster_before + NATIVE_DRAIN_WITH_PLAYER_SP
    );
}

#[test]
fn native_power_drain_floors_at_zero_and_credits_only_remaining_power() {
    let (mut h, target) = fixture();
    for _ in 0..93 {
        execute(&mut h, DRAIN_MANA, target);
    }
    assert_eq!(mana(&h, target), 41.0);
    spend_intellect_mana(&mut h, 2);
    let caster_before = mana(&h, h.caster);
    execute(&mut h, DRAIN_MANA, target);
    assert_eq!(mana(&h, target), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + 41.0);
    execute(&mut h, DRAIN_MANA, target);
    assert_eq!(mana(&h, target), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + 41.0);
}

#[test]
fn native_power_drain_frost_breath_retains_health_damage_and_native_variance() {
    let (mut h, target) = fixture();
    spend_intellect_mana(&mut h, 30);
    let mana_before = mana(&h, target);
    let caster_before = mana(&h, h.caster);
    let health_before = h.health(target);
    execute(&mut h, FROST_BREATH, target);
    assert!(h.health(target) > 0.0);
    assert!(
        h.health(target) < health_before,
        "native effect0 still deals damage"
    );
    let drained = mana_before - mana(&h, target);
    // Native1666 +100SP; retain authored variance0.300120056 (no RNG override).
    assert!((1501.0..=2030.0).contains(&drained), "drained={drained}");
    assert_eq!(drained.fract(), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + drained);
}

#[test]
fn native_power_drain_does_not_drain_nonprimary_legacy_mana_on_a_rogue() {
    let mut h = harness(Vec::new(), &[DRAIN_MANA], 4).unwrap();
    let caster = h.caster;
    let enemy = h.npc;
    let pools = h
        .app
        .world()
        .get::<super::powers::PowerPools>(caster)
        .unwrap();
    assert_eq!(pools.order.first(), Some(&PowerType::Energy));
    let energy_before = h.power(PowerType::Energy);
    assert_eq!(mana(&h, caster), NATURAL_PLAYER_MANA);
    super::creature::creature_cast(h.app.world_mut(), enemy, DRAIN_MANA, Some(caster), true)
        .unwrap();
    assert_eq!(mana(&h, caster), NATURAL_PLAYER_MANA);
    assert_eq!(h.power(PowerType::Energy), energy_before);
}

#[test]
fn native_power_drain_target_without_mana_does_not_credit_or_damage() {
    let mut h = harness(Vec::new(), &[SOUL_REND, ARCANE_INTELLECT], 8).unwrap();
    spend_intellect_mana(&mut h, 2);
    let target = h.npc;
    assert!(h.app.world().get::<Mana>(target).is_none());
    let caster_before = mana(&h, h.caster);
    let health_before = h.app.world().get::<Health>(target).unwrap().current;
    execute(&mut h, SOUL_REND, target);
    assert_eq!(mana(&h, h.caster), caster_before);
    assert_eq!(h.health(target), health_before);
}
