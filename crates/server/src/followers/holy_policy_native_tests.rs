//! Real 70448 percentage damage on a native level90 group, not a trivial Stockade overlevel.
use super::kit_clearcasting_tests::{complete_real_cast, frame_seconds, mana_cost, own_aura};
use super::kit_tests::{at_level, uses_spell};
use crate::auras::{ActiveAuras, has_aura};
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;
use crate::spell_cast::creature_cast;
use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents};
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{
    CombatRatings, DerivedStats, Health, Level, Player, Position, UnitLevel, UnitStats,
};
use shared::group::GroupRole;
use shared::protocol::CombatLogKind;

const HENRY: u32 = 215011;
/// Native165/10% max-health, source-centered enemy area; no fabricated spell row.
const BURST_OF_HATRED: u32 = 124257;
/// Native NPC Shadow Word: Pain has fixed72/3s and no power cost.
/// Unlike17146 (95mana), this source spell is valid for the no-mana dummy.
const MAGIC_DOT: u32 = 23268;
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
        view.insert(member, follower.online(unit, level, 1));
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
fn holy_policy_native90_earned_surge_precedes_paid_heal_and_mending() {
    let mut group = native_group();
    let priest = group.priest;
    let chance = 0.08_f64;
    let bound = (1e-6_f64.ln() / (-chance).ln_1p()).ceil() as usize;
    for _ in 0..bound {
        complete_real_cast(&mut group.h, priest, 2061);
        if own_aura(&group.h, priest, 114255).is_some() {
            break;
        }
        group.h.advance(300);
    }
    assert!(own_aura(&group.h, priest, 114255).is_some());
    apply_pressure(&mut group, 2);
    assert!(own_aura(&group.h, priest, 114255).is_some());
    let start = group.h.inbox.go.len();
    super::update_ai(group.h.app.world_mut());
    assert!(group.h.app.world().get::<CastState>(priest).is_none());
    group.h.advance(3);
    let action = group.h.inbox.go[start..]
        .iter()
        .find(|event| event.caster == priest.to_bits())
        .unwrap();
    assert_eq!(
        action.spell_id, 2061,
        "earned instant Surge wins routine priority"
    );
}

#[test]
fn holy_policy_native32_sanctify_effectively_heals_the_native_cluster() {
    let mut group = native_group_at(32);
    apply_pressure(&mut group, 4);
    uses_spell(&mut group.h, group.priest, 34861);
    group.h.advance(5);
    let healed: std::collections::BTreeSet<_> = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(group.priest.to_bits())
                && event.spell_id == Some(34861)
                && event.kind == CombatLogKind::Heal
                && event.amount > event.overflow
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(
        healed.len() >= 3,
        "native destination heal must reach real patients"
    );
}

#[test]
fn holy_policy_native32_spread_patients_do_not_spend_sanctify() {
    let mut group = native_group_at(32);
    apply_pressure(&mut group, 4);
    for (index, &member) in group.members.iter().enumerate() {
        let mut position = group.h.app.world_mut().get_mut::<Position>(member).unwrap();
        position.x = index as f32 * 40.0;
    }
    let start = casts(&group, 34861);
    for _ in 0..20 {
        super::update_ai(group.h.app.world_mut());
        group.h.advance(1);
    }
    assert_eq!(
        casts(&group, 34861),
        start,
        "global injury is not a native-radius cluster"
    );
}

fn engage_weave_enemies(group: &mut GroupFixture, count: usize) -> Vec<Entity> {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let source = crate::spell_cast::test_harness::load_native_creature_world(&[6]);
    let display_id = source.templates[&6].models[0].display_id;
    group
        .h
        .app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(source.templates);
    group
        .h
        .app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let mut enemies = vec![group.h.npc];
    for index in 1..count {
        enemies.push(crate::spell_cast::test_harness::spawn_npc(
            &mut group.h.app,
            index as f32,
        ));
    }
    let owner = group.h.caster;
    // Keep native melee actors outside owner swing range, inside Henry's Nova.
    // No health, power, cast chance or damage amount is substituted.
    group
        .h
        .app
        .world_mut()
        .get_mut::<Position>(owner)
        .unwrap()
        .x = -40.0;
    for &enemy in &enemies {
        group.h.app.world_mut().entity_mut(enemy).insert((
            Level(90),
            UnitLevel(90),
            shared::components::ModelDisplay { display_id },
            shared::components::ThreatTable::default(),
            crate::creature_ai::CreatureState::Idle,
        ));
        crate::creature_aggro::add_threat(group.h.app.world_mut(), enemy, owner, 1000.0);
    }
    enemies
}

