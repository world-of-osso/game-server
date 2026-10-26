use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_native_main_gauche_is_earned_from_landed_builders_and_cannot_recurse() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                193315,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    for _ in 0..10 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.targets[0].to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(86392)
                && event.amount > 0
        })
        .count();
    // SimC76f8118 sc_rogue.cpp5999–6001/7948–7973: native mastery
    // rolls from landed Sinister roots/extras and main-hand weapon swings.
    assert!(
        hits > 0,
        "real native builders must earn actual Main Gauche payload damage"
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    crate::spell_cast::cast_triggered(
        run.app.world_mut(),
        run.actor,
        86392,
        Some(run.targets[0]),
        1,
    );
    for _ in 0..10 {
        run.app.update();
    }
    let copies = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == Some(86392)
                && event.amount > 0
        })
        .count();
    assert_eq!(
        copies, 1,
        "a Main Gauche payload cannot produce another Main Gauche"
    );
}
