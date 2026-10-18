//! TC a352b1fa Unit.cpp2389/2775: signed attacker251 reaches ordinary swings.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use shared::components::{Level, Position, Rotation, UnitLevel};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::{
    Harness, harness, harness_with, load_native_creature_world, world_db,
};

const SUNWELL: u32 = 45_769;
const FEINT: u32 = 48_250;
const SHA: u32 = 110_294;
const DUST: u32 = 26_072;
const STUN: u32 = 853;
const DUMMY_SWINGS: usize = 512;
const PLAYER_SWINGS: usize = 256;

type NativeEffect = (Entity, u32, u8);

fn apply_effects(h: &mut Harness, effects: &[NativeEffect]) {
    for &(target, spell_id, effect) in effects {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: target,
                target,
                spell_id,
                effect_mask: 1 << effect,
                source: AuraSource::Cast,
            },
        );
    }
}

fn modifier(h: &Harness, actor: Entity) -> f32 {
    h.app
        .world()
        .get::<ActiveAuras>(actor)
        .map_or(0.0, |a| a.modifier(251))
}

fn swings(
    h: &mut Harness,
    attacker: Entity,
    victim: Entity,
    count: usize,
    effects: &[NativeEffect],
) -> Vec<CombatLogEvent> {
    apply_effects(h, effects);
    let mut cursor = h.inbox.log.len();
    h.app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            super::start_attack(&mut commands, attacker, victim, None);
        })
        .unwrap();
    let mut events = Vec::new();
    for _ in 0..count * 60 {
        h.advance(1);
        let before = events.len();
        events.extend(
            h.inbox.log[cursor..]
                .iter()
                .filter(|event| {
                    event.source == Some(attacker.to_bits())
                        && event.target == Some(victim.to_bits())
                        && event.spell_id.is_none()
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .cloned(),
        );
        cursor = h.inbox.log.len();
        if events.len() == count {
            super::attack_stop(h.app.world_mut(), attacker);
            h.advance(3);
            return events;
        }
        if events.len() != before {
            // Ordinary reapplication, not duration/swing-timer mutation. Each
            // authored effect remains live through the next native2s swing.
            apply_effects(h, effects);
        }
    }
    panic!(
        "only {} of{count} ordinary swings: {events:?}",
        events.len()
    );
}

fn dummy_duel() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[SUNWELL], 1).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), UnitLevel(80)));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let target =
        crate::zones::summon_creature(h.app.world_mut(), caster, 31_144, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    assert_eq!(h.app.world().get::<UnitLevel>(target).unwrap().0, 80);
    (h, target)
}

fn player_duel(behind: bool) -> Harness {
    let mut h = harness_with(Vec::new(), &[FEINT, SHA, DUST, STUN], 4, true).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        Level(80),
        UnitLevel(80),
        Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    let native = load_native_creature_world(&[40]);
    let template = &native.templates[&40];
    h.app.world_mut().entity_mut(h.npc).insert((
        shared::components::Npc {
            template_id: 40,
            name: template.name.clone(),
        },
        shared::components::UnitFactionTemplate(u32::from(template.faction)),
        UnitLevel(80),
        Position {
            x: if behind { -3.0 } else { 3.0 },
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.insert_resource(native);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .defense
            .dodge,
        0.0
    );
    // A real level80 player's computed pool survives this fixed native sample;
    // no fabricated HP, damage, rating, RNG or shortened swing timers.
    assert!(h.health(h.caster) > (PLAYER_SWINGS * 20) as f32);
    assert!(crate::spell_cast::targets::is_hostile(
        h.app.world(),
        h.npc,
        h.caster
    ));
    h
}

fn dodged(event: &CombatLogEvent) -> bool {
    event.kind == CombatLogKind::Miss(MissKind::Dodge)
}

fn assert_landed(events: &[CombatLogEvent]) {
    assert!(
        events
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.amount > 0),
        "{events:?}"
    );
}

#[test]
#[ignore]
fn native_melee_attacker_251_sunwell_removes_dodge_until_cancelled() {
    let (mut h, target) = dummy_duel();
    let caster = h.caster;
    let before = swings(&mut h, caster, target, DUMMY_SWINGS, &[]);
    // Each native3% positive control: P(noDodge)=.97^512≈1.69e-7.
    assert!(before.iter().any(dodged), "{before:?}");
    let reduced = swings(
        &mut h,
        caster,
        target,
        DUMMY_SWINGS,
        &[(caster, SUNWELL, 1)],
    );
    assert_eq!(modifier(&h, caster), -20.0);
    assert!(!reduced.iter().any(dodged), "{reduced:?}");
    crate::auras::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |a| {
        a.spell_id == SUNWELL
    });
    let restored = swings(&mut h, caster, target, DUMMY_SWINGS, &[]);
    assert_eq!(modifier(&h, caster), 0.0);
    assert!(restored.iter().any(dodged), "{restored:?}");
}

