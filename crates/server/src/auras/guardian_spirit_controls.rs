//! Native lifetime and ordered-melee controls; not a legal loadout certificate.
use super::guardian_spirit_tests::{
    ANNIHILATION, DEATH_BY_DARKNESS, GUARDIAN_HEAL, GUARDIAN_SPIRIT, cast_guardian,
    cast_native_attack, fixture,
};
use super::has_aura;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc};
use shared::protocol::{CombatLogKind, MissKind};

#[test]
fn guardian_spirit_native_nonlethal_then_expiry_does_not_rescue() {
    let mut h = fixture();
    cast_guardian(&mut h);
    cast_native_attack(&mut h, ANNIHILATION);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(has_aura(h.app.world(), h.caster, GUARDIAN_SPIRIT));
    let duration = h
        .app
        .world()
        .get::<super::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == GUARDIAN_SPIRIT)
        .unwrap()
        .duration_ms;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    h.advance((duration as f32 / 1000.0 / dt).ceil() as usize + 3);
    assert!(!has_aura(h.app.world(), h.caster, GUARDIAN_SPIRIT));
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 0.0);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(GUARDIAN_HEAL))
    );
}

#[test]
fn guardian_spirit_native_rescue_is_one_shot_for_direct_hits() {
    let mut h = fixture();
    cast_native_attack(&mut h, ANNIHILATION);
    cast_guardian(&mut h);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 600.0);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 0.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == CombatLogKind::Heal && event.spell_id == Some(GUARDIAN_HEAL)
            )
            .count(),
        1
    );
}

#[test]
fn guardian_spirit_native_exact_twice_maximum_is_not_absorbed() {
    // Consume461408: base-difficulty2000, zero coefficient/variance.
    let mut h = harness(Vec::new(), &[GUARDIAN_SPIRIT, GUARDIAN_HEAL, 461_408], 5).unwrap();
    cast_guardian(&mut h);
    cast_native_attack(&mut h, 461_408);
    assert_eq!(h.health(h.caster), 0.0);
    assert!(!h.inbox.log.iter().any(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(GUARDIAN_HEAL)));
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(461_408))
        .unwrap();
    assert_eq!((hit.amount, hit.absorbed), (2000, 0));
}

#[test]
fn guardian_spirit_native_periodic_rescue_then_next_tick_kills() {
    // Burning Blood98957: one native800-damage effect every2000ms.
    let mut h = harness(
        Vec::new(),
        &[GUARDIAN_SPIRIT, GUARDIAN_HEAL, ANNIHILATION, 98_957],
        5,
    )
    .unwrap();
    cast_native_attack(&mut h, ANNIHILATION);
    cast_guardian(&mut h);
    cast_native_attack(&mut h, 98_957);
    let period = h
        .app
        .world()
        .get::<super::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 98_957)
        .unwrap()
        .effects[0]
        .period_ms;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let frames = (period as f32 / 1000.0 / dt).ceil() as usize;
    h.advance(frames);
    assert_eq!(h.health(h.caster), 600.0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(98_957)
                && event.periodic
                && event.amount == 0
                && event.absorbed == 800
                && event.overflow == 0)
    );
    h.advance(frames);
    assert_eq!(h.health(h.caster), 0.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == CombatLogKind::Heal && event.spell_id == Some(GUARDIAN_HEAL)
            )
            .count(),
        1
    );
}

fn two_native_melee_attackers(h: &mut Harness) {
    let other = spawn_npc(&mut h.app, 3.0);
    for attacker in [h.npc, other] {
        // Actual NPC Hit Buffs62627: hit100/expertise10000, no RNG/flag overrides.
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            attacker,
            62_627,
            Some(attacker),
            false,
        )
        .unwrap();
    }
    h.advance(3);
    for attacker in [h.npc, other] {
        h.app
            .world_mut()
            .entity_mut(attacker)
            .insert(crate::combat::AutoAttack::new(h.caster));
    }
    h.advance(3);
}

