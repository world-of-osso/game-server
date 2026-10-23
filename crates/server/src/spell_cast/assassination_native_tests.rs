//! TC a352b1fa spell_rog_improved_garrote_damage: caster-owned effect1 snapshots.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use bevy::prelude::*;
use shared::protocol::CombatLogKind;

const GARROTE: u32 = 703;
const AFTER_STEALTH: u32 = 392401;
const STEALTH: u32 = 392403;
const STEALTH_SPELL: u32 = 1784;
const IMPROVED_GARROTE: u32 = 381632;

fn apply_native_aura(h: &mut Harness, caster: Entity, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn garrote_tick(buffs: &[u32], foreign: bool, remove_before_tick: bool) -> (f32, i32) {
    let mut h = harness(Vec::new(), &[GARROTE, AFTER_STEALTH, STEALTH], 4).unwrap();
    h.stop_attack();
    let caster = h.caster;
    let target = h.npc;
    let buff_owner = if foreign { target } else { caster };
    for &spell in buffs {
        apply_native_aura(&mut h, buff_owner, buff_owner, spell);
    }
    apply_native_aura(&mut h, caster, target, GARROTE);
    let amount = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == GARROTE && aura.caster == caster)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    if remove_before_tick {
        crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            buffs.contains(&aura.spell_id)
        });
        assert!(
            buffs
                .iter()
                .all(|spell| !crate::auras::has_aura(h.app.world(), caster, *spell))
        );
    }
    h.advance(45);
    let tick = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.periodic
                && event.spell_id == Some(GARROTE)
                && event.source == Some(caster.to_bits())
        })
        .expect("ordinary native Garrote must produce its real periodic damage");
    (amount, tick.amount)
}

#[test]
fn assassination_garrote_native_stealth_earns_and_replaces_its_owned_bonus() {
    let mut h = harness(
        Vec::new(),
        &[
            GARROTE,
            AFTER_STEALTH,
            STEALTH,
            STEALTH_SPELL,
            IMPROVED_GARROTE,
        ],
        4,
    )
    .unwrap();
    let caster = h.caster;
    let enemy = h.npc;
    crate::combat::combat_stop(h.app.world_mut(), caster);
    crate::combat::combat_stop(h.app.world_mut(), enemy);
    apply_native_aura(&mut h, caster, caster, IMPROVED_GARROTE);
    h.cast_and_deliver(STEALTH_SPELL, Some(caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, STEALTH));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        AFTER_STEALTH
    ));
    crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == STEALTH_SPELL
    });
    assert!(!crate::auras::has_aura(h.app.world(), caster, STEALTH));
    assert!(crate::auras::has_aura(h.app.world(), caster, AFTER_STEALTH));
}

#[test]
fn assassination_garrote_native_unbuffed_tick_is_positive() {
    let (amount, tick) = garrote_tick(&[], false, false);
    assert!(amount > 0.0 && tick > 0);
}

#[test]
fn assassination_garrote_native_foreign_stealth_bonus_does_not_apply() {
    let ordinary = garrote_tick(&[], false, false);
    assert_eq!(garrote_tick(&[STEALTH], true, false), ordinary);
}

#[test]
fn assassination_garrote_native_improved_bonus_snapshots_through_removal() {
    let (ordinary_amount, ordinary_tick) = garrote_tick(&[], false, false);
    for buffs in [
        &[STEALTH][..],
        &[AFTER_STEALTH][..],
        &[STEALTH, AFTER_STEALTH][..],
    ] {
        let (amount, tick) = garrote_tick(buffs, false, true);
        assert!(
            (amount - ordinary_amount * 1.5).abs() < 0.01,
            "native50% effect1 must snapshot once, including when both child buffs exist: {amount} vs {ordinary_amount}"
        );
        assert!(
            (tick as f32 - ordinary_tick as f32 * 1.5).abs() <= 2.0,
            "earned bonus must affect actual ticks after the buff is removed: {tick} vs {ordinary_tick}"
        );
    }
}
