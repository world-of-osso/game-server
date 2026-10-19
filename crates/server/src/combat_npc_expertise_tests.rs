//! TC a352b1fa Unit.cpp2777–2782/2817–2822: NPC attacker240/4, not block.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::components::{Level, Position, Rotation, UnitLevel};
use shared::protocol::{CombatEvent, CombatEventType, CombatLogEvent, CombatLogKind, MissKind};

use super::facing_tests::collect_swings;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{Harness, drain, harness, harness_with, spawn_npc, world_db};

const NPC_HIT_BUFFS: u32 = 62_627;
const PET_PASSIVE: u32 = 28_305;
const EVASION: u32 = 5_277;
const DUMMY: u32 = 31_144;
const EXPERTISE: u32 = 240;
const EVASION_SWINGS: usize = 5;
const NPC_SWINGS: usize = 512;

fn expertise(h: &Harness, actor: Entity) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(EXPERTISE))
}

fn cast_npc_buffs(h: &mut Harness, npc: Entity) {
    crate::spell_cast::creature_cast(h.app.world_mut(), npc, NPC_HIT_BUFFS, Some(npc), true)
        .unwrap();
    h.advance(3);
    assert_eq!(expertise(h, npc), 10_000.0);
}

fn cancel_expertise(h: &mut Harness, actor: Entity, spell: u32) {
    crate::auras::remove_auras(h.app.world_mut(), actor, RemoveReason::Cancelled, |a| {
        a.spell_id == spell
    });
    h.advance(3);
    assert_eq!(expertise(h, actor), 0.0);
}

fn apply_small_expertise(h: &mut Harness, actor: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target: actor,
            spell_id: PET_PASSIVE,
            effect_mask: 1 << 1,
            source: AuraSource::Cast,
        },
    );
    h.advance(3);
    assert_eq!(expertise(h, actor), 14.0);
}

fn rogue_duel() -> Harness {
    let mut h = harness_with(Vec::new(), &[EVASION, NPC_HIT_BUFFS], 4, true).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 3.0,
        y: 0.0,
        z: 0.0,
    });
    h.advance(3);
    h
}

fn cast_evasion(h: &mut Harness) {
    h.cast_and_deliver(EVASION, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .modifier(49),
        200.0
    );
    assert!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .defense
            .dodge
            >= 200.0
    );
}

fn assert_evasion_live(h: &Harness) {
    assert!(crate::auras::has_aura(h.app.world(), h.caster, EVASION));
}

fn dodged(event: &CombatLogEvent) -> bool {
    event.kind == CombatLogKind::Miss(MissKind::Dodge)
}

fn assert_evasion_avoids(events: &[CombatLogEvent]) {
    assert!(
        events.iter().all(|e| matches!(
            e.kind,
            CombatLogKind::Miss(MissKind::Miss | MissKind::Dodge)
        )),
        "{events:?}"
    );
    // Five unseeded native attempts: P(allMiss)=.05^5=3.125e-7.
    assert!(events.iter().any(dodged), "{events:?}");
}

fn collect_broadcast_swings(h: &mut Harness, attacker: Entity, victim: Entity) -> Vec<CombatEvent> {
    h.app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            super::start_attack(&mut commands, attacker, victim, None);
        })
        .unwrap();
    let mut events = Vec::new();
    for _ in 0..NPC_SWINGS * 60 {
        h.advance(1);
        events.extend(
            drain::<CombatEvent>(&mut h.app, h.connection)
                .into_iter()
                .filter(|event| {
                    event.attacker == attacker.to_bits()
                        && event.target == victim.to_bits()
                        && event.spell_id == 0
                        && matches!(
                            event.event_type,
                            CombatEventType::MeleeDamage
                                | CombatEventType::CriticalHit
                                | CombatEventType::Miss
                                | CombatEventType::Dodge
                                | CombatEventType::Parry
                                | CombatEventType::Block
                        )
                }),
        );
        if events.len() == NPC_SWINGS {
            super::attack_stop(h.app.world_mut(), attacker);
            h.advance(3);
            return events;
        }
    }
    panic!(
        "only {} of{NPC_SWINGS} broadcast swings: {events:?}",
        events.len()
    );
}

fn dummy_duel() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[NPC_HIT_BUFFS, PET_PASSIVE], 1).unwrap();
    // Neither NPC has a player owner: subscribe the real all-client broadcast,
    // not the owner-only CombatLogEvent path. Do not fabricate NPC ownership.
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<CombatEvent>::default(),
        MessageReceiver::<CombatEvent>::default(),
    ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.world_mut().entity_mut(h.npc).insert(UnitLevel(80));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    // The native victim faces the NPC attacker atx3, not the player owner.
    let target = crate::zones::summon_creature(
        h.app.world_mut(),
        caster,
        DUMMY,
        [6.0, 0.0, 0.0, std::f32::consts::PI],
    )
    .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(target).unwrap().0, 80);
    (h, target)
}

