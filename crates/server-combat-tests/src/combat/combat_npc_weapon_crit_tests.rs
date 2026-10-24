//! TC a352b1fa Unit.cpp2885–2926/2387: NPC52 and pre334 victim187 clamp.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const NPC_BUFFS: u32 = 62_627;
const POSITIVE: u32 = 24_353;
const NEGATIVE: u32 = 64_481;
const AUTO_CRIT: u32 = 277_903;
const VICTIM_REDUCTION: u32 = 460_575;
const SWINGS: usize = 256;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[NPC_BUFFS, POSITIVE, NEGATIVE, AUTO_CRIT, VICTIM_REDUCTION],
        1,
    )
    .unwrap();
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
    // Both possible attackers stand behind this native dummy: no parry/block.
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
    assert_eq!(modifier(&h, h.npc, 290), 0.0);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    let attacker = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), attacker, NPC_BUFFS, Some(attacker), true)
        .unwrap();
    h.advance(3);
    assert_eq!(modifier(&h, attacker, 52), -100.0);
    assert_npc_hit(&h);
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

fn cancel(h: &mut Harness, actor: Entity, spell_id: u32) {
    crate::auras::remove_auras(h.app.world_mut(), actor, RemoveReason::Cancelled, |a| {
        a.spell_id == spell_id
    });
    h.advance(3);
    assert!(!crate::auras::has_aura(h.app.world(), actor, spell_id));
}

fn assert_npc_hit(h: &Harness) {
    assert_eq!(modifier(h, h.npc, 54), 100.0);
    assert_eq!(modifier(h, h.npc, 240), 10_000.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.npc)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
    assert_eq!(modifier(h, h.npc, 290), 0.0);
}

fn npc_swings(h: &mut Harness, victim: Entity, count: usize) -> Vec<CombatEvent> {
    let attacker = h.npc;
    let events = collect_broadcast_swings(h, attacker, victim, count);
    assert_npc_hit(h);
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

fn player_clamp_fixture() -> (Harness, Entity) {
    let (mut h, victim) = fixture();
    let caster = h.caster;
    // Existing player stat input stays native harness default0 crit, not aura52 support.
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    apply(&mut h, victim, VICTIM_REDUCTION, 1 << 2);
    apply(&mut h, caster, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, caster, 334), 50.0);
    assert_eq!(modifier(&h, caster, 52), 0.0);
    assert_eq!(modifier(&h, caster, 290), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(caster)
            .unwrap()
            .autoattack_crit_pct,
        50.0
    );
    (h, victim)
}

#[test]
fn native_melee_npc_weapon_crit_inputs_are_runtime_qualified() {
    let (mut h, _) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1);
    apply(&mut h, npc, POSITIVE, 1); // Positive LAST: summation, not last-wins.
    apply(&mut h, npc, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, npc, 52), -99.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in [NPC_BUFFS, NEGATIVE, POSITIVE, AUTO_CRIT] {
        assert!(crate::auras::has_aura(h.app.world(), npc, spell));
    }
    assert_eq!(modifier(&h, npc, 52), -99.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(npc)
            .unwrap()
            .autoattack_crit_pct,
        50.0
    );
    assert_npc_hit(&h);
    eprintln!(
        "native62627+64481+24353 raw52=-99;334=50/hit100/expertise10000 live{}s",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_weapon_crit_player_187_334_inputs_are_runtime_qualified() {
    let (mut h, victim) = player_clamp_fixture();
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    assert!(
        (h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() > SWINGS as f64 * 2.0
    );
    assert!(crate::auras::has_aura(
        h.app.world(),
        victim,
        VICTIM_REDUCTION
    ));
    assert!(crate::auras::has_aura(h.app.world(), h.caster, AUTO_CRIT));
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, h.caster, 334), 50.0);
}

#[test]
fn native_melee_npc_weapon_crit_negative_prevents_crit() {
    let (mut h, victim) = fixture();
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_weapon_crit_positive_saturates_table() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), 900.0);
    let events = npc_swings(&mut h, victim, 5);
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit),
        "{events:?}"
    );
}

#[test]
fn native_melee_npc_weapon_crit_signed_sum_not_last_effect_or_clipped_effects() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, NEGATIVE, 1);
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, npc, 52), -99.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_eq!(modifier(&h, npc, 52), -99.0);
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_weapon_crit_cancelled_positive_leaves_negative() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    cancel(&mut h, npc, POSITIVE);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_weapon_crit_foreign_positive_does_not_change_attacker() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 52), 1000.0);
    assert_eq!(modifier(&h, h.npc, 52), -100.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_no_crit(&events);
}

#[test]
fn native_melee_npc_weapon_crit_without_52_keeps_five_percent_baseline() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    cancel(&mut h, npc, NPC_BUFFS);
    // Reapply ONLY the native hit/expertise effects; no synthetic stats or flags.
    apply(&mut h, npc, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_eq!(modifier(&h, npc, 52), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
fn native_melee_npc_weapon_crit_clamps_source_before_autoattack_334() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, AUTO_CRIT, 1);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
fn native_melee_npc_weapon_crit_player_187_clamps_before_attacker_334() {
    let (mut h, victim) = player_clamp_fixture();
    let caster = h.caster;
    let events = collect_broadcast_swings(&mut h, caster, victim, SWINGS);
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, caster, 334), 50.0);
    // Native player0 + victim187(-60) clamps to0, THEN attacker334 adds50.
    // Old combined clamp gives0, so it cannot produce even one Crit.
    assert_both(&events);
}
