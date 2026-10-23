//! Ordinary native70 Mage casts; no flight timers, grants or admission bypasses.
//! TC177345 native3s AT→153564 native1s missile is the Meteor lifecycle oracle.
use bevy::prelude::Entity;
use server::simc_apl::Facts;
use shared::casting::CastState;
use shared::components::{Level, UnitLevel};
use shared::protocol::{CombatLogEvent, CombatLogKind};

use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents};

const METEOR: u32 = 153561;
const METEOR_IMPACT: u32 = 351140;
const METEOR_BURN: u32 = 155158;
const PYROBLAST: u32 = 11366;
const FIREBALL: u32 = 133;

fn prepare_flight() -> (Harness, Entity, Entity) {
    let (mut h, caster, targets) = super::meteor_tests::prepare_meteor(1);
    h.app
        .world_mut()
        .entity_mut(targets[0])
        .insert((Level(70), UnitLevel(70)));
    h.app.insert_resource(ObservedCombatEvents::default());
    (h, caster, targets[0])
}

fn flight(h: &Harness, actor: Entity, name: &str) -> (f64, f64) {
    let bindings = Bindings {
        actions: [("meteor".into(), METEOR), ("pyroblast".into(), PYROBLAST)].into(),
        global_cooldown_spell: METEOR,
        ..Default::default()
    };
    let world = h.app.world();
    let clock = Encounter::Dungeon {
        started_at: crate::spell_cast::now(world),
    };
    let facts = NativeFacts::new(world, actor, &bindings, clock, 1);
    // Evaluate BOTH existing fact-interface requests before failing the RED.
    match (
        facts.value(&format!("action.{name}.in_flight")),
        facts.value(&format!("action.{name}.in_flight_remains")),
    ) {
        (Ok(active), Ok(remains)) => (active, remains),
        results => panic!("native {name} flight facts unsupported: {results:?}"),
    }
}

fn events_for_spell(
    h: &Harness,
    caster: Entity,
    spell: u32,
) -> impl Iterator<Item = &CombatLogEvent> {
    h.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(move |event| {
            event.source == Some(caster.to_bits()) && event.spell_id == Some(spell)
        })
}

fn saw_event(h: &Harness, caster: Entity, spell: u32, kind: CombatLogKind) -> bool {
    events_for_spell(h, caster, spell).any(|event| event.kind == kind)
}

fn saw_impact(h: &Harness, caster: Entity, spell: u32) -> bool {
    events_for_spell(h, caster, spell)
        .any(|event| matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_)))
}

#[test]
fn native_apl_flight_hardcast_is_not_yet_launched_travel() {
    let (mut h, caster, target) = prepare_flight();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, PYROBLAST, Some(target), false)
        .unwrap();
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().spell_id,
        PYROBLAST
    );
    assert_eq!(flight(&h, caster, "pyroblast"), (0.0, 0.0));
    assert!(!saw_impact(&h, caster, PYROBLAST));
}

#[test]
fn native_apl_flight_pyroblast_uses_actor_scoped_actual_travel() {
    let (mut h, caster, target) = prepare_flight();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, PYROBLAST, Some(target), false)
        .unwrap();
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().spell_id,
        PYROBLAST
    );
    for _ in 0..200 {
        if h.app.world().get::<CastState>(caster).is_none() {
            break;
        }
        h.advance(1);
    }
    assert!(h.app.world().get::<CastState>(caster).is_none());
    assert!(saw_event(&h, caster, PYROBLAST, CombatLogKind::CastSuccess));
    assert!(!saw_impact(&h, caster, PYROBLAST));
    let (active, before) = flight(&h, caster, "pyroblast");
    assert_eq!(active, 1.0);
    assert!(before > 0.0);
    assert_eq!(flight(&h, h.caster, "pyroblast"), (0.0, 0.0));
    let at = crate::spell_cast::now(h.app.world());
    h.advance(1);
    let elapsed = crate::spell_cast::now(h.app.world()) - at;
    assert_eq!(flight(&h, caster, "pyroblast").0, 1.0);
    assert!((flight(&h, caster, "pyroblast").1 - (before - elapsed)).abs() < 0.001);
    h.advance(200);
    assert!(saw_impact(&h, caster, PYROBLAST));
    assert_eq!(flight(&h, caster, "pyroblast"), (0.0, 0.0));
}

#[test]
fn native_apl_flight_meteor_follows_real_at_to_missile_handoff() {
    let (mut h, caster, target) = prepare_flight();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, METEOR, Some(target), false)
        .unwrap();
    let (active, remains) = flight(&h, caster, "meteor");
    assert_eq!(active, 1.0);
    assert!(
        (remains - 4.0).abs() < 0.001,
        "native3s field plus1s missile"
    );
    assert_eq!(flight(&h, h.caster, "meteor"), (0.0, 0.0));
    h.advance(59);
    assert!(!saw_impact(&h, caster, METEOR_IMPACT));
    assert_eq!(flight(&h, caster, "meteor").0, 1.0);
    assert!((flight(&h, caster, "meteor").1 - 1.05).abs() < 0.001);
    h.advance(1);
    assert_eq!(flight(&h, caster, "meteor").0, 1.0);
    assert!((flight(&h, caster, "meteor").1 - 1.0).abs() < 0.001);
    h.advance(20);
    assert!(saw_impact(&h, caster, METEOR_IMPACT));
    assert_eq!(flight(&h, caster, "meteor"), (0.0, 0.0));
    h.advance(10);
    assert!(saw_impact(&h, caster, METEOR_BURN));
    assert_eq!(
        flight(&h, caster, "meteor"),
        (0.0, 0.0),
        "remaining burn is not flight"
    );
}

#[test]
fn native_apl_flight_meteor_ends_at_failed_native_expiry_admission() {
    let (mut h, caster, target) = prepare_flight();
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, METEOR, Some(target), false)
        .unwrap();
    h.advance(50);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIREBALL, Some(target), false)
        .unwrap();
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().spell_id,
        FIREBALL
    );
    h.advance(10);
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().spell_id,
        FIREBALL
    );
    assert_eq!(
        flight(&h, caster, "meteor"),
        (0.0, 0.0),
        "no synthetic4s timer after real153564 refusal"
    );
    h.advance(60);
    assert!(saw_impact(&h, caster, FIREBALL));
    assert!(!saw_impact(&h, caster, METEOR_IMPACT));
    assert_eq!(flight(&h, caster, "meteor"), (0.0, 0.0));
}
