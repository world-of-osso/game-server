//! TC a352b1fa Unit.cpp2851–2861: NPC victim51 block chance, not240 reduction.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const BLOCK: u32 = 10_021;
const NEGATIVE_BLOCK: u32 = 62_356;
const NPC_HIT_BUFFS: u32 = 62_627;
const DUMMY: u32 = 31_144;
const ICE_TOMB: u32 = 36_980;
const SWINGS: usize = 5;

fn fixture(entry: u32, front: bool) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[BLOCK, NEGATIVE_BLOCK, NPC_HIT_BUFFS], 1).unwrap();
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
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let orientation = if front { std::f32::consts::PI } else { 0.0 };
    let victim = crate::zones::summon_creature(
        h.app.world_mut(),
        caster,
        entry,
        [3.0, 0.0, 0.0, orientation],
    )
    .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    for kind in [47, 49, 51] {
        assert_eq!(modifier(&h, victim, kind), 0.0);
    }
    let attacker = h.npc;
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        attacker,
        NPC_HIT_BUFFS,
        Some(attacker),
        true,
    )
    .unwrap();
    h.advance(3);
    assert_attacker_fold(&h);
    (h, victim)
}

fn modifier(h: &Harness, actor: Entity, kind: u32) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(kind))
}

fn assert_attacker_fold(h: &Harness) {
    assert!(crate::auras::has_aura(h.app.world(), h.npc, NPC_HIT_BUFFS));
    assert_eq!(modifier(h, h.npc, 240), 10_000.0);
    assert_eq!(modifier(h, h.npc, 54), 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.npc)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
    assert_eq!(
        super::creature_miss_chance(h.app.world().get::<crate::auras::AuraStatMods>(h.npc)),
        0.0
    );
}

fn apply_block(h: &mut Harness, actor: Entity, spell_id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    // 62356 effect0 is block51; effect1 is dodge49 and must not enter this slice.
    assert_eq!(modifier(h, actor, 49), 0.0);
}

fn assert_unblocked(events: &[CombatEvent]) {
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
}

fn assert_baseline_unblocked(events: &[CombatEvent]) {
    // Native62627 removes miss/dodge/parry, not block. P(all5Block)=.03^5.
    assert!(
        events.iter().any(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit
        ) && e.amount > 0.0),
        "{events:?}"
    );
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_signed_coexistence_and_fold_are_qualified() {
    let (mut h, victim) = fixture(DUMMY, true);
    apply_block(&mut h, victim, BLOCK);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    apply_block(&mut h, victim, NEGATIVE_BLOCK);
    assert_eq!(modifier(&h, victim, 51), -100.0);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(160);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed >= std::time::Duration::from_secs(8));
    for spell in [BLOCK, NEGATIVE_BLOCK] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
    assert_eq!(modifier(&h, victim, 51), -100.0);
    assert_eq!(modifier(&h, victim, 49), 0.0);
    assert_attacker_fold(&h);
    eprintln!(
        "native10021+62356effect0 sum51=-100 after{}s,49=0; NPC62627hit100/expertise10000 folded",
        elapsed.as_secs_f64()
    );
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_positive_fills_table_despite_expertise() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.npc;
    apply_block(&mut h, victim, BLOCK);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    assert_attacker_fold(&h);
    // Block103 saturates after actual native miss/dodge/parry0; expertise
    // must NOT reduce block. Old3% escapes only .03^5=2.43e-8.
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::Block && e.amount > 0.0),
        "{events:?}"
    );
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_cancelled_aura_restores_baseline() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.npc;
    apply_block(&mut h, victim, BLOCK);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |a| {
        a.spell_id == BLOCK
    });
    h.advance(3);
    assert_eq!(modifier(&h, victim, 51), 0.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_baseline_unblocked(&events);
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_foreign_aura_does_not_modify_target() {
    let (mut h, victim) = fixture(DUMMY, true);
    let (attacker, foreign) = (h.npc, h.caster);
    apply_block(&mut h, foreign, BLOCK);
    assert_eq!(modifier(&h, foreign, 51), 100.0);
    assert_eq!(modifier(&h, victim, 51), 0.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_baseline_unblocked(&events);
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_signed_native_sum_clamps_at_zero() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.npc;
    apply_block(&mut h, victim, BLOCK);
    apply_block(&mut h, victim, NEGATIVE_BLOCK);
    assert_eq!(modifier(&h, victim, 51), -100.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    for spell in [BLOCK, NEGATIVE_BLOCK] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
    assert_eq!(modifier(&h, victim, 49), 0.0);
    assert_unblocked(&events);
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_negative_native_amount_clamps_at_zero() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.npc;
    apply_block(&mut h, victim, NEGATIVE_BLOCK);
    assert_eq!(modifier(&h, victim, 51), -200.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(modifier(&h, victim, 49), 0.0);
    assert_unblocked(&events);
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_positive_does_not_override_behind_arc() {
    let (mut h, victim) = fixture(DUMMY, false);
    let attacker = h.npc;
    apply_block(&mut h, victim, BLOCK);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    assert_unblocked(&events);
}

#[test]
#[ignore]
fn native_melee_npc_victim_block_positive_does_not_override_native_no_block() {
    let (mut h, victim) = fixture(ICE_TOMB, true);
    let attacker = h.npc;
    assert_ne!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(victim)
            .unwrap()
            .0
            & 0x10,
        0
    );
    apply_block(&mut h, victim, BLOCK);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(modifier(&h, victim, 51), 100.0);
    assert_unblocked(&events);
}
