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

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn unholy_native_max_charges_survives_putrefy_consumption() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::{Action, ActionFacts, Facts};
    let mut run = prepare_literal(252, 1).expect("native Unholy fixture");
    let bindings = Bindings {
        actions: [("putrefy".into(), 1247378)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture { started_at: 0.0, duration: 300.0, style: FightStyle::Patchwerk };
    let action = Action { name: "putrefy".into(), source_row: None, target: None, options: Default::default() };
    let read = |run: &Run| {
        let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1);
        let maximum = facts.value("cooldown.putrefy.max_charges").expect("native maximum charges");
        assert_eq!(facts.value_for(&action, "max_charges").unwrap(), maximum);
        (maximum, facts.value("cooldown.putrefy.charges").unwrap())
    };
    let (maximum, available) = read(&run);
    assert!(maximum >= 2.0, "native Putrefy starts with multiple charges");
    assert_eq!(available, maximum);
    literal_rotation::execution::execute(run.app.world_mut(), run.actor,
        literal_rotation::execution::Execution::Spell(1247378, run.targets[0], Some(run.targets[0]))).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let (after_maximum, after_available) = read(&run);
    assert_eq!(after_maximum, maximum);
    assert_eq!(after_available, available - 1.0);
}
