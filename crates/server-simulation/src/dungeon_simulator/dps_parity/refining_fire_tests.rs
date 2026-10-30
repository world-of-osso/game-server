//! Real Avenger's Shield impacts deposit a rolling Refining Fire budget.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "native66 reference inputs and private world database"]
fn avengers_shield_refining_fire_carries_overlapping_damage_without_recritting() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(66, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 469883));
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    for _ in 0..2 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            31935,
            Some(target),
            0
        ));
        for _ in 0..8 {
            run.app.update();
        }
    }
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let source: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(31935)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    let ticks: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(469882)
                && event.kind == CombatLogKind::Damage
        })
        .collect();
    assert_eq!(source.len(), 2, "two native Shield impacts");
    assert!(
        !ticks.is_empty(),
        "the selected talent applies native Refining Fire"
    );
    assert!(
        ticks.iter().all(|event| !event.crit),
        "the residual cannot crit again"
    );
    let source_total: i64 = source.iter().map(|event| i64::from(event.amount)).sum();
    let tick_total: i64 = ticks.iter().map(|event| i64::from(event.amount)).sum();
    assert!(
        (source_total - tick_total).abs() <= 10,
        "100% residual must retain both overlapping hit budgets: shield={source_total}, ticks={tick_total}"
    );
}
