use super::*;

fn owned_howl_deadline(run: &Run) -> Option<f64> {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == 471877 && aura.caster == run.actor)?
        .expires_at
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_dire_summons_shortens_real_howl_clock_and_earns_next_beast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for (spell, effect) in [(34026, 0), (193455, 2)] {
        let before = owned_howl_deadline(&run).expect("actual running native Howl clock");
        let reduction = crate::spell_cast::scripts::script_effect_value(
            run.app.world(),
            run.actor,
            472352,
            effect,
        ) as f64
            / 1000.0;
        assert!(
            reduction > 0.0,
            "selected native Dire Summons owns a time value"
        );
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary native recovery producer");
        let after = owned_howl_deadline(&run).expect("shortened clock still has time remaining");
        assert!(
            (before - after - reduction).abs() < 0.001,
            "ordinary {spell} must shorten Howl by its native Dire Summons effect: before={before}, after={after}, reduction={reduction}"
        );
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let deadline = owned_howl_deadline(&run).unwrap();
    while crate::spell_cast::now(run.app.world()) < deadline - 0.5 {
        run.app.update();
    }
    assert!(owned_howl_deadline(&run).is_some());
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        193455,
        Some(run.targets[0]),
        false,
    )
    .expect("actual Cobra shortens the final native half-second");
    assert!(
        owned_howl_deadline(&run).is_none(),
        "exhausted native Howl clock must end"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "early clock expiry must run the real next-beast callback"
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        193455,
        Some(run.targets[0]),
        false,
    )
    .expect("Cobra remains an ordinary native cast without a running Howl clock");
    assert!(
        owned_howl_deadline(&run).is_none(),
        "recovery must not start an inactive clock"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 471878));
}
