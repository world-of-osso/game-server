//! Ruthlessness on an actual resource-sized Rogue channel.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_ruthlessness_killing_spree_refunds_once_on_channel_execution() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 14161));
    engage(&mut run);
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
        literal_rotation::execution::Execution::Spell(51690, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.spell_id == Some(51690) && event.kind == CombatLogKind::CastSuccess)
    );
    let gains = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1258235)
                && event.kind == CombatLogKind::Energize
        })
        .count();
    // sc_rogue.cpp2683–2688/4784–4810/8091–8127: actual channel
    // execution earns one guaranteed20%-per-point Ruthlessness gain at5 CP.
    assert_eq!(
        gains, 1,
        "Killing Spree must trigger Ruthlessness once, not once per hand tick"
    );
}
