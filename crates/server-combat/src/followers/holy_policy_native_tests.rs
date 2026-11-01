//! Real 70448 percentage damage on a native level90 group, not a trivial Stockade overlevel.
mod echo_of_light;
use super::kit_clearcasting_tests::{complete_real_cast, mana_cost, own_aura};
use super::kit_tests::at_level;
use crate::auras::has_aura;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents};
use bevy::prelude::*;
use shared::components::{
    CombatRatings, DerivedStats, Health, Level, Player, UnitLevel, UnitStats,
};
use shared::group::GroupRole;
use shared::protocol::CombatLogKind;

const HENRY: u32 = 215011;
/// Native165/10% max-health, source-centered enemy area; no fabricated spell row.
const BURST_OF_HATRED: u32 = 124257;
/// Fixture pressure spacing, not a claim about a Retail encounter timer.
const PRESSURE_FRAMES: usize = 100;

struct GroupFixture {
    h: Harness,
    priest: Entity,
    members: Vec<Entity>,
}

fn native_group() -> GroupFixture {
    native_group_at(90)
}

fn native_group_at(level: u8) -> GroupFixture {
    let (mut h, priest) = at_level(HENRY, level, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
        app.init_resource::<ObservedCombatEvents>();
    });
    let owner = h.caster;
    // The fixed-stat cast harness omits the three stat projection components.
    // Install the real entry shape so StatsPlugin computes native90 values.
    h.app.world_mut().entity_mut(owner).insert((
        Level(level),
        UnitLevel(level),
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
    ));
    // This damage-only dummy's hit level matches the group. Native165 reads
    // recipient maximum health; caster health/power are never substituted.
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((Level(level), UnitLevel(level)));
    crate::combat::attack_stop(h.app.world_mut(), owner);
    let mut members = vec![owner, priest];
    for entry in [209057, 209053, 209059] {
        let unit = super::native_fixture::spawn(h.app.world_mut(), owner, entry).unwrap();
        members.push(unit);
    }
    form_native_party(&mut h, &members, level);
    h.advance(3);
    for &member in &members {
        assert_eq!(h.app.world().get::<Level>(member).unwrap().0, level);
        assert_eq!(h.app.world().get::<UnitLevel>(member).unwrap().0, level);
        let health = h.app.world().get::<Health>(member).unwrap();
        let stats = h
            .app
            .world()
            .get::<crate::stats::CharacterStats>(member)
            .unwrap();
        assert!(
            stats.primary.stamina > 0.0,
            "native{level} primary stats missing for {member:?}"
        );
        assert_eq!(
            health.max, stats.max_health,
            "native derived health projection for {member:?}"
        );
        assert_eq!(health.current, health.max);
    }
    GroupFixture { h, priest, members }
}

fn form_native_party(h: &mut Harness, members: &[Entity], level: u8) {
    let owner = h.caster;
    let player = h.app.world().get::<Player>(owner).unwrap();
    let owner_info = OnlineMember {
        entity: owner.to_bits(),
        name: player.name.clone(),
        class: player.class,
        level,
        portrait: default(),
        dungeon_difficulty: 1,
        raid_difficulty: default(),
    };
    h.app.world_mut().entity_mut(owner).insert(ClientId(1));
    let mut view = OnlineView::from([(1, owner_info)]);
    let mut companions = Vec::new();
    for &unit in members.iter().filter(|&&unit| unit != owner) {
        let follower = h.app.world().get::<super::Follower>(unit).unwrap();
        let member = super::Follower::member_id(unit);
        let display_id = super::native_fixture::template_display(follower.profile.entry);
        view.insert(member, follower.online(unit, level, 1, display_id));
        companions.push((member, follower.profile.role));
    }
    let mut groups = GroupRegistry::default();
    groups
        .create_companion_party(1, GroupRole::Dps, &companions, &view)
        .unwrap();
    assert!(
        companions
            .iter()
            .all(|(member, _)| groups.is_in_same_group(1, *member))
    );
    h.app.insert_resource(groups);
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

fn cast_binding_source(group: &mut GroupFixture, spell: u32, target: Entity) {
    super::kit_clearcasting_tests::wait_for_gcd(&mut group.h, group.priest, spell);
    creature_cast(
        group.h.app.world_mut(),
        group.priest,
        spell,
        Some(target),
        false,
    )
    .unwrap();
    group.h.advance(200);
}

#[test]
fn holy_binding_heals_native90_other_heal_copies_pre_overheal_amount_once() {
    let mut group = native_group();
    assert!(own_aura(&group.h, group.priest, 368275).is_some());
    // Authored percent-health damage wounds both Henry and the recipient.
    apply_pressure(&mut group, 1);
    let priest_health = group.h.health(group.priest);
    let owner = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 2061, owner);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    let source = events
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(group.priest.to_bits())
                && event.target == Some(owner.to_bits())
                && event.spell_id == Some(2061)
        })
        .expect("ordinary Flash Heal must land on the other patient");
    assert!(source.amount > 0);
    assert!(
        source.overflow > 0,
        "native heal must cross the overheal boundary: {source:?}"
    );
    let children: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(group.priest.to_bits())
                && event.spell_id == Some(368276)
        })
        .collect();
    assert_eq!(
        children.len(),
        1,
        "Binding Heals must execute one native child: {events:?}"
    );
    let child = children[0];
    assert_eq!(child.target, Some(group.priest.to_bits()));
    assert_eq!(
        child.amount,
        source.amount / 5,
        "TC GetHeal, not GetEffectiveHeal"
    );
    assert!(!child.periodic);
    assert!(group.h.health(group.priest) > priest_health);
}

