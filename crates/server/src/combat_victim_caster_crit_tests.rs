//! TC a352b1fa Unit.cpp2911–2914: victim306 filters actual aura caster identity.
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, UnitLevel};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use shared::protocol::{CombatEvent, CombatEventType};

use super::npc_expertise_tests::collect_broadcast_swings;
use crate::spell_cast::test_harness::{Harness, harness, world_db};

const CASTER_CRIT: u32 = 1_259_256;
const NPC_BUFFS: u32 = 62_627;
const NEGATIVE_RECEIVED: u32 = 1_248_794;
const AUTO_CRIT: u32 = 277_903;
const SWINGS: usize = 256;

fn fixture() -> (Harness, Entity) {
    let mut h = harness(
        Vec::new(),
        &[CASTER_CRIT, NPC_BUFFS, NEGATIVE_RECEIVED, AUTO_CRIT],
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
        MessageSender::<shared::protocol::CombatEvent>::default(),
        MessageReceiver::<shared::protocol::CombatEvent>::default(),
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(modifier(&h, victim, 306), 0.0);
    assert_eq!(modifier(&h, victim, 187), 0.0);
    assert_eq!(modifier(&h, victim, 197), 0.0);
    let npc = h.npc;
    apply(&mut h, npc, npc, NPC_BUFFS, (1 << 1) | (1 << 2));
    assert_hit(&h, npc);
    (h, victim)
}

fn modifier(h: &Harness, actor: Entity, kind: u32) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(kind))
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell_id: u32, effect_mask: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

fn assert_caster(h: &Harness, target: Entity, caster: Entity) {
    let auras = h.app.world().get::<ActiveAuras>(target).unwrap();
    let aura = auras.iter().find(|a| a.spell_id == CASTER_CRIT).unwrap();
    assert_eq!(aura.caster, caster);
    assert_eq!(auras.modifier(306), 100.0);
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

fn qualify_lifetime(h: &mut Harness, victim: Entity, aura_caster: Entity) {
    assert_caster(h, victim, aura_caster);
    let start = h.app.world().resource::<Time>().elapsed();
    h.advance(SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed() - start;
    assert!(elapsed.as_secs_f64() > SWINGS as f64 * 2.0);
    assert_caster(h, victim, aura_caster);
    eprintln!(
        "native1259256 victim306=100 actualcaster{:?} remains{}s",
        aura_caster,
        elapsed.as_secs_f64()
    );
}

fn apply_full_source(h: &mut Harness) {
    let npc = h.npc;
    crate::auras::remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |a| {
        a.spell_id == NPC_BUFFS
    });
    h.advance(3);
    apply(h, npc, npc, NPC_BUFFS, 7);
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

fn assert_all_crit(events: &[CombatEvent]) {
    assert!(
        events
            .iter()
            .all(|e| e.event_type == CombatEventType::CriticalHit),
        "{events:?}"
    );
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
    let player = h.caster;
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(player)
            .unwrap()
            .crit_pct,
        0.0
    );
    let events = collect_broadcast_swings(h, player, victim, SWINGS);
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
    // Ordinary miss/dodge are valid attempts, not landed normal hits.
    let damage: Vec<_> = events.into_iter().filter(|e| e.amount > 0.0).collect();
    assert!(!damage.is_empty());
    damage
}

fn spawn_darion(h: &mut Harness) -> Entity {
    let player = h.caster;
    let darion =
        crate::zones::summon_creature(h.app.world_mut(), player, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(darion)
            .unwrap()
            .0,
        0x20000
    );
    apply(h, darion, darion, NPC_BUFFS, (1 << 1) | (1 << 2));
    darion
}

#[test]
fn native_melee_victim_caster_crit_matching_native_identity_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    apply_full_source(&mut h);
    qualify_lifetime(&mut h, victim, npc);
    assert_eq!(modifier(&h, npc, 306), 0.0);
    assert_eq!(modifier(&h, npc, 52), -100.0);
    assert_hit(&h, npc);
}

#[test]
fn native_melee_victim_caster_crit_foreign_identity_and_joint_inputs_are_runtime_qualified() {
    let (mut h, victim) = fixture();
    let (foreign, npc) = (h.caster, h.npc);
    apply(&mut h, foreign, victim, CASTER_CRIT, 1);
    apply(&mut h, victim, victim, NEGATIVE_RECEIVED, 1 << 1);
    apply(&mut h, npc, npc, AUTO_CRIT, 1);
    qualify_lifetime(&mut h, victim, foreign);
    assert_eq!(modifier(&h, victim, 197), -101.0);
    assert_eq!(modifier(&h, npc, 334), 50.0);
    assert_hit(&h, npc);
}

#[test]
fn native_melee_victim_caster_crit_player_zero_source_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    let player = h.caster;
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(player)
            .unwrap()
            .crit_pct,
        0.0
    );
    apply(&mut h, player, victim, CASTER_CRIT, 1);
    qualify_lifetime(&mut h, victim, player);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(player)
            .unwrap()
            .crit_pct,
        0.0
    );
    assert_eq!(modifier(&h, player, 306), 0.0);
}

