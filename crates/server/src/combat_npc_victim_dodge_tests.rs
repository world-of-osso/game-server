//! TC a352b1fa Unit.cpp2761–2762: native NPC victim49 before attacker251/240.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType, CombatLogEvent, CombatLogKind, MissKind};

use super::facing_tests::collect_swings;
use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const EVASION: u32 = 5_277;
const NEGATIVE_DODGE: u32 = 62_356;
const NPC_HIT_BUFFS: u32 = 62_627;
const DUMMY: u32 = 31_144;
const SWINGS: usize = 5;

fn fixture(front: bool) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[EVASION, NEGATIVE_DODGE, NPC_HIT_BUFFS], 1).unwrap();
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
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let orientation = if front { std::f32::consts::PI } else { 0.0 };
    let victim = crate::zones::summon_creature(
        h.app.world_mut(),
        caster,
        DUMMY,
        [3.0, 0.0, 0.0, orientation],
    )
    .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    // Existing native NO_PARRY/NO_BLOCK masks must not hide baseline outcomes.
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(victim)
            .unwrap()
            .0
            & 0x14,
        0
    );
    (h, victim)
}

fn dodge_modifier(h: &Harness, actor: Entity) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(49))
}

fn cast_evasion(h: &mut Harness, actor: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, EVASION, Some(actor), true).unwrap();
    h.advance(3);
    assert_eq!(dodge_modifier(h, actor), 200.0);
}

fn apply_negative_dodge(h: &mut Harness, actor: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: NEGATIVE_DODGE,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert_eq!(dodge_modifier(h, actor), 0.0);
    // Native62356 effect0 is block51=-200: it must not enter this fixture.
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .modifier(51),
        0.0
    );
}

fn assert_signed_auras_live(h: &Harness, actor: Entity) {
    assert!(crate::auras::has_aura(h.app.world(), actor, EVASION));
    assert!(crate::auras::has_aura(h.app.world(), actor, NEGATIVE_DODGE));
    assert_eq!(dodge_modifier(h, actor), 0.0);
}

fn assert_native_damage(events: &[CombatLogEvent]) {
    // Equal80 ordinary baseline: P(five Miss/Dodge/Parry)=.14^5≈5.378e-5.
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.amount > 0),
        "{events:?}"
    );
}

fn assert_live_npc_evasion(front: bool) {
    let (mut h, victim) = fixture(front);
    let attacker = h.caster;
    cast_evasion(&mut h, victim);
    let before = h.health(victim);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert!(crate::auras::has_aura(h.app.world(), victim, EVASION));
    assert_eq!(dodge_modifier(&h, victim), 200.0);
    assert!(
        events.iter().all(|e| matches!(
            e.kind,
            CombatLogKind::Miss(MissKind::Miss | MissKind::Dodge)
        )),
        "{events:?}"
    );
    // Five native2s swings fit10s; P(allMiss)=.05^5=3.125e-7.
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Miss(MissKind::Dodge)),
        "{events:?}"
    );
    assert_eq!(h.health(victim), before);
}

#[test]
fn native_melee_npc_victim_dodge_signed_coexistence_is_runtime_qualified() {
    let (mut h, victim) = fixture(true);
    cast_evasion(&mut h, victim);
    apply_negative_dodge(&mut h, victim);
    let start = h.app.world().resource::<Time>().elapsed();
    // Qualify native+200/-200 coexistence before any signed behavior test.
    // No refresh, altered clock or duration; amounts come from real rows.
    h.advance(160);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed >= std::time::Duration::from_secs(8));
    assert_signed_auras_live(&h, victim);
    eprintln!(
        "native5277+62356effect1 summed49=0 after{}s; block51 remains0",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_victim_dodge_live_front_evasion_fills_the_table() {
    assert_live_npc_evasion(true);
}

#[test]
fn native_melee_npc_victim_dodge_live_behind_evasion_still_dodges() {
    assert_live_npc_evasion(false);
}

#[test]
fn native_melee_npc_victim_dodge_cancellation_and_expiry_restore_baseline() {
    for expire in [false, true] {
        let (mut h, victim) = fixture(true);
        let attacker = h.caster;
        cast_evasion(&mut h, victim);
        if expire {
            let duration = h
                .app
                .world()
                .get::<ActiveAuras>(victim)
                .unwrap()
                .iter()
                .find(|a| a.spell_id == EVASION)
                .unwrap()
                .duration_ms;
            let dt = h.app.world().resource::<Time>().delta_secs();
            h.advance((duration as f32 / 1000.0 / dt).ceil() as usize + 3);
        } else {
            crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |a| {
                a.spell_id == EVASION
            });
            h.advance(3);
        }
        assert!(!crate::auras::has_aura(h.app.world(), victim, EVASION));
        assert_eq!(dodge_modifier(&h, victim), 0.0);
        let events = collect_swings(&mut h, attacker, victim, SWINGS);
        assert_native_damage(&events);
    }
}

#[test]
fn native_melee_npc_victim_dodge_foreign_evasion_does_not_modify_target() {
    let (mut h, victim) = fixture(true);
    let (attacker, foreign) = (h.caster, h.npc);
    cast_evasion(&mut h, foreign);
    assert_eq!(dodge_modifier(&h, victim), 0.0);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_native_damage(&events);
    assert!(crate::auras::has_aura(h.app.world(), foreign, EVASION));
}

#[test]
fn native_melee_npc_victim_dodge_signed_native_sum_preserves_baseline() {
    let (mut h, victim) = fixture(true);
    let attacker = h.caster;
    cast_evasion(&mut h, victim);
    apply_negative_dodge(&mut h, victim);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_signed_auras_live(&h, victim);
    assert_native_damage(&events);
}

#[test]
fn native_melee_npc_victim_dodge_attacker_expertise_still_wins() {
    let (mut h, victim) = fixture(true);
    let attacker = h.npc;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        attacker,
        NPC_HIT_BUFFS,
        Some(attacker),
        true,
    )
    .unwrap();
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(attacker)
            .unwrap()
            .modifier(240),
        10_000.0
    );
    cast_evasion(&mut h, victim);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert!(crate::auras::has_aura(h.app.world(), victim, EVASION));
    assert_eq!(dodge_modifier(&h, victim), 200.0);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit | CombatEventType::Block
        ) && e.amount > 0.0),
        "{events:?}"
    );
}