#[test]
fn holy_binding_heals_native90_self_heal_does_not_transfer() {
    let mut group = native_group();
    apply_pressure(&mut group, 2);
    let priest = group.priest;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 2061, priest);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(2061))
    );
    assert!(!events.iter().any(|event| event.spell_id == Some(368276)));
}

#[test]
fn holy_binding_heals_native90_serenity_outside_source_family_mask_does_not_transfer() {
    let mut group = native_group();
    apply_pressure(&mut group, 2);
    let owner = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 2050, owner);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(2050))
    );
    assert!(!events.iter().any(|event| event.spell_id == Some(368276)));
}

#[test]
fn holy_binding_heals_native29_unlearned_talent_does_not_transfer() {
    let mut group = native_group_at(29);
    assert!(own_aura(&group.h, group.priest, 368275).is_none());
    apply_pressure(&mut group, 2);
    let owner = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 2061, owner);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(2061))
    );
    assert!(!events.iter().any(|event| event.spell_id == Some(368276)));
}

#[test]
fn holy_binding_heals_native90_renew_periodic_mask_does_not_transfer() {
    let mut group = native_group();
    apply_pressure(&mut group, 2);
    let owner = group.h.caster;
    let start = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .len();
    cast_binding_source(&mut group, 139, owner);
    let events = &group.h.app.world().resource::<ObservedCombatEvents>().0[start..];
    assert!(events.iter().any(|event| event.kind == CombatLogKind::Heal
        && event.spell_id == Some(139)
        && event.periodic));
    assert!(!events.iter().any(|event| event.spell_id == Some(368276)));
}

#[test]
fn holy_policy_native90_apotheosis_uses_native_word_discount_not_free_words() {
    let mut group = native_group();
    let priest = group.priest;
    let paid = mana_cost(&group.h, priest, 2050);
    complete_real_cast(&mut group.h, priest, 200183);
    let discounted = mana_cost(&group.h, priest, 2050);
    assert!(discounted > 0.0);
    let expected = paid * 0.30;
    assert!(
        (discounted - expected).abs() < 0.001,
        "native70% discount, not a free Word"
    );
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

fn cast_native_bite(group: &mut GroupFixture, target: Entity) {
    // Use a new native dummy if the previous one died.
    if !crate::spell_cast::targets::is_alive(group.h.app.world(), group.h.npc) {
        let enemy = crate::spell_cast::test_harness::spawn_npc(&mut group.h.app, 3.0);
        group
            .h
            .app
            .world_mut()
            .entity_mut(enemy)
            .insert((Level(90), UnitLevel(90)));
        group.h.npc = enemy;
    }
    let enemy = group.h.npc;
    creature_cast(group.h.app.world_mut(), enemy, 123483, Some(target), false).unwrap();
    group.h.advance(3);
}

#[test]
fn holy_policy_native90_guardian_rescues_an_actual_npc_once() {
    let mut group = native_group();
    apply_pressure(&mut group, 9);
    let protected = group.members[2];
    creature_cast(
        group.h.app.world_mut(),
        group.priest,
        47788,
        Some(protected),
        false,
    )
    .unwrap();
    group.h.advance(3);
    assert!(has_aura(group.h.app.world(), protected, 47788));
    // Native Bite123483 is one100%-max-health physical hit, no power/cast time.
    // Ordinary native avoidance/armor remain intact; no hit or pool is replaced.
    for _ in 0..20 {
        cast_native_bite(&mut group, protected);
        if !has_aura(group.h.app.world(), protected, 47788) {
            break;
        }
    }
    assert!(!has_aura(group.h.app.world(), protected, 47788));
    assert!(fraction(&group, protected) > 0.0);
    assert!(
        group
            .h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.spell_id == Some(48153)
                    && event.kind == CombatLogKind::Heal
                    && event.target == Some(protected.to_bits())
                    && event.amount > event.overflow
            })
    );
    for _ in 0..20 {
        cast_native_bite(&mut group, protected);
        if fraction(&group, protected) == 0.0 {
            return;
        }
    }
    panic!("the same native hits must kill after the one-shot guardian is consumed");
}
