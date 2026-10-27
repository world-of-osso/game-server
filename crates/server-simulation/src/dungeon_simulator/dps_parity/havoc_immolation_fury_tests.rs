//! SimC76f8118 sc_demon_hunter.cpp4651–4677,4784–4800.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_immolation_uses_spec_and_talent_fury_only() {
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_literal(577, 1).unwrap();
        let actor = run.actor;
        let known = run
            .app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap();
        assert!(known.contains(320374));
        let spells = known
            .iter()
            .filter(|spell| selected || *spell != 320374)
            .collect();
        run.app
            .world_mut()
            .entity_mut(actor)
            .insert(crate::class_progression::KnownSpells::fixture(spells));
        let before = run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .len();
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 258920, Some(actor), 1);
        for _ in 0..35 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..];
        let amounts = |spell| {
            events
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Energize && event.spell_id == Some(spell)
                })
                .map(|event| event.amount)
                .collect::<Vec<_>>()
        };
        assert_eq!(amounts(258920), vec![20], "only Havoc initial Fury");
        let ticks = amounts(258922);
        if selected {
            assert!(
                !ticks.is_empty() && ticks.iter().all(|amount| *amount == 3),
                "only selected Burning Hatred tick Fury: {ticks:?}"
            );
        } else {
            assert!(
                ticks.is_empty(),
                "unselected Burning Hatred cannot energize: {ticks:?}"
            );
        }
    }
}
