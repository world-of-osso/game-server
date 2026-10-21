//! TC a352b1fa Unit.cpp2906–2909 / Unit.h804,809: victim183 applies AT/ABOVE threshold.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Health, Level, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const HEALTH_CRIT: u32 = 372_261;
const NPC_BUFFS: u32 = 62_627;
const WINDOW_SWINGS: usize = 20;
const BOUNDARY_SWINGS: usize = 400;

fn fixture() -> (Harness, Entity, Entity) {
    let mut h = harness(Vec::new(), &[HEALTH_CRIT, NPC_BUFFS], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    // Only level metadata is fixture input; NEVER write current/max Health.
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(80));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let owner = h.caster;
    let attacker =
        crate::zones::summon_creature(h.app.world_mut(), owner, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(attacker)
            .unwrap()
            .0,
        0x20000
    );
    let victim = h.npc; // Original native-backed harness NPC, NOT an immortal TrainingDummy.
    assert_eq!(health(&h, victim), (5000.0, 5000.0));
    assert!(
        h.app
            .world()
            .get::<crate::creature_scripts::TrainingDummy>(victim)
            .is_none()
    );
    apply(&mut h, attacker, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_eq!(modifier(&h, attacker, 54), 100.0);
    assert_eq!(modifier(&h, attacker, 240), 10_000.0);
    assert_eq!(modifier(&h, attacker, 52), 0.0);
    (h, attacker, victim)
}

fn health(h: &Harness, actor: Entity) -> (f32, f32) {
    let value = h.app.world().get::<Health>(actor).unwrap();
    (value.current, value.max)
}

fn modifier(h: &Harness, actor: Entity, kind: u32) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(kind))
}

fn apply(h: &mut Harness, actor: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), actor, spell_id));
}

fn assert_native_health_aura(h: &Harness, victim: Entity) {
    let auras = h.app.world().get::<ActiveAuras>(victim).unwrap();
    let aura = auras.iter().find(|a| a.spell_id == HEALTH_CRIT).unwrap();
    let effect = aura.effects.iter().find(|e| e.aura_type == 183).unwrap();
    assert_eq!(effect.amount, 50.0);
    assert_eq!(effect.misc_values[1], 20);
    assert_eq!(aura.duration_ms, 0);
    assert!(aura.expires_at.is_none());
    assert_eq!(modifier(h, victim, 187), 0.0);
    assert_eq!(modifier(h, victim, 197), 0.0);
}

fn swings(h: &mut Harness, attacker: Entity, victim: Entity, count: usize) -> Vec<CombatEvent> {
    let events = collect_broadcast_swings(h, attacker, victim, count);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && (e.amount == 10.0 || e.amount == 20.0)),
        "{events:?}"
    );
    assert!(health(h, victim).0 > 0.0);
    events
}

fn reach_native_boundary(h: &mut Harness, attacker: Entity, victim: Entity) {
    assert_eq!(modifier(h, victim, 183), 0.0);
    let events = swings(h, attacker, victim, BOUNDARY_SWINGS);
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::MeleeDamage && e.amount == 10.0),
        "native path differs: {events:?}"
    );
    assert_eq!(
        health(h, victim),
        (1000.0, 5000.0),
        "native path differs; do NOT rewrite HP/damage/counts"
    );
}

fn read_native_crit_chance(h: &Harness, attacker: Entity, victim: Entity) -> f32 {
    let world = h.app.world();
    let source = super::Attacker {
        entity: attacker,
        position: world.get::<shared::components::Position>(attacker).unwrap(),
        mods: world.get::<crate::auras::AuraStatMods>(attacker),
        stats: world.get::<crate::stats::CharacterStats>(attacker),
        haste_pct: world
            .get::<crate::stats::StatModifiers>(attacker)
            .map_or(0.0, |m| m.haste_pct),
        enemy_dodge_pct: modifier(h, attacker, 251),
        expertise_pct: modifier(h, attacker, 240) / 4.0,
        creature: world.get::<shared::components::Npc>(attacker).is_some(),
        authored_attack_time: world.get::<super::CreatureAttackTime>(attacker).copied(),
        creature_flags: world.get::<crate::zones::CreatureFlagsExtra>(attacker),
        temp_summoner: world
            .get::<crate::zones::TempSummon>(attacker)
            .map(|s| s.summoner),
        spirit_wolf: None,
    };
    let level_difference = i32::from(world.get::<UnitLevel>(victim).unwrap().0)
        - i32::from(world.get::<UnitLevel>(attacker).unwrap().0);
    super::read_outcome_chances(
        &source,
        crate::melee::Victim::Creature { level_difference },
        world.get::<crate::zones::CreatureFlagsExtra>(victim),
        true,
        false,
        world.get::<ActiveAuras>(victim),
        world.get::<Health>(victim).unwrap(),
    )
    .crit
}

