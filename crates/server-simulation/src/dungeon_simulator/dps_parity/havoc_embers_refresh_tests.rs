//! SimC76f8118 sc_demon_hunter.cpp8973–8980,9015–9026.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_undying_embers_can_proc_from_immolation_refresh() {
    for selected in [false, true] {
        let _seed = crate::random::seed_scope(70_448);
        let mut run = prepare_literal(577, 1).unwrap();
        let actor = run.actor;
        let known = run
            .app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .unwrap();
        assert!(known.contains(1272405));
        let spells = known
            .iter()
            .filter(|spell| selected || *spell != 1272405)
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
        for _ in 0..20 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 258920, Some(actor), 1);
            for _ in 0..12 {
                run.app.update();
            }
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0[before..];
        let applications = events
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                ) && event.spell_id == Some(258920)
            })
            .count();
        assert_eq!(
            applications > 20,
            selected,
            "only selected Undying Embers can trigger extra refreshes: {applications}"
        );
    }
}
