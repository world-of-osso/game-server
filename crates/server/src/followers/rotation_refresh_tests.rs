//! Actual native casts/resources and reapplication boundaries, no aura/power grants.
use super::kit_tests::level32;
use crate::auras::{ActiveAuras, AuraInstance, aura_now};
use crate::spell_cast::powers::power_value;
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{CastFailure, creature_cast};
use bevy::prelude::*;
use shared::components::{Health, PowerType};

fn resource_entry(
    spec: u32,
    spell: u32,
    resource: super::kit::ResourceCondition,
) -> super::kit::Entry {
    super::kit::Entry {
        spec_id: spec,
        spell_id: spell,
        min_level: 1,
        max_level: 90,
        slot: super::kit::Slot::Attack,
        conditions: super::kit::Conditions {
            resources: vec![resource],
            ..Default::default()
        },
    }
}

fn native_dummy(entry: u32) -> (Harness, Entity) {
    let (mut h, unit) = level32(entry);
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    });
    h.app.world_mut().entity_mut(unit).insert((
        crate::combat::AutoAttack::new(h.npc),
        crate::networking::CombatTarget(Some(h.npc)),
    ));
    (h, unit)
}

fn aura(h: &Harness, unit: Entity, target: Entity, spell: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == unit)
        .unwrap_or_else(|| panic!("native owned aura {spell} missing"))
        .clone()
}

fn retryable(failure: &CastFailure) -> bool {
    matches!(
        failure,
        CastFailure::OnGlobalCooldown
            | CastFailure::OnCooldown
            | CastFailure::NoCharges
            | CastFailure::NotEnoughPower(_)
            | CastFailure::InProgress
    )
}

fn cast_native(h: &mut Harness, unit: Entity, spell: u32, target: Entity) {
    for _ in 0..1_200 {
        match creature_cast(h.app.world_mut(), unit, spell, Some(target), false) {
            Ok(()) => {
                h.advance(3);
                return;
            }
            Err(failure) => assert!(retryable(&failure), "native {spell}: {failure:?}"),
        }
        h.advance(1);
    }
    panic!("native spell {spell} unavailable after 60 simulated seconds");
}

fn bank_power(h: &mut Harness, unit: Entity, power: PowerType, generators: &[(u32, Entity)]) {
    let (_, maximum) = power_value(h.app.world(), unit, power).unwrap();
    generate_power(h, unit, (power, maximum), generators);
}

fn generate_power(
    h: &mut Harness,
    unit: Entity,
    goal: (PowerType, f32),
    generators: &[(u32, Entity)],
) {
    let (power, required) = goal;
    for _ in 0..2_400 {
        let (current, _) = power_value(h.app.world(), unit, power).unwrap();
        if current >= required {
            return;
        }
        for &(spell, target) in generators {
            match creature_cast(h.app.world_mut(), unit, spell, Some(target), false) {
                Ok(()) => break,
                Err(failure) => {
                    assert!(retryable(&failure), "native generator {spell}: {failure:?}")
                }
            }
        }
        h.advance(1);
    }
    panic!("native generators did not fill {power:?}");
}

fn wait_for_pandemic(h: &mut Harness, unit: Entity, target: Entity, spell: u32) -> AuraInstance {
    let old = aura(h, unit, target, spell);
    let duration = f64::from(old.duration_ms) / 1_000.0;
    let until_window = old.expires_at.unwrap() - aura_now(h.app.world()) - duration * 0.20;
    if until_window > 0.0 {
        h.advance((until_window * 20.0).ceil() as usize);
    }
    let old = aura(h, unit, target, spell);
    let remaining = old.expires_at.unwrap() - aura_now(h.app.world());
    assert!(
        remaining > 0.0 && remaining <= duration * 0.30,
        "actual native Pandemic window"
    );
    old
}

#[test]
fn rotation_fidelity_feral_waits_for_five_real_combo_points() {
    let (mut h, unit) = native_dummy(232800);
    let target = h.npc;
    let entry = resource_entry(
        103,
        1079,
        super::kit::ResourceCondition {
            power: PowerType::ComboPoints,
            min: Some(5.0),
            below: None,
            min_deficit: None,
        },
    );
    assert!(
        !super::kit_ai::conditions_met(h.app.world_mut(), unit, target, &[], &[], &entry),
        "Rip must not spend an incomplete native combo-point stack"
    );
    let required = entry.conditions.resources[0].min.unwrap();
    generate_power(
        &mut h,
        unit,
        (entry.conditions.resources[0].power, required),
        &[(1822, target), (5221, target)],
    );
    assert!(super::kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        target,
        &[],
        &[],
        &entry
    ));
    cast_native(&mut h, unit, 1079, target);
    assert!(!super::kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        target,
        &[],
        &[],
        &entry
    ));
}

#[test]
fn rotation_fidelity_windwalker_pools_only_when_two_chi_fit() {
    let (mut h, unit) = native_dummy(227734);
    let target = h.npc;
    let entry = resource_entry(
        269,
        100780,
        super::kit::ResourceCondition {
            power: PowerType::Chi,
            min: None,
            below: None,
            min_deficit: Some(2.0),
        },
    );
    assert!(super::kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        target,
        &[],
        &[],
        &entry
    ));
    bank_power(&mut h, unit, PowerType::Chi, &[(100780, target)]);
    assert!(
        !super::kit_ai::conditions_met(h.app.world_mut(), unit, target, &[], &[], &entry),
        "native Tiger Palm's two Chi must fit instead of overflowing"
    );
    cast_native(&mut h, unit, 107428, target);
    let (current, maximum) = power_value(h.app.world(), unit, PowerType::Chi).unwrap();
    assert!(maximum - current >= entry.conditions.resources[0].min_deficit.unwrap());
    assert!(super::kit_ai::conditions_met(
        h.app.world_mut(),
        unit,
        target,
        &[],
        &[],
        &entry
    ));
}