#[test]
#[ignore]
fn native_melee_attacker_251_positive_covers_remaining_table_until_cancelled() {
    let mut h = player_duel(false);
    let (attacker, victim) = (h.npc, h.caster);
    let events = swings(
        &mut h,
        attacker,
        victim,
        PLAYER_SWINGS,
        &[(attacker, DUST, 0), (attacker, FEINT, 2)],
    );
    assert_eq!(modifier(&h, attacker), 20.0);
    assert_eq!(
        super::creature_miss_chance(h.app.world().get::<crate::auras::AuraStatMods>(attacker)),
        90.0
    );
    assert!(
        events.iter().all(|e| matches!(
            e.kind,
            CombatLogKind::Miss(MissKind::Miss | MissKind::Dodge)
        )),
        "{events:?}"
    );
    // P(noDodge)=.9^256≈1.93e-12; no artificially certain RNG.
    assert!(events.iter().any(dodged), "{events:?}");
    crate::auras::remove_auras(h.app.world_mut(), attacker, RemoveReason::Cancelled, |a| {
        a.spell_id == FEINT
    });
    let restored = swings(
        &mut h,
        attacker,
        victim,
        PLAYER_SWINGS,
        &[(attacker, DUST, 0)],
    );
    assert_eq!(modifier(&h, attacker), 0.0);
    assert!(!restored.iter().any(dodged), "{restored:?}");
    assert_landed(&restored);
}

#[test]
#[ignore]
fn native_melee_attacker_251_sums_signed_native_effects_before_clamp() {
    let mut h = player_duel(false);
    let (attacker, victim) = (h.npc, h.caster);
    let events = swings(
        &mut h,
        attacker,
        victim,
        PLAYER_SWINGS,
        &[
            (attacker, DUST, 0),
            (attacker, FEINT, 2),
            (attacker, SHA, 0),
        ],
    );
    assert_eq!(modifier(&h, attacker), -5.0);
    assert!(!events.iter().any(dodged), "{events:?}");
    assert_landed(&events);
}

#[test]
#[ignore]
fn native_melee_attacker_251_on_recipient_does_not_reduce_attacker_dodge() {
    let (mut h, target) = dummy_duel();
    let caster = h.caster;
    let events = swings(
        &mut h,
        caster,
        target,
        DUMMY_SWINGS,
        &[(target, SUNWELL, 1)],
    );
    assert_eq!(modifier(&h, caster), 0.0);
    assert_eq!(modifier(&h, target), -20.0);
    assert!(events.iter().any(dodged), "{events:?}");
}

#[test]
#[ignore]
fn native_melee_attacker_251_positive_cannot_restore_behind_player_dodge() {
    let mut h = player_duel(true);
    let (attacker, victim) = (h.npc, h.caster);
    let events = swings(
        &mut h,
        attacker,
        victim,
        PLAYER_SWINGS,
        &[(attacker, FEINT, 2)],
    );
    assert_eq!(modifier(&h, attacker), 20.0);
    assert!(!events.iter().any(dodged), "{events:?}");
    assert_landed(&events);
}

#[test]
#[ignore]
fn native_melee_attacker_251_positive_cannot_restore_stunned_player_dodge() {
    let mut h = player_duel(false);
    let (attacker, victim) = (h.npc, h.caster);
    let events = swings(
        &mut h,
        attacker,
        victim,
        PLAYER_SWINGS,
        &[(attacker, FEINT, 2), (victim, STUN, 0)],
    );
    assert_eq!(modifier(&h, attacker), 20.0);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(victim)
            .unwrap()
            .is_stunned()
    );
    assert!(!events.iter().any(dodged), "{events:?}");
    assert_landed(&events);
}
