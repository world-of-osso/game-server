use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native261 profile/traits and player actor"]
fn subtlety_literal_driver_coats_and_executes_profile_opener() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(261, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "Subtlety must execute literal SimC rows, not AssistedCombat"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 315584));
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..200 {
        drive_player(&mut run).unwrap();
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
                && matches!(event.spell_id, Some(185438 | 53 | 200758 | 197835 | 196819))),
        "literal Subtlety must execute its native offensive action"
    );
}
