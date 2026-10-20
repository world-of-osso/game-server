//! TC a352b1fa spell_warrior.cpp1145–1164; active70448 receipt-matching inputs.
//! Qualification only. VICTORIOUS_KILL_PRODUCER_UNIMPLEMENTED: direct native32216
//! application qualifies removal/lifetime inputs, never ordinary-kill production.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Position, PowerType};
use shared::protocol::CombatLogKind;

use super::test_harness::{Harness, charge_range, harness};
use crate::auras::{ActiveAuras, AuraInstance};

const ROOT: u32 = 202_168;
const HEAL: u32 = 202_166;
const VICTORIOUS: u32 = 32_216;
const CHARGE: u32 = 100;
const CHARGE_EFFECT: u32 = 198_337;
const WOUND: u32 = 209_140;
const SHATTERING_THROW: u32 = 64_382;

#[path = "impending_victory_behavior_tests.rs"]
mod behavior;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            ROOT,
            HEAL,
            VICTORIOUS,
            CHARGE,
            CHARGE_EFFECT,
            WOUND,
            SHATTERING_THROW,
        ],
        1,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(72));
    h.advance(3);
    h
}

fn aura(h: &Harness, unit: Entity, spell: u32) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .cloned()
}

fn wound(h: &mut Harness) {
    super::creature_cast(h.app.world_mut(), h.npc, WOUND, Some(h.caster), false).unwrap();
    let dt = h.app.world().resource::<Time>().delta_secs();
    let cast = h.app.world().get::<CastState>(h.npc).unwrap();
    let frames = ((cast.duration - cast.elapsed) / dt).ceil() as usize;
    h.advance(frames + 3);
    assert_eq!(h.health(h.caster), 200.0);
}

fn charge_for_real_rage(h: &mut Harness) {
    charge_range(h);
    h.cast_and_deliver(CHARGE, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), 200.0);
    h.advance(40); // Natural global recovery; this qualifies energize, not Charge movement.
    assert!(h.power(PowerType::Rage) >= 100.0);
    // Melee-range fixture placement after the actual ranged energize. No motion claim.
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
}

#[test]
fn native_impending_victory_direct_heal_payload_preserves_actual_warrior_cast() {
    let mut h = fixture();
    wound(&mut h);
    h.cast_and_deliver(SHATTERING_THROW, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let before = h.app.world().get::<CastState>(h.caster).unwrap().clone();
    assert_eq!((before.spell_id, before.duration), (SHATTERING_THROW, 1.5));
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        HEAL,
        Some(h.caster),
        0
    ));
    let after = h
        .app
        .world()
        .get::<CastState>(h.caster)
        .expect("native triggered child must preserve actual Warrior cast");
    assert_eq!(
        (after.spell_id, after.target, after.duration, after.elapsed),
        (
            before.spell_id,
            before.target,
            before.duration,
            before.elapsed
        )
    );
    h.advance(3);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(HEAL) && event.kind == CombatLogKind::Heal)
        .unwrap();
    let expected = if heal.crit { 600 } else { 300 };
    assert_eq!(
        (heal.source, heal.target, heal.amount, heal.overflow),
        (
            Some(h.caster.to_bits()),
            Some(h.caster.to_bits()),
            expected,
            0
        )
    );
    assert_eq!(h.health(h.caster), 200.0 + expected as f32);
    h.advance(100);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHATTERING_THROW)
                && event.kind == CombatLogKind::CastSuccess
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits()))
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == HEAL && entry.duration_ms > 0)
    );
    eprintln!(
        "native20216630%heal observed{expected}/preservedactualWarrior643821500msstate+ownedcompletion/sourceFULLtrigger/nativecritunchanged"
    );
}

#[test]
fn native_impending_victory_root_uses_real_charge_rage_and_native_recovery() {
    let mut h = fixture();
    charge_for_real_rage(&mut h);
    let before = h.power(PowerType::Rage);
    h.cast_and_deliver(ROOT, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.power(PowerType::Rage), before - 100.0);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == ROOT && !entry.is_gcd && entry.duration_ms == 25000)
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == ROOT && entry.is_gcd && entry.duration_ms == 1500)
    );
    h.advance(3);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.spell_id == Some(ROOT)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .unwrap();
    if hit.kind == CombatLogKind::Damage {
        let normal = hit.amount as f32 / if hit.crit { 2.0 } else { 1.0 };
        assert!(
            (57.0..=61.0).contains(&normal),
            "nativeAP0.59/variance0.05:{hit:?}"
        );
    }
    assert_eq!(
        (hit.source, hit.target),
        (Some(h.caster.to_bits()), Some(h.npc.to_bits()))
    );
    eprintln!(
        "real100Charge→200rawRage;ordinary202168takes100raw/native25sCD1500GCD/nativeAP0.59 actual hit{hit:?};ChargeMOVEMENTunqualified"
    );
}

#[test]
fn native_impending_victory_zero_rage_rejects_root_without_payment() {
    let mut h = fixture();
    assert_eq!(h.power(PowerType::Rage), 0.0);
    h.cast_and_deliver(ROOT, Some(h.npc));
    h.advance(3);
    assert!(!h.inbox.failed.is_empty());
    assert_eq!(h.power(PowerType::Rage), 0.0);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(ROOT) && event.kind == CombatLogKind::CastSuccess)
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == ROOT && entry.duration_ms > 0)
    );
}

#[test]
fn native_impending_victory_direct_victorious_has_native_twenty_second_lifetime() {
    let mut h = fixture();
    h.cast_and_deliver(VICTORIOUS, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let applied = aura(&h, h.caster, VICTORIOUS).expect("actual native32216 direct application");
    assert_eq!((applied.caster, applied.duration_ms), (h.caster, 20000));
    assert!(applied.effects.iter().any(|effect| effect.aura_type == 108
        && effect.amount == -100.0
        && effect.misc_values[0] == 14));
    let expiry = applied.expires_at.unwrap();
    while h.app.world().resource::<Time>().elapsed_secs_f64() + 0.1 < expiry {
        assert!(aura(&h, h.caster, VICTORIOUS).is_some());
        h.advance(1);
    }
    h.advance(4);
    assert!(aura(&h, h.caster, VICTORIOUS).is_none());
    eprintln!(
        "native32216directspell20snaturalexpiry;VICTORIOUS_KILL_PRODUCER_UNIMPLEMENTED/removalinputONLY/noordinarykillclaim"
    );
}

#[test]
fn native_impending_victory_actual_victorious_makes_native_root_free() {
    let mut h = fixture();
    assert_eq!(h.power(PowerType::Rage), 0.0);
    h.cast_and_deliver(VICTORIOUS, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(aura(&h, h.caster, VICTORIOUS).is_some());
    h.cast_and_deliver(ROOT, Some(h.npc));
    h.advance(3);
    assert!(
        h.inbox.failed.is_empty(),
        "native32216 A108 should make202168 free:{:?}",
        h.inbox.failed
    );
    assert_eq!(h.power(PowerType::Rage), 0.0);
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(ROOT)
        && event.kind == CombatLogKind::CastSuccess
        && event.source == Some(h.caster.to_bits())));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == ROOT && !entry.is_gcd && entry.duration_ms == 25000)
    );
    // Do not assert aura retention: qualification must remain valid after source removal binds.
    eprintln!(
        "nativeDIRECT32216A108maskword2=256→202168freeat0Rage/native25srecovery;killproducerunimplemented"
    );
}
