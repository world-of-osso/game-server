use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn read_full_escalation(run: &Run) -> bool {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 441786 && aura.caster == run.actor)
        .is_some_and(|aura| aura.max_stacks > 0 && aura.stacks == aura.max_stacks)
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_earned_coup_executes_three_native_children_and_consumes_escalation() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    engage(&mut run);
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 441776).is_err(),
        "selected Coup must not be admitted before escalation is earned"
    );
    for _ in 0..96 {
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
        if read_full_escalation(&run) {
            break;
        }
    }
    assert!(
        read_full_escalation(&run),
        "real Sinister/Unseen executions must earn maximum Escalating Blade"
    );
    assert!(
        crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 2098).is_err(),
        "earned Coup must replace Dispatch until it is consumed"
    );
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
    // sc_rogue.cpp6863–6904: actual earned Coup is an ordinary native
    // finisher, not a forged triggered root or an injected learned-spell grant.
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(441776, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for spell in [462140, 462239, 462240] {
        let attempts = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(spell)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
            })
            .count();
        assert_eq!(
            attempts, 1,
            "earned native Coup must dispatch exactly one{spell} child"
        );
    }
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && [Some(462140), Some(462239), Some(462240)].contains(&event.spell_id)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 441786),
        "successful native Coup must consume its earned escalation"
    );
    assert!(crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 2098).is_ok());
    assert!(crate::spell_cast::knowledge::resolve_actor_action(run.app.world(), run.actor, 441776).is_err());
}
