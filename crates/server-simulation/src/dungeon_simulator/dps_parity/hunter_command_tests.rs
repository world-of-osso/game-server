use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_war_orders_barbed_reduces_running_kill_command_recharge() {
    use server::simc_apl::Facts;
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Kill Command starts actual recharge");
    preparation::finish_pending_setup(&mut run).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("kill_command".into(), 34026)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let recharge = |run: &Run| {
        crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            1,
        )
        .value("cooldown.kill_command.full_recharge_time")
        .unwrap()
    };
    let before = recharge(&run);
    assert!(before > 0.0, "a real spent charge must be recharging");
    let seconds = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        393933,
        2,
    )) / 1000.0;
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        217200,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary selected War Orders producer");
    let after = recharge(&run);
    let expected = (before - seconds).max(0.0);
    assert!(
        (after - expected).abs() < 0.01,
        "successful Barbed must reduce native running KC recharge by War Orders' native amount: before={before}, after={after}, expected={expected}, reduction={seconds}"
    );
}
