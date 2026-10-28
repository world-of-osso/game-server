//! Eye of Infinity must affect only the actual Surge primary.
use super::*;

#[test]
#[ignore = "requires native Devastation follower profile and world DB"]
fn devastation_eye_of_infinity_empowers_only_real_surge_primary() {
    fn damage(selected: bool, target_index: usize) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let primary = run.targets[0];
        let target = run.targets[target_index];
        assert!(crate::auras::has_aura(run.app.world(), actor, 411165));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 411165,
            );
        }
        run.app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor, target, 359077, "surge", 1,
            ),
        );
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            actor,
            359073,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
            2,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..30 {
            run.app.update();
        }
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == 359077)
            .expect("real native Surge impact")["factors"]["raw_amount"]
            .as_f64()
            .unwrap()
    }
    let primary = damage(true, 0) / damage(false, 0);
    let secondary = damage(true, 1) / damage(false, 1);
    assert!(
        (primary - 1.15).abs() < 0.001,
        "selected native15% primary bonus: {primary}"
    );
    assert!(
        (secondary - 1.0).abs() < 0.001,
        "secondary must not receive the primary bonus: {secondary}"
    );
}
