//! Bloody Demise keeps its primary strike separate from its secondary area.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical world database and MID2 Blood inputs"]
fn bloody_demise_hits_primary_once_and_secondary_targets_once() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(250, 3).unwrap();
    let actor = run.actor;
    let targets = run.targets.clone();
    engage(&mut run);
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        1310571,
        Some(targets[0]),
        1
    ));
    for _ in 0..10 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    for target in targets {
        let hits: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(1310571)
            })
            .collect();
        assert_eq!(hits.len(), 1, "target {target:?}: {hits:?}");
        assert!(hits[0].amount > 0);
    }
}
