//! Active70448 receipt-matching Fury inputs; TC a352b1fa spell_warrior.cpp1380–1405.
//! Qualification precedes the parent-chain regression cases below.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::protocol::CombatLogKind;

use super::test_harness::{Harness, harness};
use crate::auras::{ActiveAuras, AuraInstance, AuraSource};

const ENRAGED_REGENERATION: u32 = 184_364;
const TALENT: u32 = 383_468;
const HEAL: u32 = 385_174;
const SHATTERING_THROW: u32 = 64_382;
const WOUND: u32 = 209_140;
const NON_BYPASSING_DAMAGE: u32 = 314_616;

#[path = "invigorating_fury_behavior_tests.rs"]
mod behavior;

fn fixture(talent: bool) -> Harness {
    let mut ids = vec![
        ENRAGED_REGENERATION,
        HEAL,
        SHATTERING_THROW,
        WOUND,
        NON_BYPASSING_DAMAGE,
    ];
    if talent {
        ids.push(TALENT);
    }
    let mut h = harness(Vec::new(), &ids, 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(72));
    h.advance(3);
    h
}

fn aura(h: &Harness, spell: u32) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .cloned()
}

fn cast_native_attack(h: &mut Harness, spell: u32) {
    super::creature_cast(h.app.world_mut(), h.npc, spell, Some(h.caster), false).unwrap();
    let dt = h.app.world().resource::<Time>().delta_secs();
    let frames = h.app.world().get::<CastState>(h.npc).map_or(0, |cast| {
        ((cast.duration - cast.elapsed).max(0.0) / dt).ceil() as usize
    });
    h.advance(frames + 3);
}

fn cast_root(h: &mut Harness) {
    h.cast_and_deliver(ENRAGED_REGENERATION, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(aura(h, ENRAGED_REGENERATION).is_some());
}

fn assert_native_recovery(h: &Harness) {
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|entry| entry.spell_id == ENRAGED_REGENERATION && !entry.is_gcd)
        .unwrap();
    assert_eq!(recovery.duration_ms, 120000);
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == ENRAGED_REGENERATION
                && entry.is_gcd
                && entry.duration_ms > 0)
    );
}

fn expire_root_naturally(h: &mut Harness, expiry: f64) {
    while h.app.world().resource::<Time>().elapsed_secs_f64() + 0.1 < expiry {
        assert!(aura(h, ENRAGED_REGENERATION).is_some());
        h.advance(1);
    }
    h.advance(4);
    assert!(aura(h, ENRAGED_REGENERATION).is_none());
}

#[test]
fn native_invigorating_fury_enraged_baseline_defense_duration_and_recovery_are_qualified() {
    let mut h = fixture(false);
    assert!(aura(&h, TALENT).is_none());
    cast_root(&mut h);
    let root = aura(&h, ENRAGED_REGENERATION).unwrap();
    assert_eq!(root.caster, h.caster);
    assert_eq!(root.duration_ms, 8000);
    assert!(root.effects.iter().any(|effect| effect.index == 0
        && effect.aura_type == 87
        && effect.amount == -30.0
        && effect.misc_values[0] == 127));
    assert_native_recovery(&h);
    assert_eq!(h.health(h.caster), 1000.0);
    cast_native_attack(&mut h, NON_BYPASSING_DAMAGE);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.spell_id == Some(NON_BYPASSING_DAMAGE) && event.kind == CombatLogKind::Damage
        })
        .unwrap();
    assert_eq!(
        (damage.amount, damage.absorbed, damage.overflow),
        (700, 0, 0)
    );
    assert_eq!(h.health(h.caster), 300.0);
    expire_root_naturally(&mut h, root.expires_at.unwrap());
    // Normal out-of-combat regeneration remains enabled; do not freeze wounded HP.
    eprintln!(
        "ordinary184364 native8s/-30%all-school/120srecovery/noGCD; actual3146161000damage→700/HP300; natural8sexpiry qualified"
    );
}

#[test]
fn native_invigorating_fury_talent_extends_native_root_to_eleven_seconds() {
    let mut h = fixture(true);
    let talent = aura(&h, TALENT).unwrap();
    assert_eq!(talent.source, AuraSource::KnownSpell);
    assert_eq!(talent.caster, h.caster);
    assert!(talent.expires_at.is_none());
    assert!(talent.effects.iter().any(|effect| effect.index == 0
        && effect.aura_type == 107
        && effect.amount == 3000.0
        && effect.misc_values[0] == 1
        && effect.class_mask[2] == 1));
    cast_root(&mut h);
    let root = aura(&h, ENRAGED_REGENERATION).unwrap();
    assert_eq!(root.duration_ms, 11000);
    assert_native_recovery(&h);
    expire_root_naturally(&mut h, root.expires_at.unwrap());
    assert!(aura(&h, TALENT).is_some());
    assert_eq!(h.health(h.caster), 1000.0);
    eprintln!(
        "actualKnownSpell383468A107/native+3000ms gives ordinary184364native11s naturalexpiry/120srecovery; NOT parent-heal chain proof"
    );
}

#[test]
fn native_invigorating_fury_limited_heal_preserves_actual_warrior_cast_and_payload() {
    let mut h = fixture(false);
    cast_native_attack(&mut h, WOUND);
    assert_eq!(h.health(h.caster), 200.0);
    let (owner, target) = (h.caster, h.npc);
    h.cast_and_deliver(SHATTERING_THROW, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let before = h
        .app
        .world()
        .get::<CastState>(owner)
        .expect("actual Warrior64382 must still be casting")
        .clone();
    assert_eq!(before.spell_id, SHATTERING_THROW);
    assert_eq!(before.duration, 1.5);
    assert!(
        h.app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(HEAL)
            .unwrap()
            .power_costs
            .is_empty()
    );
    super::creature_cast_ignoring_in_progress(h.app.world_mut(), owner, HEAL, None)
        .expect("existing limited child path must support actual Warrior self-heal during cast");
    let after = h
        .app
        .world()
        .get::<CastState>(owner)
        .expect("limited native385174 must preserve the actual in-progress Warrior cast");
    assert_eq!(
        (
            after.spell_id,
            after.target,
            after.cast_type,
            after.duration,
            after.elapsed
        ),
        (
            before.spell_id,
            before.target,
            before.cast_type,
            before.duration,
            before.elapsed
        )
    );
    h.advance(3);
    assert_eq!(h.health(owner), 300.0);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(HEAL) && event.kind == CombatLogKind::Heal)
        .unwrap();
    assert_eq!(
        (
            heal.source,
            heal.target,
            heal.amount,
            heal.overflow,
            heal.crit
        ),
        (Some(owner.to_bits()), Some(owner.to_bits()), 100, 0, false)
    );
    for _ in 0..100 {
        if h.app.world().get::<CastState>(owner).is_none() {
            break;
        }
        h.advance(1);
    }
    h.advance(10); // Observe the source cast's real completion/owner delivery and native flight.
    assert!(h.app.world().get::<CastState>(owner).is_none());
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHATTERING_THROW)
                && event.kind == CombatLogKind::CastSuccess
                && event.source == Some(owner.to_bits())
                && event.target == Some(target.to_bits()))
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == HEAL && entry.duration_ms > 0)
    );
    eprintln!(
        "native385174 actual100heal/HP200→300/noCD preserves actualWarrior64382native1500msstate/ownedCastSuccess via existinglimitedflags;noHP/clock/RNGwrites"
    );
}
