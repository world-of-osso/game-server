//! TC a352b1fa Unit.cpp2924/2926: victim197 joins187 BEFORE clamp and334.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, UnitLevel};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const POSITIVE: u32 = 291_576;
const NEGATIVE: u32 = 1_248_794;
const NPC_BUFFS: u32 = 62_627;
const AUTO_CRIT: u32 = 277_903;
const MELEE_REDUCTION: u32 = 460_575;
const SWINGS: usize = 256;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[POSITIVE, NEGATIVE, NPC_BUFFS, AUTO_CRIT, MELEE_REDUCTION],
        1,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.npc).insert((
        UnitLevel(80),
        shared::components::Position {
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
    // Targetx3faces+x: both actual NPC0 and player0 attack from behind, in range.
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let npc = h.npc;
    apply(&mut h, npc, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_eq!(modifier(&h, npc, 52), 0.0);
    assert_hit(&h, npc);
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

fn assert_hit(h: &Harness, npc: Entity) {
    assert_eq!(modifier(h, npc, 54), 100.0);
    assert_eq!(modifier(h, npc, 240), 10_000.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(npc)
            .unwrap()
            .melee_hit_chance_pct,
        100.0
    );
}

fn cancel(h: &mut Harness, actor: Entity, spell_id: u32) {
    crate::auras::remove_auras(h.app.world_mut(), actor, RemoveReason::Cancelled, |a| {
        a.spell_id == spell_id
    });
    h.advance(3);
    assert!(!crate::auras::has_aura(h.app.world(), actor, spell_id));
}

fn apply_full_source(h: &mut Harness) {
    let npc = h.npc;
    cancel(h, npc, NPC_BUFFS);
    apply(h, npc, NPC_BUFFS, 7);
    assert_eq!(modifier(h, npc, 52), -100.0);
    assert_hit(h, npc);
}

fn npc_swings(h: &mut Harness, victim: Entity, count: usize) -> Vec<CombatEvent> {
    let npc = h.npc;
    let events = collect_broadcast_swings(h, npc, victim, count);
    assert_hit(h, npc);
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

fn player_damage(h: &mut Harness, victim: Entity) -> Vec<CombatEvent> {
    let caster = h.caster;
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    let events = collect_broadcast_swings(h, caster, victim, SWINGS);
    // Ordinary player miss/dodge remain valid, not counted as landed normal hits.
    assert!(
        events.iter().all(|e| matches!(
            e.event_type,
            CombatEventType::MeleeDamage
                | CombatEventType::CriticalHit
                | CombatEventType::Miss
                | CombatEventType::Dodge
        )),
        "{events:?}"
    );
    let damage: Vec<_> = events.into_iter().filter(|e| e.amount > 0.0).collect();
    assert!(!damage.is_empty());
    damage
}

#[test]
fn native_melee_victim_general_crit_signed_coexistence_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, NEGATIVE, 1 << 1);
    apply(&mut h, victim, POSITIVE, 1); // Positive LAST, not last-effect-wins.
    let npc = h.npc;
    apply(&mut h, npc, AUTO_CRIT, 1);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    for spell in [POSITIVE, NEGATIVE] {
        assert!(crate::auras::has_aura(h.app.world(), victim, spell));
    }
    assert_eq!(modifier(&h, victim, 197), -1.0);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_eq!(modifier(&h, npc, 197), 0.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_hit(&h, npc);
    eprintln!(
        "native291576effect0+1248794effect1 victim197=-1 remains{}s;187/attacker197=0;33450",
        elapsed.as_secs_f64()
    );
}

#[test]
fn native_melee_victim_general_crit_other_modifiers_and_player_are_runtime_qualified() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    apply(&mut h, victim, MELEE_REDUCTION, 1 << 2);
    let npc = h.npc;
    // Full native62627 includes52=-100 as well as the qualified hit/expertise.
    crate::auras::remove_auras(
        h.app.world_mut(),
        npc,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == NPC_BUFFS,
    );
    apply(&mut h, npc, NPC_BUFFS, 7);
    let caster = h.caster;
    apply(&mut h, caster, AUTO_CRIT, 1);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    assert!(
        (h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() > SWINGS as f64 * 2.0
    );
    assert_eq!(modifier(&h, victim, 197), 100.0);
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_eq!(modifier(&h, caster, 197), 0.0);
    assert_eq!(modifier(&h, caster, 334), 50.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct,
        0.0
    );
    assert_hit(&h, npc);
}

#[test]
fn native_melee_victim_general_crit_no_crit_source_pair_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    let caster = h.caster;
    let darion =
        crate::zones::summon_creature(h.app.world_mut(), caster, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(darion)
            .unwrap()
            .0,
        0x20000
    );
    apply(&mut h, darion, NPC_BUFFS, (1 << 1) | (1 << 2));
    apply(&mut h, victim, POSITIVE, 1);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    assert!(
        (h.app.world().resource::<Time>().elapsed() - start).as_secs_f64() > SWINGS as f64 * 2.0
    );
    assert_eq!(modifier(&h, victim, 197), 100.0);
    assert_eq!(modifier(&h, darion, 197), 0.0);
    assert_eq!(modifier(&h, darion, 52), 0.0);
    assert_hit(&h, darion);
}

#[test]
#[ignore = "behavioral RED before victim197 consumer"]
fn native_melee_victim_general_crit_positive_saturates_received_chance() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 197), 100.0);
    let events = npc_swings(&mut h, victim, 5);
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit),
        "{events:?}"
    );
}

