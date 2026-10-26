//! Count earned native Ruthlessness combo points in the actual Outlaw driver.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_ruthlessness_finisher_requires_known_talent() {
    for selected in [true, false] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(260, 1).unwrap();
        engage(&mut run);
        if !selected {
            let known = run
                .app
                .world()
                .get::<crate::class_progression::KnownSpells>(run.actor)
                .unwrap();
            let retained = known.iter().filter(|id| *id != 14161).collect();
            run.app
                .world_mut()
                .entity_mut(run.actor)
                .insert(crate::class_progression::KnownSpells::fixture(retained));
        }
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
            literal_rotation::execution::Execution::Spell(
                2098,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events.iter().any(|e| e.spell_id == Some(2098)
                && e.kind == CombatLogKind::Damage
                && e.amount > 0)
        );
        let gains = events
            .iter()
            .filter(|e| {
                e.source == Some(run.actor.to_bits())
                    && e.spell_id == Some(1258235)
                    && e.kind == CombatLogKind::Energize
            })
            .count();
        // sc_rogue.cpp8091–8127: native20% per paid point; five points guarantee one.
        assert_eq!(
            gains,
            usize::from(selected),
            "Ruthlessness requires the known native talent"
        );
    }
}

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn diag_outlaw_ruthlessness_counts_native_finisher_gains() {
    let mut total = 0;
    for targets in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_measurement(260, targets).unwrap();
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 14161));
        engage(&mut run);
        let started_at = run.app.world().resource::<Time>().elapsed();
        install_fixture_lifetimes(&mut run, started_at);
        while run.app.world().resource::<Time>().elapsed() - started_at
            < std::time::Duration::from_secs(300)
        {
            drive_player(&mut run).unwrap();
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let gains: Vec<_> = events
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::Energize
                    && matches!(event.spell_id, Some(14161 | 1258235))
            })
            .collect();
        let points: i64 = gains.iter().map(|event| i64::from(event.amount)).sum();
        let overflow: i64 = gains.iter().map(|event| i64::from(event.overflow)).sum();
        println!(
            "Ruthlessness targets={targets}: events={} points={points} overflow={overflow}",
            gains.len()
        );
        total += points;
    }
    assert!(
        total > 0,
        "actual native finishers must earn Ruthlessness combo points"
    );
}
