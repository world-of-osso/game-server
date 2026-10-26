use super::*;
use server::simc_apl::Facts;

fn barbed_recharge(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("barbed_shot".into(), 217200)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    crate::simc_apl_native::NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1)
        .value("cooldown.barbed_shot.full_recharge_time")
        .unwrap()
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pack_mentality_recovers_real_barbed_recharge_after_consuming_howl() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 472358),
        "native reference must select Pack Mentality"
    );
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "real combat clock must earn Wyvern readiness"
    );
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(19574, run.actor, Some(run.targets[0])),
    )
    .expect("ordinary Bestial Wrath advances a second earned beast");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 472324),
        "selected native Stampede must also earn Boar readiness"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        217200,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary Barbed starts native recharge");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878)
            && crate::auras::has_aura(run.app.world(), run.actor, 472324),
        "both earned beasts must remain ready after Barbed and its real GCD"
    );
    let before = barbed_recharge(&run);
    let reduction = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        472358,
        1,
    )) / 1000.0;
    assert!(
        before > reduction,
        "the actual running charge must have time to recover"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes both actually earned beasts");
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 471878));
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 472324));
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471881),
        "actual Wyvern summon must produce Cry"
    );
    let after = barbed_recharge(&run);
    // SimC76f8118 hunter3808–3811: each actual consumed beast reduces
    // already-running Barbed recharge by selected Pack Mentality's native time.
    let expected = (before - reduction * 2.0).max(0.0);
    assert!(
        (after - expected).abs() < 0.001,
        "actual Wyvern+Boar consumption must recover Barbed twice: before={before}, after={after}, per_beast={reduction}, expected={expected}"
    );
}
