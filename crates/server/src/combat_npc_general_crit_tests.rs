//! TC a352b1fa Unit.cpp2889–2890: NPC source crit includes both52 and290.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const NPC_BUFFS: u32 = 62_627;
const POSITIVE: u32 = 235_027;
const NEGATIVE: u32 = 426_659;
const AUTO_CRIT: u32 = 277_903;
const HIT_EXPERTISE_MASK: u32 = (1 << 1) | (1 << 2);
const SWINGS: usize = 256;

fn fixture(npc_buff_mask: u32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[NPC_BUFFS, POSITIVE, NEGATIVE, AUTO_CRIT], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.npc).insert((
        UnitLevel(80),
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<shared::protocol::CombatEvent>::default(),
        MessageReceiver::<shared::protocol::CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    // Behind native equal-level31144: no parry/block, but native dodge still needs240.
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(h.npc)
            .unwrap()
            .0,
        2
    );
    assert_eq!(modifier(&h, victim, 187), 0.0);
    let npc = h.npc;
    apply(&mut h, npc, NPC_BUFFS, npc_buff_mask);
    assert_eq!(
        modifier(&h, npc, 52),
        if npc_buff_mask & 1 == 0 { 0.0 } else { -100.0 }
    );
    assert_eq!(modifier(&h, npc, 290), 0.0);
    assert_hit(&h);
    (h, victim)
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

fn assert_hit(h: &Harness) {
    assert_eq!(modifier(h, h.npc, 54), 100.0);
    assert_eq!(modifier(h, h.npc, 240), 10_000.0);
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.npc)
        .unwrap();
    assert_eq!(mods.melee_hit_chance_pct, 100.0);
    assert_eq!(super::creature_miss_chance(Some(mods)), 0.0);
}

fn cancel(h: &mut Harness, actor: Entity, spell_id: u32) {
    crate::auras::remove_auras(h.app.world_mut(), actor, RemoveReason::Cancelled, |a| {
        a.spell_id == spell_id
    });
    h.advance(3);
    assert!(!crate::auras::has_aura(h.app.world(), actor, spell_id));
}

fn swings(h: &mut Harness, victim: Entity, count: usize) -> Vec<CombatEvent> {
    let npc = h.npc;
    let events = collect_broadcast_swings(h, npc, victim, count);
    assert_hit(h);
    assert_eq!(modifier(h, victim, 187), 0.0);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
    events
}

fn assert_no_crit(events: &[CombatEvent]) {
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::MeleeDamage),
        "{events:?}"
    );
}

fn assert_both(events: &[CombatEvent]) {
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

fn qualify_lifetime(h: &mut Harness, spells: &[u32]) {
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in spells {
        assert!(crate::auras::has_aura(h.app.world(), h.npc, *spell));
    }
    assert_hit(h);
    eprintln!(
        "native290 qualification{}s without refresh, HP/timer/RNG edits",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_general_crit_signed_inputs_are_runtime_qualified() {
    let (mut h, _) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1 << 1);
    apply(&mut h, npc, POSITIVE, 1); // Positive LAST: raw290 must sum to-100.
    apply(&mut h, npc, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    qualify_lifetime(&mut h, &[NPC_BUFFS, NEGATIVE, POSITIVE, AUTO_CRIT]);
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(npc)
            .unwrap()
            .crit_pct,
        -100.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(npc)
            .unwrap()
            .autoattack_crit_pct,
        50.0
    );
}

#[test]
fn native_melee_npc_general_crit_weapon_combination_inputs_are_runtime_qualified() {
    let (mut h, _) = fixture(7);
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    qualify_lifetime(&mut h, &[NPC_BUFFS, POSITIVE]);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(npc)
            .unwrap()
            .crit_pct,
        100.0
    );
}

#[test]
fn native_melee_npc_general_crit_positive_saturates_table() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    let events = swings(&mut h, victim, 5);
    // Source105 saturates; old5% escapes only .05^5=3.125e-7.
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit),
        "{events:?}"
    );
}

#[test]
fn native_melee_npc_general_crit_negative_prevents_crit() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1 << 1);
    assert_eq!(modifier(&h, npc, 290), -200.0);
    let events = swings(&mut h, victim, SWINGS);
    // Old5% table's all-normal escape is .95^256≈1.983e-6.
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_general_crit_signed_sum_not_last_effect_or_clipped_effects() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1 << 1);
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    let events = swings(&mut h, victim, SWINGS);
    assert_eq!(modifier(&h, npc, 290), -100.0);
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_general_crit_cancelled_positive_restores_baseline() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    cancel(&mut h, npc, POSITIVE);
    assert_eq!(modifier(&h, npc, 290), 0.0);
    let events = swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
fn native_melee_npc_general_crit_foreign_positive_does_not_change_attacker() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    apply(&mut h, victim, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 290), 100.0);
    assert_eq!(modifier(&h, h.npc, 290), 0.0);
    let events = swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
fn native_melee_npc_general_crit_without_aura_keeps_five_percent_baseline() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    assert_eq!(modifier(&h, h.npc, 290), 0.0);
    let events = swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
fn native_melee_npc_general_crit_meets_weapon_52_before_source_clamp() {
    let (mut h, victim) = fixture(7);
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 290), 100.0);
    let events = swings(&mut h, victim, SWINGS);
    // Correct raw5-100+100 gives5%; old52-only gives0 and early52clamp gives100%.
    assert_both(&events);
}

#[test]
fn native_melee_npc_general_crit_clamps_source_before_autoattack_334() {
    let (mut h, victim) = fixture(HIT_EXPERTISE_MASK);
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1 << 1);
    apply(&mut h, npc, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, npc, 290), -200.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    let events = swings(&mut h, victim, SWINGS);
    assert_both(&events);
}
