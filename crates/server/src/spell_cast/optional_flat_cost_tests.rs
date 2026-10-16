//! TC a352b1fa SpellInfo.cpp4220-4226: optional cost ignores flat modifiers.
//! Native429647E6op34 affects only Execute's Prot137048-required cost row.
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;

use super::test_harness::{Harness, harness};

const PROTECTION_WARRIOR: u32 = 137_048;
const ARMS_WARRIOR: u32 = 137_049;
const PRACTICED_STRIKES: u32 = 429_647;
const SUDDEN_DEATH: u32 = 52_437;
const EXECUTE: u32 = 163_201;
const EXECUTE_DAMAGE: u32 = 260_798;
const WARRIOR: u8 = 1;

fn native_harness(spec: u32, talented: bool) -> Harness {
    let mut known = vec![spec, EXECUTE, EXECUTE_DAMAGE, SUDDEN_DEATH];
    if talented {
        known.push(PRACTICED_STRIKES);
    }
    let mut h = harness(Vec::new(), &known, WARRIOR).expect("native70391 Execute fixtures");
    h.advance(8);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, spec));
    // Eligible10% current health, retaining the actual existing maximum.
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 500.0;
    h
}

fn cast_execute(h: &mut Harness, rage: f32) -> f32 {
    // Raw resource input within the unchanged native1000 Rage cap.
    h.set_power(PowerType::Rage, rage);
    h.cast_and_deliver(EXECUTE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(EXECUTE) && event.kind == CombatLogKind::CastSuccess
    }));
    h.power(PowerType::Rage)
}

#[test]
fn optional_flat_native_practiced_strikes_discounts_only_base_protection_execute_cost() {
    let mut ordinary = native_harness(PROTECTION_WARRIOR, false);
    assert_eq!(cast_execute(&mut ordinary, 400.0), 0.0);
    let mut talented = native_harness(PROTECTION_WARRIOR, true);
    // Required200-100, optional200 unchanged: spend300, not200.
    assert_eq!(cast_execute(&mut talented, 400.0), 100.0);
}

#[test]
fn optional_flat_native_practiced_strikes_preserves_available_rage_and_base_admission() {
    for rage in [100.0, 200.0, 300.0] {
        let mut h = native_harness(PROTECTION_WARRIOR, true);
        assert_eq!(cast_execute(&mut h, rage), 0.0);
    }
}

#[test]
fn optional_flat_native_practiced_strikes_does_not_modify_arms_execute_cost_order() {
    let mut h = native_harness(ARMS_WARRIOR, true);
    assert_eq!(cast_execute(&mut h, 400.0), 0.0);
}

fn apply_sudden_death_cost_one(h: &mut Harness) {
    // Native52437 effect1359006: -100% PowerCost1, Execute family mask536870912.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: SUDDEN_DEATH,
            effect_mask: 1 << 5,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        SUDDEN_DEATH
    ));
}

#[test]
fn optional_flat_native_sudden_death_percentage_keeps_protection_optional_spend_free() {
    for talented in [false, true] {
        let mut h = native_harness(PROTECTION_WARRIOR, talented);
        apply_sudden_death_cost_one(&mut h);
        assert_eq!(cast_execute(&mut h, 400.0), 400.0);
    }
}

#[test]
fn optional_flat_native_cost_one_percentage_does_not_modify_arms_cost_zero() {
    let mut h = native_harness(ARMS_WARRIOR, true);
    apply_sudden_death_cost_one(&mut h);
    assert_eq!(cast_execute(&mut h, 400.0), 0.0);
}