#[test]
#[ignore = "behavioral RED before victim197 consumer"]
fn native_melee_victim_general_crit_negative_prevents_received_crit() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, NEGATIVE, 1 << 1);
    assert_eq!(modifier(&h, victim, 197), -101.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_no_crit(&events);
}

#[test]
#[ignore = "behavioral control for signed victim197 sum"]
fn native_melee_victim_general_crit_signed_sum_with_negative_source_stays_zero() {
    let (mut h, victim) = fixture();
    apply_full_source(&mut h);
    apply(&mut h, victim, NEGATIVE, 1 << 1);
    apply(&mut h, victim, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 197), -1.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    // Source-95 + raw197(-1)=0 after clamp; last-positive/per-effect-clipping gives5%.
    assert_no_crit(&events);
}

#[test]
#[ignore = "behavioral control for victim197 cancellation"]
fn native_melee_victim_general_crit_cancelled_positive_restores_baseline() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    cancel(&mut h, victim, POSITIVE);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for victim-owned197"]
fn native_melee_victim_general_crit_foreign_positive_does_not_change_actual_victim() {
    let (mut h, victim) = fixture();
    let foreign = h.caster;
    apply(&mut h, foreign, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for victim-owned197"]
fn native_melee_victim_general_crit_attacker_positive_does_not_change_received_chance() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral RED before received197 pre-clamp source sum"]
fn native_melee_victim_general_crit_meets_source_52_before_zero_clamp() {
    let (mut h, victim) = fixture();
    apply_full_source(&mut h);
    apply(&mut h, victim, POSITIVE, 1);
    let events = npc_swings(&mut h, victim, SWINGS);
    // Source5-100 + victim100 gives5, not0 (missing197) or100 (earlysourceclamp).
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for landed pre334 clamp"]
fn native_melee_victim_general_crit_negative_clamps_before_attacker_334() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, NEGATIVE, 1 << 1);
    let npc = h.npc;
    apply(&mut h, npc, AUTO_CRIT, 1);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral RED before joint victim187 and197 sum"]
fn native_melee_victim_general_crit_meets_victim_187_before_zero_clamp() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, MELEE_REDUCTION, 1 << 2);
    apply(&mut h, victim, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 187), -60.0);
    assert_eq!(modifier(&h, victim, 197), 100.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral RED before player-source victim197 consumer"]
fn native_melee_victim_general_crit_player_source_uses_actual_victim() {
    let (mut h, victim) = fixture();
    apply(&mut h, victim, POSITIVE, 1);
    let damage = player_damage(&mut h, victim);
    assert!(
        damage
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit),
        "{damage:?}"
    );
}

#[test]
#[ignore = "behavioral control for player attacker-owned197 exclusion"]
fn native_melee_victim_general_crit_player_attacker_only_does_not_gain_crit() {
    let (mut h, victim) = fixture();
    let caster = h.caster;
    apply(&mut h, caster, POSITIVE, 1);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let damage = player_damage(&mut h, victim);
    assert_no_crit(&damage);
}

#[test]
#[ignore = "behavioral RED: source NO_CRIT must not exclude received197"]
fn native_melee_victim_general_crit_applies_after_native_source_no_crit() {
    let (mut h, victim) = fixture();
    let caster = h.caster;
    let darion =
        crate::zones::summon_creature(h.app.world_mut(), caster, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(darion)
            .unwrap()
            .0,
        0x20000
    );
    apply(&mut h, darion, NPC_BUFFS, (1 << 1) | (1 << 2));
    apply(&mut h, victim, POSITIVE, 1);
    let events = collect_broadcast_swings(&mut h, darion, victim, 5);
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit && e.amount > 0.0),
        "{events:?}"
    );
}
