use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_each_earned_howl_survives_ordinary_cobra_until_actual_kill_command() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324, 472325] {
        let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
        while !crate::auras::has_aura(run.app.world(), run.actor, ready)
            && crate::spell_cast::now(run.app.world()) < deadline
        {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, ready),
            "native clock must actually earn readiness{ready}"
        );
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                193455,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .expect("ordinary literal Cobra between earning Howl and casting KC");
        preparation::finish_pending_setup(&mut run).unwrap();
        // Let the actual Cobra projectile and ordinary next-GCD interval finish.
        for _ in 0..30 {
            run.app.update();
        }
        // SimC76f8118 hunter3776–3817: only Kill Command consumes earned
        // beasts. Cobra/Barbed/other successful casts must not spend readiness.
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, ready),
            "actual readiness{ready} must survive ordinary Cobra until KC"
        );
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary KC consumes the earned beast");
        assert!(!crate::auras::has_aura(run.app.world(), run.actor, ready));
        preparation::finish_pending_setup(&mut run).unwrap();
    }
}