#[test]
fn native_melee_npc_expertise_28305_lifetime_is_runtime_qualified() {
    let mut h = harness(Vec::new(), &[PET_PASSIVE], 1).unwrap();
    let actor = h.npc;
    apply_small_expertise(&mut h, actor);
    let start = h.app.world().resource::<Time>().elapsed_secs_f64();
    // Qualify one actual application across more than the512-swing window.
    // No reapplication, auto-attacks, clock edits or duration mutation.
    h.advance(NPC_SWINGS * 60);
    let elapsed = h.app.world().resource::<Time>().elapsed_secs_f64() - start;
    assert!(elapsed > NPC_SWINGS as f64 * 2.0);
    assert!(crate::auras::has_aura(h.app.world(), actor, PET_PASSIVE));
    assert_eq!(expertise(&h, actor), 14.0);
    eprintln!("native28305 effect1 remains14 after {elapsed}s without refresh");
}

#[test]
fn native_melee_npc_expertise_beats_live_player_evasion() {
    let mut h = rogue_duel();
    let (attacker, victim) = (h.npc, h.caster);
    cast_npc_buffs(&mut h, attacker);
    cast_evasion(&mut h);
    let before = h.health(victim);
    let events = collect_swings(&mut h, attacker, victim, EVASION_SWINGS);
    assert_evasion_live(&h);
    assert_eq!(expertise(&h, attacker), 10_000.0);
    assert!(!events.iter().any(dodged), "{events:?}");
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.amount > 0),
        "{events:?}"
    );
    assert_eq!(
        before - h.health(victim),
        events.iter().map(|e| e.amount as f32).sum::<f32>()
    );
}

#[test]
fn native_melee_npc_expertise_cancelled_before_evasion_restores_dodge() {
    let mut h = rogue_duel();
    let (attacker, victim) = (h.npc, h.caster);
    cast_npc_buffs(&mut h, attacker);
    cancel_expertise(&mut h, attacker, NPC_HIT_BUFFS);
    cast_evasion(&mut h);
    let before = h.health(victim);
    let events = collect_swings(&mut h, attacker, victim, EVASION_SWINGS);
    assert_evasion_live(&h);
    assert_evasion_avoids(&events);
    assert_eq!(h.health(victim), before);
}

#[test]
fn native_melee_npc_expertise_on_foreign_actor_does_not_reduce_dodge() {
    let mut h = rogue_duel();
    let (attacker, victim) = (h.npc, h.caster);
    let foreign = spawn_npc(&mut h.app, 1.0);
    cast_npc_buffs(&mut h, foreign);
    assert_eq!(expertise(&h, attacker), 0.0);
    cast_evasion(&mut h);
    let before = h.health(victim);
    let events = collect_swings(&mut h, attacker, victim, EVASION_SWINGS);
    assert_evasion_live(&h);
    assert_evasion_avoids(&events);
    assert_eq!(h.health(victim), before);
}

#[test]
fn native_melee_npc_expertise_suppresses_npc_dodge_parry_not_block() {
    let (mut h, victim) = dummy_duel();
    let attacker = h.npc;
    cast_npc_buffs(&mut h, attacker);
    let events = collect_broadcast_swings(&mut h, attacker, victim);
    assert_eq!(expertise(&h, attacker), 10_000.0);
    // Old missed9% dodge/parry bug probability .91^512≈1.07e-21.
    assert!(
        !events.iter().any(|e| matches!(
            e.event_type,
            CombatEventType::Dodge | CombatEventType::Parry
        )),
        "{events:?}"
    );
    // Block remains native3%; P(noBlock)=.97^512≈1.687e-7.
    assert!(
        events
            .iter()
            .any(|e| e.event_type == CombatEventType::Block),
        "{events:?}"
    );
}

#[test]
fn native_melee_npc_expertise_small_native_amount_preserves_remaining_parry() {
    // Executed only AFTER the separate runtime28305 lifetime qualification passes.
    let (mut h, victim) = dummy_duel();
    let attacker = h.npc;
    apply_small_expertise(&mut h, attacker);
    let events = collect_broadcast_swings(&mut h, attacker, victim);
    assert_eq!(expertise(&h, attacker), 14.0);
    assert!(
        !events
            .iter()
            .any(|e| e.event_type == CombatEventType::Dodge),
        "{events:?}"
    );
    // Native14/4=3.5 removesDodge3 but leavesParry2.5, not a blanket mask.
    // P(noParry)=.975^512≈2.346e-6; block remains3%.
    assert!(
        events
            .iter()
            .any(|e| e.event_type == CombatEventType::Parry),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| e.event_type == CombatEventType::Block),
        "{events:?}"
    );
    cancel_expertise(&mut h, attacker, PET_PASSIVE);
    let restored = collect_broadcast_swings(&mut h, attacker, victim);
    assert!(
        restored
            .iter()
            .any(|e| e.event_type == CombatEventType::Dodge),
        "{restored:?}"
    );
}
