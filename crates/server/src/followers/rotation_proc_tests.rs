//! Native proc/snapshot lifetimes: ordinary casts, never granted buffs or resources.
use super::rotation_refresh_tests::{cast_native, native_dummy};
use crate::auras::{ActiveAuras, AuraInstance, has_aura};
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const RAKE: u32 = 1822;
const RAKE_BLEED: u32 = 155722;
const TIGERS_FURY: u32 = 5217;

fn owned_bleed(h: &Harness, unit: Entity) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)?
        .iter()
        .find(|aura| aura.spell_id == RAKE_BLEED && aura.caster == unit)
        .cloned()
}

fn apply_rake(h: &mut Harness, unit: Entity) -> AuraInstance {
    for _ in 0..8 {
        cast_native(h, unit, RAKE, h.npc);
        if let Some(aura) = owned_bleed(h, unit) {
            return aura;
        }
    }
    panic!("eight ordinary native Rake casts produced no landed bleed");
}

fn bleed_amount(aura: &AuraInstance) -> f32 {
    aura.effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount
}

#[test]
fn rotation_fidelity_feral_tigers_fury_is_frozen_in_fresh_bleed_amount() {
    let (mut h, unit) = native_dummy(232800);
    assert!(!has_aura(h.app.world(), unit, TIGERS_FURY));
    let baseline = apply_rake(&mut h, unit);
    h.advance(400);
    assert!(owned_bleed(&h, unit).is_none(), "ordinary Rake expired");
    cast_native(&mut h, unit, TIGERS_FURY, unit);
    let fury = h
        .app
        .world()
        .get::<ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TIGERS_FURY)
        .unwrap();
    let native_percent = fury
        .effects
        .iter()
        .find(|effect| effect.misc_values[0] == 22)
        .unwrap()
        .amount;
    let empowered = apply_rake(&mut h, unit);
    let expected = bleed_amount(&baseline) * (1.0 + native_percent / 100.0);
    assert!(
        (bleed_amount(&empowered) - expected).abs() < 0.01,
        "SimC cat_attack_t persistent TF modifier uses actual native5217s3; baseline={}, empowered={}, expected={expected}",
        bleed_amount(&baseline),
        bleed_amount(&empowered)
    );
}

fn ordinary_ticks(h: &Harness, unit: Entity, from: usize) -> Vec<i32> {
    h.inbox.log[from..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(unit.to_bits())
                && event.spell_id == Some(RAKE_BLEED)
                && event.periodic
                && !event.crit
        })
        .map(|event| event.amount)
        .collect()
}

#[test]
fn rotation_fidelity_feral_late_tigers_fury_does_not_empower_existing_bleed() {
    for _ in 0..8 {
        let (mut h, unit) = native_dummy(232800);
        apply_rake(&mut h, unit);
        let start = h.inbox.log.len();
        h.advance(80);
        let before = ordinary_ticks(&h, unit, start);
        let start = h.inbox.log.len();
        cast_native(&mut h, unit, TIGERS_FURY, unit);
        h.advance(80);
        let after = ordinary_ticks(&h, unit, start);
        if let (Some(before), Some(after)) = (before.first(), after.first()) {
            assert_eq!(
                before, after,
                "TF cannot change a bleed cast before its application"
            );
            return;
        }
    }
    panic!("eight natural Rake lifecycles yielded no noncritical before/after ticks");
}
