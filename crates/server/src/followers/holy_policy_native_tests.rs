//! Real 70448 percentage damage on a native level90 group, not a trivial Stockade overlevel.
use super::kit_clearcasting_tests::complete_real_cast;
use super::kit_tests::{at_level, uses_spell};
use crate::auras::{ActiveAuras, has_aura};
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents};
use bevy::prelude::*;
use shared::components::{Health, Level, UnitLevel};
use shared::protocol::CombatLogKind;

const HENRY: u32 = 215011;
/// Native165/10% max-health, source-centered enemy area; no fabricated spell row.
const BURST_OF_HATRED: u32 = 124257;
/// Native NPC Shadow Word: Pain has fixed72/3s, unlike SP-only player589.
const MAGIC_DOT: u32 = 17146;
/// Fixture pressure spacing, not a claim about a Retail encounter timer.
const PRESSURE_FRAMES: usize = 100;

struct GroupFixture {
    h: Harness,
    priest: Entity,
    members: Vec<Entity>,
}

fn native_group() -> GroupFixture {
    let (mut h, priest) = at_level(HENRY, 90, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
        app.init_resource::<ObservedCombatEvents>();
    });
    let owner = h.caster;
    h.app.world_mut().entity_mut(owner).insert(UnitLevel(90));
    // This damage-only dummy's hit level matches the group. Native165 reads
    // recipient maximum health; caster health/power are never substituted.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((Level(90), UnitLevel(90)));
    crate::combat::attack_stop(h.app.world_mut(), owner);
    let mut members = vec![owner, priest];
    for entry in [209057, 209053, 209059] {
        let unit = super::native_fixture::spawn(h.app.world_mut(), owner, entry).unwrap();
        members.push(unit);
    }
    h.advance(3);
    for &member in &members {
        assert_eq!(h.app.world().get::<Level>(member).unwrap().0, 90);
        assert_eq!(h.app.world().get::<UnitLevel>(member).unwrap().0, 90);
        let health = h.app.world().get::<Health>(member).unwrap();
        assert!(
            health.max > 1000.0,
            "native90 stats must replace the small harness pool"
        );
        assert_eq!(health.current, health.max);
    }
    GroupFixture { h, priest, members }
}

fn apply_pressure(group: &mut GroupFixture, rounds: usize) {
    for _ in 0..rounds {
        let enemy = group.h.npc;
        creature_cast(group.h.app.world_mut(), enemy, BURST_OF_HATRED, None, false).unwrap();
        group.h.advance(PRESSURE_FRAMES);
    }
}

fn fraction(group: &GroupFixture, member: Entity) -> f32 {
    let health = group.h.app.world().get::<Health>(member).unwrap();
    health.current / health.max
}

fn owned_ward(group: &GroupFixture) -> bool {
    group.members.iter().any(|&member| {
        group
            .h
            .app
            .world()
            .get::<ActiveAuras>(member)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == 41635 && aura.caster == group.priest)
            })
    })
}

fn casts(group: &GroupFixture, spell: u32) -> usize {
    group
        .h
        .inbox
        .go
        .iter()
        .filter(|event| event.caster == group.priest.to_bits() && event.spell_id == spell)
        .count()
}

#[test]
fn holy_policy_native90_fixture_has_real_group_damage_and_native_stats() {
    let mut group = native_group();
    apply_pressure(&mut group, 6);
    let wounded = group
        .members
        .iter()
        .filter(|&&member| fraction(&group, member) < 0.55)
        .count();
    assert!(
        wounded >= 3,
        "authentic damage must put a real group under pressure, got {wounded}"
    );
    assert!(
        group
            .members
            .iter()
            .all(|&member| fraction(&group, member) > 0.0)
    );
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(BURST_OF_HATRED)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
fn holy_policy_native90_group_pressure_uses_real_hymn_ticks() {
    let mut group = native_group();
    apply_pressure(&mut group, 6);
    uses_spell(&mut group.h, group.priest, 64843);
    group.h.advance(200);
    let recipients: std::collections::BTreeSet<_> = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(group.priest.to_bits())
                && event.spell_id == Some(64844)
                && event.kind == CombatLogKind::Heal
                && event.amount > event.overflow
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(
        recipients.len() >= 3,
        "real Hymn must effectively heal the injured group"
    );
}

#[test]
fn holy_policy_native90_depleted_words_under_pressure_use_apotheosis() {
    let mut group = native_group();
    complete_real_cast(&mut group.h, group.priest, 2050);
    complete_real_cast(&mut group.h, group.priest, 2050);
    apply_pressure(&mut group, 6);
    uses_spell(&mut group.h, group.priest, 200183);
    assert!(has_aura(group.h.app.world(), group.priest, 200183));
}

#[test]
fn holy_policy_native90_fortitude_covers_the_actual_party() {
    let mut group = native_group();
    for _ in 0..600 {
        super::update_ai(group.h.app.world_mut());
        group.h.advance(1);
        if group
            .members
            .iter()
            .all(|&member| has_aura(group.h.app.world(), member, 21562))
        {
            return;
        }
    }
    let missing: Vec<_> = group
        .members
        .iter()
        .copied()
        .filter(|&member| !has_aura(group.h.app.world(), member, 21562))
        .collect();
    assert!(
        missing.is_empty(),
        "native party recipients missing Fortitude: {missing:?}"
    );
}

#[test]
fn holy_policy_native90_purify_removes_real_magic_during_triage_pressure() {
    let mut group = native_group();
    let target = group.members[2];
    let enemy = group.h.npc;
    creature_cast(
        group.h.app.world_mut(),
        enemy,
        MAGIC_DOT,
        Some(target),
        false,
    )
    .unwrap();
    group.h.advance(75);
    assert!(has_aura(group.h.app.world(), target, MAGIC_DOT));
    assert!(
        fraction(&group, target) < 1.0,
        "the real periodic damage must land before AI admission"
    );
    uses_spell(&mut group.h, group.priest, 527);
    group.h.advance(5);
    assert!(!has_aura(group.h.app.world(), target, MAGIC_DOT));
}

#[test]
fn holy_policy_native90_mending_keeps_the_owned_bounced_ward() {
    let mut group = native_group();
    apply_pressure(&mut group, 2);
    uses_spell(&mut group.h, group.priest, 33076);
    group.h.advance(3);
    assert!(owned_ward(&group));
    apply_pressure(&mut group, 1);
    group.h.advance(220);
    assert!(
        owned_ward(&group),
        "a naturally bounced ward must still belong to Henry"
    );
    assert_eq!(casts(&group, 33076), 1);
    for _ in 0..45 {
        super::update_ai(group.h.app.world_mut());
        group.h.advance(1);
    }
    assert_eq!(
        casts(&group, 33076),
        1,
        "root33076 absence must not overwrite41635 on another holder"
    );
}

#[test]
fn holy_policy_native90_critical_patient_receives_guardian_spirit() {
    let mut group = native_group();
    apply_pressure(&mut group, 9);
    assert!(
        group
            .members
            .iter()
            .any(|&member| fraction(&group, member) < 0.20)
    );
    uses_spell(&mut group.h, group.priest, 47788);
}

#[test]
fn holy_policy_native90_incoming_self_damage_uses_desperate_prayer() {
    let mut group = native_group();
    apply_pressure(&mut group, 6);
    assert!(fraction(&group, group.priest) < 0.50);
    let before = group.h.health(group.priest);
    uses_spell(&mut group.h, group.priest, 19236);
    assert!(group.h.health(group.priest) > before);
}