fn step_henry(group: &mut GroupFixture) {
    let priest = group.priest;
    let follower = group
        .h
        .app
        .world()
        .get::<super::Follower>(priest)
        .unwrap()
        .clone();
    // Exercise the production controller without unrelated DPS killing the fixture.
    super::ai::update_follower(group.h.app.world_mut(), priest, &follower);
    group.h.advance(1);
}

fn wait_henry_cast(group: &mut GroupFixture, spell: u32) {
    let start = casts(group, spell);
    for _ in 0..240 {
        step_henry(group);
        if casts(group, spell) > start {
            group.h.advance(3);
            return;
        }
    }
    panic!("Henry never launched native offensive spell {spell}");
}

fn effectively_damaged(group: &GroupFixture, spell: u32, target: Entity) -> bool {
    group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .any(|event| {
            event.source == Some(group.priest.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
}

#[test]
fn holy_weave_native90_holy_fire_then_smite_deal_real_damage_when_safe() {
    let _rng = crate::random::seed_scope(257);
    let mut group = native_group();
    engage_weave_enemies(&mut group, 1);
    let first = group.h.npc;
    wait_henry_cast(&mut group, 14914);
    assert!(effectively_damaged(&group, 14914, first));
    let next = crate::spell_cast::test_harness::spawn_npc(&mut group.h.app, 3.0);
    group.h.npc = next;
    engage_weave_enemies(&mut group, 1);
    wait_henry_cast(&mut group, 585);
    assert!(effectively_damaged(&group, 585, next));
}

#[test]
fn holy_weave_native90_nova_reaches_three_real_engaged_enemies() {
    let _rng = crate::random::seed_scope(257);
    let mut group = native_group();
    let enemies = engage_weave_enemies(&mut group, 3);
    wait_henry_cast(&mut group, 132157);
    let outcomes: std::collections::BTreeSet<_> = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(group.priest.to_bits())
                && event.spell_id == Some(132157)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(
        enemies
            .iter()
            .all(|enemy| outcomes.contains(&enemy.to_bits())),
        "ordinary misses still prove actual native footprint selection"
    );
    assert!(
        enemies
            .iter()
            .any(|&enemy| effectively_damaged(&group, 132157, enemy))
    );
}

#[test]
fn holy_weave_native90_incoming_group_damage_prevents_offense() {
    let mut group = native_group();
    apply_pressure(&mut group, 6);
    engage_weave_enemies(&mut group, 1);
    let start = group.h.inbox.go.len();
    for _ in 0..20 {
        step_henry(&mut group);
    }
    let actions: Vec<_> = group.h.inbox.go[start..]
        .iter()
        .filter(|event| event.caster == group.priest.to_bits())
        .collect();
    assert!(
        !actions.is_empty(),
        "native healing controller must actually act"
    );
    assert!(
        actions
            .iter()
            .all(|event| !matches!(event.spell_id, 14914 | 585 | 132157))
    );
    assert!(
        group
            .members
            .iter()
            .any(|&member| fraction(&group, member) < 0.9)
    );
}

#[test]
fn holy_weave_native90_nova_does_not_recruit_a_fresh_nearby_enemy() {
    let _rng = crate::random::seed_scope(257);
    let mut group = native_group();
    engage_weave_enemies(&mut group, 3);
    let fresh = crate::spell_cast::test_harness::spawn_npc(&mut group.h.app, 2.5);
    let health = group.h.app.world().get::<Health>(fresh).unwrap().current;
    wait_henry_cast(&mut group, 14914);
    assert_eq!(casts(&group, 132157), 0);
    assert_eq!(
        group.h.app.world().get::<Health>(fresh).unwrap().current,
        health
    );
}

#[test]
fn holy_weave_native90_nova_preserves_real_polymorph() {
    let _rng = crate::random::seed_scope(257);
    let mut group = native_group();
    let enemies = engage_weave_enemies(&mut group, 4);
    let protected = enemies[3];
    let mage = group
        .members
        .iter()
        .copied()
        .find(|&member| {
            group
                .h
                .app
                .world()
                .get::<super::Follower>(member)
                .is_some_and(|follower| follower.profile.entry == 209059)
        })
        .unwrap();
    for _ in 0..10 {
        creature_cast(group.h.app.world_mut(), mage, 118, Some(protected), false).unwrap();
        group.h.advance(80);
        if has_aura(group.h.app.world(), protected, 118) {
            break;
        }
    }
    assert!(has_aura(group.h.app.world(), protected, 118));
    wait_henry_cast(&mut group, 14914);
    assert_eq!(casts(&group, 132157), 0);
    assert!(
        has_aura(group.h.app.world(), protected, 118),
        "native breakable CC must survive Henry's weave"
    );
}

#[test]
fn holy_policy_native90_full_health_does_not_spend_throughput_cooldowns() {
    let mut group = native_group();
    for _ in 0..40 {
        super::update_ai(group.h.app.world_mut());
        group.h.advance(1);
    }
    assert_eq!(casts(&group, 64843), 0);
    assert_eq!(casts(&group, 200183), 0);
    assert_eq!(casts(&group, 33076), 0);
    assert_eq!(casts(&group, 32375), 0);
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
fn holy_policy_native90_prayer_of_healing_covers_moderate_cluster_with_spent_words() {
    let mut group = native_group();
    let priest = group.priest;
    complete_real_cast(&mut group.h, priest, 200183);
    let expiry = own_aura(&group.h, priest, 200183)
        .unwrap()
        .expires_at
        .unwrap();
    let remaining = expiry - crate::auras::aura_now(group.h.app.world());
    let frames = (remaining / frame_seconds(&group.h)).ceil() as usize + 1;
    group.h.advance(frames);
    assert!(!has_aura(group.h.app.world(), priest, 200183));
    complete_real_cast(&mut group.h, priest, 2050);
    complete_real_cast(&mut group.h, priest, 2050);
    apply_pressure(&mut group, 4);
    uses_spell(&mut group.h, priest, 596);
    group.h.advance(5);
    let healed: std::collections::BTreeSet<_> = group
        .h
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(priest.to_bits())
                && event.spell_id == Some(596)
                && event.kind == CombatLogKind::Heal
                && event.amount > event.overflow
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(
        healed.len() >= 3,
        "native group heal must effectively cover its actual cluster"
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

fn apply_group_magic(group: &mut GroupFixture, targets: &[Entity]) {
    let enemy = group.h.npc;
    for &target in targets {
        creature_cast(
            group.h.app.world_mut(),
            enemy,
            MAGIC_DOT,
            Some(target),
            false,
        )
        .unwrap();
        group.h.advance(35);
    }
    group.h.advance(40);
    assert!(
        targets
            .iter()
            .all(|&target| has_aura(group.h.app.world(), target, MAGIC_DOT))
    );
}

#[test]
fn holy_policy_native90_mass_dispel_refuses_separated_magic_patients() {
    let mut group = native_group();
    let targets = group.members[2..].to_vec();
    apply_group_magic(&mut group, &targets);
    group
        .h
        .app
        .world_mut()
        .get_mut::<Position>(targets[2])
        .unwrap()
        .x = 80.0;
    for _ in 0..20 {
        super::update_ai(group.h.app.world_mut());
        group.h.advance(1);
    }
    assert_eq!(
        casts(&group, 32375),
        0,
        "three Magic auras are not one native-radius footprint"
    );
}

#[test]
fn holy_policy_native90_mass_dispel_clears_real_clustered_magic() {
    let mut group = native_group();
    let targets = group.members[2..].to_vec();
    apply_group_magic(&mut group, &targets);
    uses_spell(&mut group.h, group.priest, 32375);
    group.h.advance(5);
    assert!(
        targets
            .iter()
            .all(|&target| !has_aura(group.h.app.world(), target, MAGIC_DOT))
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
    let patient = super::healer_triage::read_patient(group.h.app.world(), target).unwrap();
    assert!(!patient.emergency && fraction(&group, target) > 0.95);
    let start = group.h.inbox.go.len();
    super::update_ai(group.h.app.world_mut());
    group.h.advance(5);
    let first = group.h.inbox.go[start..]
        .iter()
        .find(|event| event.caster == group.priest.to_bits())
        .map(|event| event.spell_id);
    assert_eq!(
        first,
        Some(527),
        "actual non-emergency Magic must be dispelled before a paid heal"
    );
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

fn cast_native_bite(group: &mut GroupFixture, target: Entity) {
    // The full party AI can naturally kill its damage dummy. Use the ordinary
    // next dummy, never replenish health or change the native damage spell.
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
    uses_spell(&mut group.h, group.priest, 47788);
    let protected = group
        .members
        .iter()
        .copied()
        .find(|&member| has_aura(group.h.app.world(), member, 47788))
        .unwrap();
    assert!(
        group
            .h
            .app
            .world()
            .get::<super::Follower>(protected)
            .is_some()
    );
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
fn holy_policy_native90_imminent_death_above_hp_threshold_receives_guardian_spirit() {
    let mut group = native_group();
    for _ in 0..7 {
        let enemy = group.h.npc;
        creature_cast(group.h.app.world_mut(), enemy, BURST_OF_HATRED, None, false).unwrap();
        group.h.advance(20);
    }
    let threatened = group.members.iter().any(|&member| {
        let patient = super::healer_triage::read_patient(group.h.app.world(), member).unwrap();
        fraction(&group, member) > 0.25 && patient.projected_death_seconds <= 3.0
    });
    assert!(
        threatened,
        "native fast damage must threaten death above the HP cutoff"
    );
    uses_spell(&mut group.h, group.priest, 47788);
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
