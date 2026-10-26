use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn outlaw_sinister_strike_earns_opportunity_and_delayed_extra_attack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    for _ in 0..32 {
        {
            let mut pools = run.app.world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor).unwrap();
            let energy = pools.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(), run.actor,
            literal_rotation::execution::Execution::Spell(193315, run.targets[0], Some(run.targets[0])),
        ).unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        if crate::auras::has_aura(run.app.world(), run.actor, 195627) {
            break;
        }
    }
    // SimC76f8118 sc_rogue.cpp8255–8280: landed Sinister Strike rolls
    // Opportunity and dispatches its independent extra attack after300ms.
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 195627),
        "ordinary native Sinister Strikes must earn Opportunity");
    for _ in 0..10 { run.app.update(); }
    let events = &run.app.world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>().0;
    assert!(events.iter().any(|event| event.source == Some(run.actor.to_bits())
        && event.target == Some(run.targets[0].to_bits())
        && event.kind == CombatLogKind::Damage
        && event.spell_id == Some(197834) && event.amount > 0),
        "earned Opportunity must launch the actual native extra strike on the source target");
}
