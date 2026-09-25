//! `npc_training_dummy` against real spells from data/world.db (skipped when absent).

use shared::components::{CombatStatus, Health, Npc, PowerType, UnitFactionTemplate};
use shared::protocol::CombatLogKind;

use super::*;
use crate::auras::ActiveAuras;
use crate::combat::DeathState;
use crate::spell_cast::test_harness::{Harness, harness};

const MORTAL_STRIKE: u32 = 12294;
/// Weapon damage plus a 15 s MOD_DECREASE_SPEED aura.
const HAMSTRING: u32 = 1715;
/// Grandmaster's Training Dummy (31144): HealthModifier 0.000187 × 100.
const DUMMY_HEALTH: f32 = 0.000187 * 100.0;

/// The harness NPC turned into Grandmaster's Training Dummy (faction 7).
fn dummy_harness() -> Option<Harness> {
    let mut h = harness(Vec::new(), &[MORTAL_STRIKE, HAMSTRING], 1)?;
    h.app.world_mut().entity_mut(h.npc).insert((
        Npc {
            template_id: 31144,
            name: "Grandmaster's Training Dummy".into(),
        },
        UnitFactionTemplate(7),
        Health {
            current: DUMMY_HEALTH,
            max: DUMMY_HEALTH,
        },
        TrainingDummy::default(),
    ));
    h.set_power(PowerType::Rage, 1000.0);
    Some(h)
}

fn has_aura(h: &Harness, spell_id: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == spell_id))
}

fn in_combat(h: &Harness) -> bool {
    h.app
        .world()
        .get::<CombatStatus>(h.npc)
        .is_some_and(|status| status.0)
}

#[test]
fn spell_damage_beyond_its_health_is_logged_but_the_dummy_keeps_its_health() {
    let Some(mut h) = dummy_harness() else {
        return;
    };
    h.cast_and_deliver(MORTAL_STRIKE, Some(h.npc));

    let log = std::mem::take(&mut h.inbox.log);
    let damage = log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage)
        .expect("Mortal Strike damage logged");
    // 5.2012 AP coefficient × 100 AP; overkill against the 0.0187 health.
    assert_eq!((damage.amount, damage.overflow), (520, 519));
    assert!(log.iter().all(|e| e.kind != CombatLogKind::Death));
    assert_eq!(h.health(h.npc), DUMMY_HEALTH);
    assert!(h.app.world().get::<DeathState>(h.npc).is_none());
    assert!(in_combat(&h));
}

#[test]
fn dummy_evades_five_seconds_after_the_last_hit_dropping_its_debuffs() {
    let Some(mut h) = dummy_harness() else {
        return;
    };
    h.cast_and_deliver(HAMSTRING, Some(h.npc));
    assert!(has_aura(&h, HAMSTRING));
    assert!(in_combat(&h));

    // 4.5 s at 20 Hz: still in combat, slowed.
    h.advance(90);
    assert!(in_combat(&h));
    assert!(has_aura(&h, HAMSTRING));

    // Past the 5 s reset timer.
    h.advance(20);
    assert!(!in_combat(&h));
    assert!(
        !has_aura(&h, HAMSTRING),
        "evade removes the attacker's Hamstring"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::AuraRemoved && e.spell_id == Some(HAMSTRING))
    );
    assert_eq!(h.health(h.npc), DUMMY_HEALTH);
}

#[test]
fn melee_swings_on_the_dummy_reach_the_attackers_combat_log() {
    let Some(mut h) = dummy_harness() else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    // Swings at 0 s and 2 s.
    h.advance(45);

    let swings: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.kind == CombatLogKind::Damage && e.spell_id.is_none())
        .map(|e| (e.source, e.target, e.school_mask, e.amount, e.overflow))
        .collect();
    let (caster, npc) = (Some(h.caster.to_bits()), Some(h.npc.to_bits()));
    assert_eq!(swings, vec![(caster, npc, 1, 10, 9); 2]);
    assert!(h.inbox.log.iter().all(|e| e.kind != CombatLogKind::Death));
    assert_eq!(h.health(h.npc), DUMMY_HEALTH);
}