#[test]
fn guardian_spirit_native_shared_ardent_two_melee_hits_consume_once() {
    let mut h = harness(Vec::new(), &[31_850, ANNIHILATION, 381_775, 62_627], 2).unwrap();
    cast_native_attack(&mut h, ANNIHILATION);
    cast_native_attack(&mut h, 381_775);
    assert_eq!(h.health(h.caster), 1.0);
    h.cast_and_deliver(31_850, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, 31_850));
    two_native_melee_attackers(&mut h);
    let swings: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id.is_none()
                && event.target == Some(h.caster.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::Damage | CombatLogKind::Miss(MissKind::Absorb)
                )
        })
        .collect();
    assert_eq!(swings.len(), 2, "{swings:?}");
    assert!(swings[0].absorbed > 0 && swings[0].amount == 0);
    assert!(swings[1].amount > 0 && swings[1].absorbed == 0);
    // Untuned fixture NPCs use combat::BASE_DAMAGE10; the consumed Ardent's
    //30% reduction must not survive into the next same-frame swing.
    assert_eq!(swings[1].amount, if swings[1].crit { 20 } else { 10 });
    assert_eq!(h.health(h.caster), 200.0 - swings[1].amount as f32);
    assert!(!has_aura(h.app.world(), h.caster, 31_850));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Death)
    );
    let combat = h.app.world().resource::<crate::combat::CombatLog>();
    let entries = combat.range(0.0, f64::MAX);
    let attacks: Vec<_> = entries
        .iter()
        .filter(|entry| entry.event.spell_id == 0 && entry.event.target == h.caster.to_bits())
        .collect();
    assert_eq!(attacks.len(), 2);
    assert_eq!(attacks[0].timestamp, attacks[1].timestamp);
}

#[test]
fn guardian_spirit_native_shared_ardent_periodic_reports_partial_absorption() {
    let mut h = harness(Vec::new(), &[31_850, DEATH_BY_DARKNESS, 98_957], 2).unwrap();
    h.cast_and_deliver(31_850, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    cast_native_attack(&mut h, DEATH_BY_DARKNESS);
    assert_eq!(h.health(h.caster), 300.0);
    cast_native_attack(&mut h, 98_957);
    let period = h
        .app
        .world()
        .get::<super::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 98_957)
        .unwrap()
        .effects[0]
        .period_ms;
    let dt = h.app.world().resource::<bevy::prelude::Time>().delta_secs();
    let frames = (period as f32 / 1000.0 / dt).ceil() as usize;
    h.advance(frames);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!has_aura(h.app.world(), h.caster, 31_850));
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(98_957) && event.periodic
        })
        .unwrap();
    assert_eq!((hit.amount, hit.absorbed, hit.overflow), (100, 460, 0));
    h.advance(frames);
    assert_eq!(h.health(h.caster), 0.0);
}

#[test]
fn guardian_spirit_native_two_melee_hits_commit_real_child_before_second() {
    let mut h = harness(
        Vec::new(),
        &[
            GUARDIAN_SPIRIT,
            GUARDIAN_HEAL,
            ANNIHILATION,
            381_775,
            62_627,
        ],
        5,
    )
    .unwrap();
    cast_native_attack(&mut h, ANNIHILATION);
    cast_native_attack(&mut h, 381_775);
    assert_eq!(h.health(h.caster), 1.0);
    cast_guardian(&mut h);
    two_native_melee_attackers(&mut h);
    let swings: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id.is_none()
                && event.target == Some(h.caster.to_bits())
                && matches!(
                    event.kind,
                    CombatLogKind::Damage | CombatLogKind::Miss(MissKind::Absorb)
                )
        })
        .collect();
    assert_eq!(swings.len(), 2, "{swings:?}");
    assert!(swings[0].absorbed > 0 && swings[0].amount == 0);
    assert!(swings[1].amount > 0 && swings[1].absorbed == 0);
    assert_eq!(h.health(h.caster), 401.0 - swings[1].amount as f32);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == CombatLogKind::Heal && event.spell_id == Some(GUARDIAN_HEAL)
            )
            .count(),
        1
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Death)
    );
    let combat = h.app.world().resource::<crate::combat::CombatLog>();
    let entries = combat.range(0.0, f64::MAX);
    let attacks: Vec<_> = entries
        .iter()
        .filter(|entry| entry.event.spell_id == 0 && entry.event.target == h.caster.to_bits())
        .collect();
    assert_eq!(attacks.len(), 2);
    assert_eq!(attacks[0].timestamp, attacks[1].timestamp);
}
