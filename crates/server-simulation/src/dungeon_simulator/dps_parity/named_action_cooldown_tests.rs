//! Named action aliases read native charge state after an actual spell execution.
use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn native_named_action_charges_follow_real_conflagrate_payment() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(267, 1).unwrap();
    engage(&mut run);
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("conflagrate".into(), 17962)].into(),
        ..Default::default()
    };
    let read = |run: &Run, property: &str| {
        crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            crate::simc_apl_native::Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            1,
        )
        .value(&format!("action.conflagrate.{property}"))
        .unwrap()
    };
    let maximum = read(&run, "max_charges");
    assert!(maximum >= 2.0);
    assert_eq!(read(&run, "charges"), maximum);
    assert_eq!(read(&run, "charges_fractional"), maximum);
    assert_eq!(read(&run, "full_recharge_time"), 0.0);
    assert_eq!(read(&run, "usable"), 1.0);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(17962, run.targets[0], Some(run.targets[0])),
    )
    .unwrap();
    assert_eq!(
        read(&run, "usable"),
        1.0,
        "remaining native charges are usable even during the new GCD"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    let remaining = read(&run, "charges");
    assert!(remaining < maximum && remaining > 0.0);
    assert_eq!(read(&run, "max_charges"), maximum);
    assert!(read(&run, "charges_fractional") >= remaining);
    assert!(read(&run, "charges_fractional") < maximum);
    assert!(read(&run, "full_recharge_time") > 0.0);
    let combat = report::combat(run.app.world(), &run.targets, 2.0);
    assert!(
        combat["actions"].as_array().unwrap().iter().any(|action| {
            action["spell_id"] == 17962 && action["damage"].as_u64().unwrap() > 0
        }),
        "native Conflagrate must actually damage its recipient"
    );
}
