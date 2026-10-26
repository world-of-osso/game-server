//! Count earned native Ruthlessness combo points in the actual Outlaw driver.
use super::*;
use shared::protocol::CombatLogKind;

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
