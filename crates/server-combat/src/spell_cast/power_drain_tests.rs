//! Native effect8 execution: unchanged imported rows, real costs and creature pools.
use bevy::prelude::*;
use shared::components::{Health, Level, Mana, PowerType, UnitLevel};

use super::test_harness::*;

const SOUL_REND: u32 = 3405;
const DRAIN_MANA: u32 = 46453;
const FROST_BREATH: u32 = 16094;
const ARCANE_INTELLECT: u32 = 1459;
const SKUL: u32 = 10393;
const NATURAL_PLAYER_MANA: f32 = 10_000.0;
const INTELLECT_COST: f32 = 100.0;
const NATIVE_DRAIN_WITH_PLAYER_SP: f32 = 278.0;

pub(super) fn fixture(class: u8) -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[SOUL_REND, DRAIN_MANA, FROST_BREATH, ARCANE_INTELLECT],
        class,
    )
    .unwrap();
    // Match real player-entry UnitLevel to the fixture's existing Level10.
    let level = h.app.world().get::<Level>(h.caster).unwrap().0;
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(UnitLevel(level));
    let source = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_creature_spell_stats(&source));
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
fn native_power_drain_soul_rend_uses_caster_level_expected_mana_without_credit() {
    let (mut h, target) = fixture(8);
    spend_intellect_mana(&mut h, 2);
    let target_before = mana(&h, target);
    let caster_before = mana(&h, h.caster);
    let health_before = h.health(target);
    execute(&mut h, SOUL_REND, target);
    assert_eq!(mana(&h, target), target_before - 100.0);
    assert_eq!(
        mana(&h, h.caster),
        caster_before,
        "native amplitude is zero"
    );
    assert_eq!(h.health(target), health_before);
}

#[test]
fn native_power_drain_mana_uses_expected_mana_rounding_spell_power_and_credits_caster() {
    let (mut h, target) = fixture(8);
    spend_intellect_mana(&mut h, 3);
    let target_before = mana(&h, target);
    let caster_before = mana(&h, h.caster);
    execute(&mut h, DRAIN_MANA, target);
    // Native Level10 PlayerMana1000 ×17.7514801% rounds178, then100SP.
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
    let (mut h, target) = fixture(8);
    for _ in 0..39 {
        execute(&mut h, DRAIN_MANA, target);
    }
    assert_eq!(mana(&h, target), 80.0);
    spend_intellect_mana(&mut h, 2);
    let caster_before = mana(&h, h.caster);
    execute(&mut h, DRAIN_MANA, target);
    assert_eq!(mana(&h, target), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + 80.0);
    execute(&mut h, DRAIN_MANA, target);
    assert_eq!(mana(&h, target), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + 80.0);
}

#[test]
fn native_power_drain_frost_breath_retains_health_damage_and_native_variance() {
    let (mut h, target) = fixture(8);
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
    // Native ScalesWithCreatureLevel chooses Level10 ExpectedStat DPS17.0521317:
    // base rounds284, then correlated native variance and100SP give290..491.
    assert!((290.0..=491.0).contains(&drained), "drained={drained}");
    assert_eq!(drained.fract(), 0.0);
    assert_eq!(mana(&h, h.caster), caster_before + drained);
}

#[test]
fn native_power_drain_does_not_drain_nonprimary_legacy_mana_on_a_rogue() {
    let (mut h, enemy) = fixture(4);
    let caster = h.caster;
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
fn native_power_drain_refuses_missing_caster_unit_level_instead_of_guessing_one() {
    let mut h = harness(Vec::new(), &[SOUL_REND], 8).unwrap();
    assert!(h.app.world().get::<UnitLevel>(h.caster).is_none());
    let spell = world_db().unwrap().store.get(SOUL_REND).unwrap();
    let result = super::execute_cast(h.app.world_mut(), h.caster, spell, Some(h.npc));
    assert!(matches!(
        result,
        Err(super::CastFailure::Unsupported(detail)) if detail.contains("UnitLevel")
    ));
    assert_eq!(mana(&h, h.caster), NATURAL_PLAYER_MANA);
    assert_eq!(h.health(h.npc), 5000.0);
}

#[test]
fn native_power_drain_refuses_missing_expected_stat_resource_instead_of_using_raw_base() {
    let (mut h, target) = fixture(8);
    h.app
        .world_mut()
        .remove_resource::<crate::level_scaling::CreatureSpellStats>();
    let before = mana(&h, target);
    let spell = world_db().unwrap().store.get(SOUL_REND).unwrap();
    let result = super::execute_cast(h.app.world_mut(), h.caster, spell, Some(target));
    assert!(matches!(
        result,
        Err(super::CastFailure::Unsupported(detail)) if detail.contains("expected_stat")
    ));
    assert_eq!(mana(&h, target), before);
}

#[test]
fn native_power_drain_target_without_mana_does_not_credit_or_damage() {
    let (mut h, _) = fixture(8);
    spend_intellect_mana(&mut h, 2);
    let target = h.npc;
    assert!(h.app.world().get::<Mana>(target).is_none());
    let caster_before = mana(&h, h.caster);
    let health_before = h.app.world().get::<Health>(target).unwrap().current;
    execute(&mut h, SOUL_REND, target);
    assert_eq!(mana(&h, h.caster), caster_before);
    assert_eq!(h.health(target), health_before);
}
