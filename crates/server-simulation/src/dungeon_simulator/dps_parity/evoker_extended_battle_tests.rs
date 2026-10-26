//! Actual selected Mass marks are extended by actual Essence-base-cost casts.
use super::*;

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn evoker_extended_battle_real_paid_disintegrate_extends_each_owned_mark_once() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[1];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 441212));
        let seconds = f64::from(crate::spell_cast::scripts::script_effect_value(
            run.app.world(),
            run.actor,
            441212,
            0,
        ));
        assert!(seconds > 0.0);
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 441212,
            );
        }
        crate::spell_cast::creature_cast_empowered_with_targets(
            run.app.world_mut(),
            run.actor,
            359073,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
            1,
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            356995,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let expires = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(primary)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.spell_id == 434473 && aura.caster == run.actor)
                })
                .and_then(|aura| aura.expires_at)
                .expect("actual Mass primary must retain its native mark")
        };
        let before = expires(&run);
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 359618),
            "test the actual paid base-cost path, not a fabricated Essence Burst"
        );
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            356995,
            crate::spell_cast::CastTargets {
                unit: primary,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        let after = expires(&run);
        // SimC76f8118 sc_evoker.cpp3199–3212/3261–3275: extend each existing owned mark once.
        let expected = if selected { seconds } else { 0.0 };
        assert!(
            (after - before - expected).abs() < 0.001,
            "actual paid Disintegrate must extend exactly its native selected duration: before={before}, after={after}, expected={expected}"
        );
    }
}
