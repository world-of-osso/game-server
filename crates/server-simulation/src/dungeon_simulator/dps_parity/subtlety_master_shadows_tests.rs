//! Master of Shadows earns its native Energy clock on in-combat Shadow Dance.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_master_shadows_dance_starts_native_periodic_energy() {
    let mut run = prepare_literal(261, 1).unwrap();
    engage(&mut run);
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 196976),
        "selected native Master of Shadows"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(53, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap()
        .get_mut(PowerType::Energy)
        .unwrap()
        .current = 0.0;
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(185313, run.actor, Some(run.targets[0])),
    )
    .unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 196980),
        "actual in-combat Shadow Dance earns the native Master of Shadows buff"
    );
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.spell_id == Some(196980)
                && event.kind == CombatLogKind::Energize
                && event.periodic
                && event.amount > 0),
        "the native periodic aura actually returns Energy"
    );
}
