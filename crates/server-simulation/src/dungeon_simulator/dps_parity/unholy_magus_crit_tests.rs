//! Actual Magus periodic critical hits in the literal Unholy fight.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native252 profile and Magus templates"]
fn unholy_real_magus_withering_grasp_ticks_can_crit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    let ticks: Vec<_> = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.spell_id == Some(1297091)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0
        })
        .collect();
    assert!(
        !ticks.is_empty(),
        "actual Magus must deliver Withering Grasp ticks"
    );
    assert!(
        ticks.iter().any(|event| event.crit),
        "actual native Magus ticks must inherit owner critical chance"
    );
    assert!(
        ticks.iter().any(|event| !event.crit),
        "ordinary ticks must remain possible"
    );
}
