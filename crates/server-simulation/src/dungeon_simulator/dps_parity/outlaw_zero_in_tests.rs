//! Zero In must be earned and consumed by the real next Between the Eyes.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_zero_in_earned_by_auto_attack_is_consumed_by_bte() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1259485));
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    for _ in 0..400 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), run.actor, 1259486) {
            break;
        }
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1259486),
        "real auto-attacks must earn native Zero In"
    );
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .remove::<crate::combat::AutoAttack>();
    {
        let mut powers = run
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
            .unwrap();
        let energy = powers.get_mut(PowerType::Energy).unwrap();
        energy.current = energy.max;
        powers.get_mut(PowerType::ComboPoints).unwrap().current = 5.0;
    }
    run.app
        .world_mut()
        .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .clear();
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(315341, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..10 {
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
                && event.spell_id == Some(315341)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    let relevant: Vec<_> = events
        .iter()
        .filter(|event| matches!(event.spell_id, Some(1259486 | 315341)))
        .collect();
    eprintln!("Zero In cast events: {relevant:?}");
    // SimC76f8118 sc_rogue.cpp3422–3430/3871: earned Zero In
    // affects the next real Between the Eyes and then expires.
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 1259486),
        "actual Between the Eyes must consume its earned Zero In stacks"
    );
}
