use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn overflowing_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 394195)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_noncritical_frostbolt_earns_overflowing_energy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 390218));
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 0.0;
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        228597,
        Some(run.targets[0]),
        1,
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(228597)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && !event.crit),
        "the actual native Frostbolt payload must land noncritically"
    );
    run.app.update();
    assert_eq!(
        overflowing_stacks(&run),
        1,
        "selected Overflowing Energy must earn one native stack after the actual noncritical hit"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_critical_spike_clears_overflowing_energy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 394195, Some(run.actor), 1);
    assert_eq!(overflowing_stacks(&run), 1);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        228600,
        Some(run.targets[0]),
        1,
    );
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(228600)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.crit),
        "the actual native Spike payload must critically hit"
    );
    run.app.update();
    assert_eq!(
        overflowing_stacks(&run),
        0,
        "an eligible real critical Spike must clear Overflowing Energy after its hit batch"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_lance_preserves_unrelated_overflowing_energy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 394195, Some(run.actor), 1);
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        228598,
        Some(run.targets[0]),
        1,
    );
    run.app.update();
    assert_eq!(
        overflowing_stacks(&run),
        1,
        "Ice Lance is not an Overflowing Energy producer or consumer in the pinned source"
    );
}
