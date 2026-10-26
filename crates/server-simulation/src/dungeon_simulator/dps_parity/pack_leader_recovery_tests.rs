use super::*;
use server::simc_apl::Facts;

fn bomb_recharge(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("wildfire_bomb".into(), 259495)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    crate::simc_apl_native::NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .value("cooldown.wildfire_bomb.full_recharge_time")
        .unwrap()
}

#[test]
#[ignore = "requires selected native255 Pack Leader inputs and world DB"]
fn native_pack_leader_bomb_clock_recovers_per_actual_consumed_beast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(255, 1).expect("native selected Pack Leader fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 472358));
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 471878));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(259495, run.targets[0], Some(run.targets[0])),
    )
    .expect("ordinary native Bomb starts its real recharge");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "Bomb must not consume the Wyvern intended for KC"
    );
    let before = bomb_recharge(&run);
    let reduction = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        472358,
        2,
    )) / 1000.0;
    assert!(before > reduction, "native Bomb must have running recovery");
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        259489,
        Some(run.targets[0]),
        false,
    )
    .expect("actual native command consumes one earned Wyvern");
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    // SimC76f8118 hunter3811: actual consumption reduces native Bomb
    // recovery by selected Pack Mentality effect3, not the BM effect2.
    let after = bomb_recharge(&run);
    assert!(
        (before - after - reduction).abs() < 0.001,
        "one actual consumed Wyvern must pay native Bomb recovery: before={before}, after={after}, reduction={reduction}"
    );
}
