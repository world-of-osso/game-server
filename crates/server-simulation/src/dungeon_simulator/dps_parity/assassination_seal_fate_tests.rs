//! Seal Fate must use critical native builder impacts, including triggered hands.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires prepared259 private native DB and player actor"]
fn assassination_seal_fate_critical_builders_earn_native_combo_points() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(259, 3).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 14190));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..800 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let critical_builders = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.crit
                && event
                    .spell_id
                    .is_some_and(|spell| [51723, 5374, 27576, 385627, 1265795].contains(&spell))
        })
        .count();
    assert!(
        critical_builders > 10,
        "actual builder crits must reach the missing producer"
    );
    let grants: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Energize
                && event.spell_id == Some(14189)
        })
        .collect();
    assert_eq!(
        grants.len(),
        critical_builders,
        "each eligible critical impact, not white swings or noncritical hits, must grant one native Seal Fate child"
    );
    assert!(grants.iter().all(|event| event.amount == 1));
}