#[test]
fn rotation_fidelity_demonology_does_not_waste_demonbolt_at_shard_cap() {
    let (mut h, unit) = native_dummy(230861);
    let target = h.npc;
    let entry = resource_entry(
        266,
        264178,
        super::kit::ResourceCondition {
            power: PowerType::SoulShards,
            min: None,
            below: Some(4.0),
            min_deficit: None,
        },
    );
    bank_power(&mut h, unit, PowerType::SoulShards, &[(686, target)]);
    let (raw, _) = power_value(h.app.world(), unit, PowerType::SoulShards).unwrap();
    let scale = h
        .app
        .world()
        .resource::<crate::spell_cast::powers::PowerTypeTable>()
        .get(PowerType::SoulShards)
        .unwrap()
        .display_modifier as f32;
    assert!(raw / scale >= entry.conditions.resources[0].below.unwrap());
    assert!(
        !super::kit_ai::conditions_met(h.app.world_mut(), unit, target, &[], &[], &entry),
        "APL shard<4 gate must use the native raw-to-display divisor"
    );
}

#[test]
fn rotation_fidelity_rake_pandemic_carries_actual_owned_remainder() {
    let (mut h, unit) = native_dummy(232800);
    let target = h.npc;
    cast_native(&mut h, unit, 1822, target);
    let old = wait_for_pandemic(&mut h, unit, target, 155722);
    cast_native(&mut h, unit, 1822, target);
    let refreshed = aura(&h, unit, target, 155722);
    let expected = old.expires_at.unwrap() + f64::from(old.duration_ms) / 1_000.0;
    assert!(
        (refreshed.expires_at.unwrap() - expected).abs() < 0.10,
        "native Rake must carry remaining duration: old={old:?} refreshed={refreshed:?}"
    );
}

#[test]
fn rotation_fidelity_rip_pandemic_carries_five_point_duration() {
    let (mut h, unit) = native_dummy(232800);
    let target = h.npc;
    bank_power(
        &mut h,
        unit,
        PowerType::ComboPoints,
        &[(1822, target), (5221, target)],
    );
    cast_native(&mut h, unit, 1079, target);
    bank_power(
        &mut h,
        unit,
        PowerType::ComboPoints,
        &[(5217, unit), (1822, target), (5221, target)],
    );
    let old = wait_for_pandemic(&mut h, unit, target, 1079);
    cast_native(&mut h, unit, 1079, target);
    let refreshed = aura(&h, unit, target, 1079);
    let expected = old.expires_at.unwrap() + f64::from(old.duration_ms) / 1_000.0;
    assert!(
        (refreshed.expires_at.unwrap() - expected).abs() < 0.10,
        "five-point Rip must retain native duration plus remainder: old={old:?} refreshed={refreshed:?}"
    );
}

#[test]
fn rotation_fidelity_shield_block_extends_instead_of_replacing_remaining_time() {
    let (mut h, unit) = native_dummy(215517);
    let target = h.npc;
    bank_power(
        &mut h,
        unit,
        PowerType::Rage,
        &[(23922, target), (6343, target)],
    );
    cast_native(&mut h, unit, 2565, unit);
    let old = aura(&h, unit, unit, 132404);
    cast_native(&mut h, unit, 2565, unit);
    let refreshed = aura(&h, unit, unit, 132404);
    let expected = old.expires_at.unwrap() + f64::from(old.duration_ms) / 1_000.0;
    assert!(
        (refreshed.expires_at.unwrap() - expected).abs() < 0.10,
        "SimC shield_block_t7196 extends by native duration: old={old:?} refreshed={refreshed:?}"
    );
}

#[test]
fn rotation_fidelity_shield_of_righteous_extends_actual_armor_duration() {
    let (mut h, unit) = native_dummy(209057);
    let target = h.npc;
    let generators = [(275779, target), (204019, target)];
    bank_power(&mut h, unit, PowerType::HolyPower, &generators);
    // Actual white attacks preserve combat/resources while native generator
    // charges refill; no cooldown or resource reset is used.
    h.advance(600);
    cast_native(&mut h, unit, 53600, target);
    let old = aura(&h, unit, unit, 132403);
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(53600)
        .unwrap();
    let required = crate::spell_cast::applicable_costs(world, unit, spell)
        .unwrap()
        .into_iter()
        .filter(|(pool, _, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::HolyPower))
        .map(|(_, amount, _)| amount)
        .sum();
    generate_power(&mut h, unit, (PowerType::HolyPower, required), &generators);
    cast_native(&mut h, unit, 53600, target);
    let refreshed = aura(&h, unit, unit, 132403);
    let expected = old.expires_at.unwrap() + f64::from(old.duration_ms) / 1_000.0;
    assert!(
        (refreshed.expires_at.unwrap() - expected).abs() < 0.10,
        "SimC SoTR buff2864 carries time up to three native durations: old={old:?} refreshed={refreshed:?}"
    );
}