fn assert_no_crit(events: &[CombatEvent]) {
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::MeleeDamage && e.amount == 10.0),
        "{events:?}"
    );
}

#[test]
fn native_melee_victim_health_crit_above_window_and_native_lifetime_are_qualified() {
    let (mut h, attacker, victim) = fixture();
    apply(&mut h, victim, HEALTH_CRIT, 1);
    assert_native_health_aura(&h, victim);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert_eq!(events.len(), WINDOW_SWINGS);
    assert!(health(&h, victim).0 >= 4600.0);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(16_000);
    assert!((h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() >= 800.0);
    assert_native_health_aura(&h, victim);
    assert_eq!(health(&h, victim).1, 5000.0);
}

#[test]
fn native_melee_victim_health_crit_actual_damage_boundary_is_qualified() {
    let (mut h, attacker, victim) = fixture();
    reach_native_boundary(&mut h, attacker, victim);
    apply(&mut h, victim, HEALTH_CRIT, 1);
    assert_native_health_aura(&h, victim);
    assert_eq!(health(&h, victim), (1000.0, 5000.0));
    let _ = swings(&mut h, attacker, victim, 1);
    let after = health(&h, victim).0;
    assert!(after == 980.0 || after == 990.0);
    let _ = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert!(health(&h, victim).0 < 1000.0);
    assert_native_health_aura(&h, victim);
    eprintln!(
        "native5000→1000 by400ordinary10damage events, then realdamage BELOW20%; no HP writes"
    );
}

#[test]
fn native_melee_victim_health_crit_above_twenty_percent_grants_crit() {
    let (mut h, attacker, victim) = fixture();
    apply(&mut h, victim, HEALTH_CRIT, 1);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert!(health(&h, victim).0 >= 4600.0);
    assert!(
        events
            .iter()
            .any(|e| e.event_type == CombatEventType::CriticalHit),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == CombatEventType::MeleeDamage),
        "{events:?}"
    );
}

#[test]
fn native_melee_victim_health_crit_inclusive_boundary_chance_uses_actual_health() {
    let (mut h, attacker, victim) = fixture();
    reach_native_boundary(&mut h, attacker, victim);
    apply(&mut h, victim, HEALTH_CRIT, 1);
    assert_eq!(health(&h, victim), (1000.0, 5000.0));
    // Pure numeric chance proof, paired with wire above/below tests, NOT twenty boundary rolls.
    assert_eq!(read_native_crit_chance(&h, attacker, victim), 50.0);
    let _ = swings(&mut h, attacker, victim, 1);
    assert!(health(&h, victim).0 < 1000.0);
    assert_eq!(read_native_crit_chance(&h, attacker, victim), 0.0);
}

#[test]
fn native_melee_victim_health_crit_below_twenty_percent_does_not_grant_crit() {
    let (mut h, attacker, victim) = fixture();
    reach_native_boundary(&mut h, attacker, victim);
    let _ = swings(&mut h, attacker, victim, 1);
    assert_eq!(health(&h, victim), (990.0, 5000.0));
    apply(&mut h, victim, HEALTH_CRIT, 1);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert!(health(&h, victim).0 < 1000.0);
    assert_no_crit(&events);
}

#[test]
fn native_melee_victim_health_crit_attacker_only_aura_does_not_grant_crit() {
    let (mut h, attacker, victim) = fixture();
    apply(&mut h, attacker, HEALTH_CRIT, 1);
    assert_eq!(modifier(&h, victim, 183), 0.0);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert_no_crit(&events);
}

#[test]
fn native_melee_victim_health_crit_foreign_recipient_does_not_grant_crit() {
    let (mut h, attacker, victim) = fixture();
    let foreign = h.caster;
    apply(&mut h, foreign, HEALTH_CRIT, 1);
    assert_eq!(modifier(&h, victim, 183), 0.0);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert_no_crit(&events);
}

#[test]
fn native_melee_victim_health_crit_cancelled_aura_does_not_grant_crit() {
    let (mut h, attacker, victim) = fixture();
    apply(&mut h, victim, HEALTH_CRIT, 1);
    crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |a| {
        a.spell_id == HEALTH_CRIT
    });
    h.advance(3);
    assert_eq!(modifier(&h, victim, 183), 0.0);
    let events = swings(&mut h, attacker, victim, WINDOW_SWINGS);
    assert_no_crit(&events);
}
