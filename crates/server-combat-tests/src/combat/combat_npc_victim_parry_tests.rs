//! TC a352b1fa Unit.cpp2807–2808/2817–2822: NPC victim47 before attacker240.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType, CombatLogEvent, CombatLogKind, MissKind};

use super::facing_tests::collect_swings;
use super::npc_expertise_tests::collect_broadcast_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const PARRY: u32 = 23_547;
const NEGATIVE_PARRY: u32 = 64_651;
const SUNWELL: u32 = 45_769;
const NPC_HIT_BUFFS: u32 = 62_627;
const DUMMY: u32 = 31_144;
const ICE_TOMB: u32 = 36_980;
const SWINGS: usize = 5;

fn fixture(entry: u32, front: bool) -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[PARRY, NEGATIVE_PARRY, SUNWELL, NPC_HIT_BUFFS],
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
    apply_native(&mut h, caster, SUNWELL, 0b11);
    assert_sunwell_fold(&h);
    (h, victim)
}

fn apply_native(h: &mut Harness, actor: Entity, spell_id: u32, effect_mask: u32) {
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
}

fn assert_sunwell_fold(h: &Harness) {
    let auras = h.app.world().get::<ActiveAuras>(h.caster).unwrap();
    assert!(crate::auras::has_aura(h.app.world(), h.caster, SUNWELL));
    assert_eq!(auras.modifier(54), 5.0);
    assert_eq!(auras.modifier(251), -20.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.caster)
            .unwrap()
            .melee_hit_chance_pct,
        5.0
    );
}

fn parry_modifier(h: &Harness, actor: Entity) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(47))
}

fn cast_parry(h: &mut Harness, actor: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), actor, PARRY, Some(actor), true).unwrap();
    h.advance(3);
    assert_eq!(parry_modifier(h, actor), 100.0);
}

fn assert_damage(events: &[CombatLogEvent]) {
    // Actual Sunwell removes miss/dodge. P(five baselineParry)=.06^5=7.776e-7.
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.amount > 0),
        "{events:?}"
    );
}

fn assert_all_damage(events: &[CombatLogEvent]) {
    assert!(
        events
            .iter()
            .all(|e| e.kind == CombatLogKind::Damage && e.amount > 0),
        "{events:?}"
    );
}

#[test]
fn native_melee_npc_victim_parry_signed_fold_and_lifetime_are_qualified() {
    let (mut h, victim) = fixture(DUMMY, true);
    cast_parry(&mut h, victim);
    apply_native(&mut h, victim, NEGATIVE_PARRY, 1);
    assert_eq!(parry_modifier(&h, victim), 0.0);
    let start = h.app.world().resource::<Time>().elapsed();
    // One real application of each permanent native aura; no duration mutation.
    h.advance(160);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed >= std::time::Duration::from_secs(8));
    for spell in [PARRY, NEGATIVE_PARRY] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
    assert_eq!(parry_modifier(&h, victim), 0.0);
    assert_sunwell_fold(&h);
    eprintln!(
        "native23547+64651 sum47=0 after{}s; Sunwell hit5/dodge-20 folded",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_npc_victim_parry_front_positive_fills_the_table() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.caster;
    cast_parry(&mut h, victim);
    let before = h.health(victim);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(parry_modifier(&h, victim), 100.0);
    assert_sunwell_fold(&h);
    // Correct table is allParry106; old6% escapes only .06^5=7.776e-7.
    assert!(
        events
            .iter()
            .all(|e| e.kind == CombatLogKind::Miss(MissKind::Parry)),
        "{events:?}"
    );
    assert_eq!(h.health(victim), before);
}

#[test]
fn native_melee_npc_victim_parry_cancelled_aura_restores_baseline() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.caster;
    cast_parry(&mut h, victim);
    crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |a| {
        a.spell_id == PARRY
    });
    h.advance(3);
    assert_eq!(parry_modifier(&h, victim), 0.0);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_damage(&events);
}

#[test]
fn native_melee_npc_victim_parry_foreign_aura_does_not_modify_target() {
    let (mut h, victim) = fixture(DUMMY, true);
    let (attacker, foreign) = (h.caster, h.npc);
    cast_parry(&mut h, foreign);
    assert_eq!(parry_modifier(&h, victim), 0.0);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_damage(&events);
    assert_eq!(parry_modifier(&h, foreign), 100.0);
}

#[test]
fn native_melee_npc_victim_parry_signed_native_sum_preserves_baseline() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.caster;
    cast_parry(&mut h, victim);
    apply_native(&mut h, victim, NEGATIVE_PARRY, 1);
    assert_eq!(parry_modifier(&h, victim), 0.0);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_damage(&events);
    for spell in [PARRY, NEGATIVE_PARRY] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
}

#[test]
fn native_melee_npc_victim_parry_negative_native_amount_clamps_at_zero() {
    let (mut h, victim) = fixture(DUMMY, true);
    let attacker = h.caster;
    apply_native(&mut h, victim, NEGATIVE_PARRY, 1);
    assert_eq!(parry_modifier(&h, victim), -100.0);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_all_damage(&events);
}

#[test]
fn native_melee_npc_victim_parry_positive_does_not_override_behind_arc() {
    let (mut h, victim) = fixture(DUMMY, false);
    let attacker = h.caster;
    cast_parry(&mut h, victim);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(parry_modifier(&h, victim), 100.0);
    assert_all_damage(&events);
}

#[test]
fn native_melee_npc_victim_parry_positive_does_not_override_native_no_parry() {
    let (mut h, victim) = fixture(ICE_TOMB, true);
    let attacker = h.caster;
    assert_ne!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(victim)
            .unwrap()
            .0
            & 0x4,
        0
    );
    cast_parry(&mut h, victim);
    let events = collect_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(parry_modifier(&h, victim), 100.0);
    assert_all_damage(&events);
}

#[test]
fn native_melee_npc_victim_parry_attacker_expertise_still_wins() {
    let (mut h, victim) = fixture(DUMMY, true);
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
    cast_parry(&mut h, victim);
    let events = collect_broadcast_swings(&mut h, attacker, victim, SWINGS);
    assert_eq!(parry_modifier(&h, victim), 100.0);
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage | CombatEventType::CriticalHit | CombatEventType::Block
        ) && e.amount > 0.0),
        "{events:?}"
    );
}
