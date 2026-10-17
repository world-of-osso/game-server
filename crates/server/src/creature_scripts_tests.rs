//! `npc_training_dummy` against real spells from data/world.db.

use shared::components::{CombatStatus, Health, Npc, PowerType, UnitFactionTemplate};
use shared::protocol::CombatLogKind;

use super::*;
use crate::auras::ActiveAuras;
use crate::combat::DeathState;
use crate::spell_cast::test_harness::{Harness, harness_with_native_enemy_dodge_reduction};

const MORTAL_STRIKE: u32 = 12294;
/// Weapon damage plus a 15 s MOD_DECREASE_SPEED aura.
const HAMSTRING: u32 = 1715;
/// Grandmaster's Training Dummy (31144): ceil(12600 level 80 basehp2 × HealthModifier 0.000187).
const DUMMY_HEALTH: f32 = 3.0;

/// The harness NPC turned into Grandmaster's Training Dummy (faction 7).
fn dummy_harness() -> Option<Harness> {
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[MORTAL_STRIKE, HAMSTRING], 1)?;
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
    // 5.2012 AP coefficient × 100 AP ± 2.5%; overkill against the 3 health.
    assert!(
        crate::spell_cast::test_harness::varied(520.12).contains(&(damage.amount as f32)),
        "{damage:?}"
    );
    assert_eq!(damage.overflow, damage.amount - 3);
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
    h.attack(h.npc);
    // Swings at 0 s and 2 s.
    h.advance(45);

    let swings: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id.is_none() && e.kind != CombatLogKind::Death)
        .collect();
    let (caster, npc) = (Some(h.caster.to_bits()), Some(h.npc.to_bits()));
    assert_eq!(swings.len(), 2, "{swings:?}");
    for swing in swings {
        assert_eq!(
            (swing.source, swing.target, swing.school_mask),
            (caster, npc, 1)
        );
        // Unarmed 1-2 + 100 AP / 6 × 2.0 s = 34-35 over the dummy's 3 health,
        // unless missed, dodged or parried; a partial block subtracts `blocked`.
        match swing.kind {
            CombatLogKind::Damage => {
                assert!(
                    (34..=35).contains(&(swing.amount + swing.blocked)),
                    "{swing:?}"
                );
                assert_eq!(swing.overflow, swing.amount - 3);
            }
            CombatLogKind::Miss(_) => assert_eq!(swing.amount, 0),
            _ => panic!("{swing:?}"),
        }
    }
    assert!(h.inbox.log.iter().all(|e| e.kind != CombatLogKind::Death));
    assert_eq!(h.health(h.npc), DUMMY_HEALTH);
}

/// Evade → `CombatStop` → `RemoveAllAttackers`: a player still auto-attacking
/// the dummy from out of range stops, and its combat with the dummy has ended with
/// the dummy's: auto-attack alone keeps nobody in combat.
#[test]
fn evade_stops_out_of_range_attackers_and_they_leave_combat() {
    let Some(mut h) = dummy_harness() else {
        return;
    };
    let (caster, dummy) = (h.caster, h.npc);
    h.attack(dummy);
    h.advance(3);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(shared::components::Position {
            x: 40.0,
            y: 0.0,
            z: 0.0,
        });
    // Past the dummy's 5 s reset timer: it evades.
    h.advance(110);
    assert!(!in_combat(&h));
    assert!(
        h.app
            .world()
            .get::<crate::combat::AutoAttack>(caster)
            .is_none(),
        "the evade stops the attacker's auto-attack"
    );
    assert_eq!(
        h.app.world().get::<CombatStatus>(caster),
        Some(&CombatStatus(false))
    );
}

/// Agony 980: instant harmful periodic damage, a tick every 2 s for 18 s.
const AGONY: u32 = 980;

/// TC `npc_training_dummy::DamageTaken`: periodic damage (`DOT`) does not restart an
/// attacker's 5 s combat timer, and `Unit::DealDamage` starts no combat itself.
#[test]
fn periodic_ticks_do_not_keep_the_attacker_in_combat_with_the_dummy() {
    let Some(mut h) = crate::spell_cast::test_harness::harness(Vec::new(), &[AGONY], 9) else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(TrainingDummy::default());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(CombatStatus(false));
    h.cast_and_deliver(AGONY, Some(h.npc));
    let caster_in_combat = |h: &Harness| {
        h.app
            .world()
            .get::<CombatStatus>(h.caster)
            .is_some_and(|status| status.0)
    };
    h.advance(90);
    assert!(caster_in_combat(&h), "4.5 s after the cast");
    h.advance(30);
    assert!(
        !caster_in_combat(&h),
        "6 s after the cast, ticks notwithstanding"
    );
    let ticks = std::mem::take(&mut h.inbox.log)
        .into_iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.periodic)
        .count();
    assert!(ticks >= 2, "Agony ticked {ticks} times");
}
