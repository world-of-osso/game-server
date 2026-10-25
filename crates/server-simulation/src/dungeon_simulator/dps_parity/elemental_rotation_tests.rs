//! Real profile-driven selection across native cast/GCD/resource boundaries.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private262 native DB and selected reference profile"]
fn elemental_literal_profile_uses_spenders_and_ascendance() {
    let mut run = prepare_measurement(262, 1).unwrap();
    let actor = run.actor;
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(actor)
            .is_some()
    );
    assert!(
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(actor)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == 191634),
        "authored Stormkeeper precombat"
    );
    engage(&mut run);
    for _ in 0..1200 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let casts: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::CastSuccess
        })
        .filter_map(|event| event.spell_id)
        .collect();
    assert!(
        casts.contains(&117014),
        "literal profile must spend Maelstrom on Elemental Blast: {casts:?}"
    );
    assert!(
        casts.contains(&1219480),
        "literal profile must cast Ascendance: {casts:?}"
    );
    assert!(
        casts.contains(&443454),
        "selected Ancestral Swiftness must execute: {casts:?}"
    );
    assert!(
        !casts.contains(&378081),
        "Ancestral Swiftness replaces Nature's Swiftness: {casts:?}"
    );
    assert!(
        !casts.contains(&8042),
        "selected Elemental Blast replaces Earth Shock: {casts:?}"
    );
}

#[test]
#[ignore = "requires private262 native DB and selected reference profile"]
fn elemental_literal_spell_targets_reads_live_native_candidates() {
    let mut run = prepare_literal(262, 3).unwrap();
    engage(&mut run);
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("chain_lightning".into(), 188443)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    for (targets, expected) in [(&run.targets[..], 3.0), (&run.targets[..1], 1.0)] {
        let facts = crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            targets.len(),
        )
        .with_targets(targets.first().copied(), targets);
        assert_eq!(
            server::simc_apl::Facts::value(&facts, "spell_targets.chain_lightning").unwrap(),
            expected
        );
    }
    run.app
        .world_mut()
        .get_mut::<shared::components::Health>(run.targets[1])
        .unwrap()
        .current = 0.0;
    let facts = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        encounter,
        3,
    )
    .with_targets(run.targets.first().copied(), &run.targets);
    assert_eq!(
        server::simc_apl::Facts::value(&facts, "spell_targets.chain_lightning").unwrap(),
        2.0
    );
}
