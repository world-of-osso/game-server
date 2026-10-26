use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_frostbolt_executions_supply_fingers_and_brain_freeze() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    let mut fingers = false;
    let mut brain_freeze = false;
    for _ in 0..40 {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            116,
            Some(run.targets[0]),
            false,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        fingers |= crate::auras::has_aura(run.app.world(), run.actor, 44544);
        brain_freeze |= crate::auras::has_aura(run.app.world(), run.actor, 190446);
    }
    assert!(
        fingers,
        "actual Frostbolt executions must roll selected native Fingers"
    );
    assert!(
        brain_freeze,
        "actual Frostbolt executions must roll selected native Brain Freeze"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_orb_supplies_selected_splintering_orbs_conjures() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 444256));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(84714, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let impacts = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(443722)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    let native_count = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(444256)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 5)
        .unwrap()
        .base_points as usize;
    assert!(
        impacts >= native_count,
        "selected Orb must conjure at least its native{native_count} Splinters, got{impacts}"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_augury_earned_executions_conjure_extra_splinters() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1280165));
    for _ in 0..100 {
        crate::spell_cast::cast_triggered(
            run.app.world_mut(),
            run.actor,
            443722,
            Some(run.targets[0]),
            1,
        );
        for _ in 0..12 {
            run.app.update();
        }
    }
    for _ in 0..100 {
        run.app.update();
    }
    let impacts = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(443722)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert!(
        impacts > 100,
        "actual Splinter executions must earn Augury, rather than just100 source hits; got{impacts}"
    );
}
