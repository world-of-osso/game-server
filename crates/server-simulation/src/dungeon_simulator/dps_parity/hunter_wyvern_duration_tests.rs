use super::*;

fn cry_deadline(run: &Run) -> f64 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 471881 && aura.caster == run.actor)
        .expect("actual earned native Wyvern Cry")
        .expires_at
        .expect("finite native Cry")
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_fury_of_wyvern_command_extends_existing_cry_without_refresh() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 472550));
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
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes actual Wyvern and earns Cry");
    preparation::finish_pending_setup(&mut run).unwrap();
    let before = cry_deadline(&run);
    let extension = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        472550,
        1,
    )) / 1000.0;
    assert!(
        extension > 0.0,
        "selected native Fury must author a duration extension"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("second legal native KC extends already-active Cry");
    let after = cry_deadline(&run);
    // SimC76f8118 hunter6626–6630: extend the remaining Cry, not a
    // fresh full-duration application; native472550 effect2 owns the time.
    assert!(
        (after - before - extension).abs() < 0.001,
        "actual BM KC must extend existing Cry by native effect2: before={before}, after={after}, extension={extension}"
    );
}
