use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn unholy_literal_driver_executes_native_profile_actions() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).expect("literal Unholy preparation");
    assert_eq!(driver_report(&run)["driver"], "LITERAL_SIMC_UNHOLY_76f8118");
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).expect("literal Unholy decision");
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(85948 | 55090 | 77575))),
        "the literal APL must launch an actual native generator or disease"
    );
}