#[test]
fn native_melee_victim_caster_crit_no_crit_source_is_runtime_qualified() {
    let (mut h, victim) = fixture();
    let player = h.caster;
    let darion =
        crate::zones::summon_creature(h.app.world_mut(), player, 30_838, [0.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::CreatureFlagsExtra>(darion)
            .unwrap()
            .0,
        0x20000
    );
    apply(&mut h, darion, darion, NPC_BUFFS, (1 << 1) | (1 << 2));
    apply(&mut h, darion, victim, CASTER_CRIT, 1);
    qualify_lifetime(&mut h, victim, darion);
    assert_hit(&h, darion);
}

#[test]
#[ignore = "behavioral RED before actual-caster victim306 consumer"]
fn native_melee_victim_caster_crit_matching_npc_saturates_received_chance() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    assert_caster(&h, victim, npc);
    let events = npc_swings(&mut h, victim, 5);
    assert_all_crit(&events);
}

#[test]
#[ignore = "behavioral RED before actual-caster victim306 consumer"]
fn native_melee_victim_caster_crit_matching_player_uses_victim_aura() {
    let (mut h, victim) = fixture();
    let player = h.caster;
    apply(&mut h, player, victim, CASTER_CRIT, 1);
    assert_caster(&h, victim, player);
    let damage = player_damage(&mut h, victim);
    assert_all_crit(&damage);
}

#[test]
#[ignore = "behavioral control: victim306 must filter actual caster"]
fn native_melee_victim_caster_crit_foreign_caster_does_not_change_npc_source() {
    let (mut h, victim) = fixture();
    let foreign = h.caster;
    apply(&mut h, foreign, victim, CASTER_CRIT, 1);
    assert_caster(&h, victim, foreign);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control: victim306 must filter actual caster"]
fn native_melee_victim_caster_crit_foreign_caster_does_not_grant_player_crit() {
    let (mut h, victim) = fixture();
    let foreign = h.npc;
    apply(&mut h, foreign, victim, CASTER_CRIT, 1);
    let damage = player_damage(&mut h, victim);
    assert_no_crit(&damage);
}

#[test]
#[ignore = "behavioral control for actual victim placement"]
fn native_melee_victim_caster_crit_attacker_only_does_not_modify_actual_victim() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, npc, CASTER_CRIT, 1);
    assert_eq!(modifier(&h, victim, 306), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for actual victim placement"]
fn native_melee_victim_caster_crit_foreign_target_does_not_modify_actual_victim() {
    let (mut h, victim) = fixture();
    let (npc, foreign) = (h.npc, h.caster);
    apply(&mut h, npc, foreign, CASTER_CRIT, 1);
    assert_caster(&h, foreign, npc);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for native aura cancellation"]
fn native_melee_victim_caster_crit_cancelled_matching_aura_restores_baseline() {
    let (mut h, victim) = fixture();
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    crate::auras::remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |a| {
        a.spell_id == CASTER_CRIT
    });
    h.advance(3);
    assert_eq!(modifier(&h, victim, 306), 0.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral RED before source and matching victim306 pre-clamp sum"]
fn native_melee_victim_caster_crit_meets_negative_source_before_zero_clamp() {
    let (mut h, victim) = fixture();
    apply_full_source(&mut h);
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    let events = npc_swings(&mut h, victim, SWINGS);
    // Source-95 + matching306100 gives5%, not0(missing306) or100(earlysourceclamp).
    assert_both(&events);
}

#[test]
#[ignore = "behavioral control for received197 and matching306 raw sum"]
fn native_melee_victim_caster_crit_combines_with_negative_197_before_clamp() {
    let (mut h, victim) = fixture();
    apply_full_source(&mut h);
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    apply(&mut h, victim, victim, NEGATIVE_RECEIVED, 1 << 1);
    assert_eq!(modifier(&h, victim, 197), -101.0);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_no_crit(&events);
}

#[test]
#[ignore = "behavioral control for received clamp before attacker334"]
fn native_melee_victim_caster_crit_received_sum_clamps_before_attacker_334() {
    let (mut h, victim) = fixture();
    apply_full_source(&mut h);
    let npc = h.npc;
    apply(&mut h, npc, victim, CASTER_CRIT, 1);
    apply(&mut h, victim, victim, NEGATIVE_RECEIVED, 1 << 1);
    apply(&mut h, npc, npc, AUTO_CRIT, 1);
    let events = npc_swings(&mut h, victim, SWINGS);
    assert_both(&events);
}

#[test]
#[ignore = "behavioral RED: source NO_CRIT must not mask matching306"]
fn native_melee_victim_caster_crit_applies_after_native_source_no_crit() {
    let (mut h, victim) = fixture();
    let darion = spawn_darion(&mut h);
    apply(&mut h, darion, victim, CASTER_CRIT, 1);
    assert_caster(&h, victim, darion);
    let events = collect_broadcast_swings(&mut h, darion, victim, 5);
    assert_all_crit(&events);
}

#[test]
#[ignore = "behavioral control: summon owner is not the actual attacker caster"]
fn native_melee_victim_caster_crit_summon_owner_caster_is_not_attacker() {
    let (mut h, victim) = fixture();
    let darion = spawn_darion(&mut h);
    let owner = h.caster;
    assert_eq!(
        h.app
            .world()
            .get::<crate::zones::TempSummon>(darion)
            .unwrap()
            .summoner,
        owner
    );
    apply(&mut h, owner, victim, CASTER_CRIT, 1);
    assert_caster(&h, victim, owner);
    let events = collect_broadcast_swings(&mut h, darion, victim, 5);
    assert_no_crit(&events);
}
