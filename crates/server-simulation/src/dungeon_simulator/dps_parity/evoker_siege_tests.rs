//! Native combat entry/exit and the actual Unrelenting Siege periodic stack clock.
use super::*;

#[test]
#[ignore = "requires exact native Devastation profile and DB"]
fn evoker_siege_actual_combat_grows_native_stacks_and_removes_them_on_exit() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 1).unwrap();
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 441246));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 441246,
            );
        }
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 441248),
            "Siege must not be active out of combat"
        );
        let actor = run.actor;
        let target = run.targets[0];
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), actor, target);
        run.app.update();
        let aura = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(run.actor)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|aura| aura.spell_id == 441248 && aura.caster == run.actor)
                })
                .cloned()
        };
        assert_eq!(
            aura(&run).is_some(),
            selected,
            "actual combat entry earns native Siege only for the selected talent"
        );
        if !selected {
            continue;
        }
        let initial = aura(&run).unwrap();
        assert_eq!(initial.stacks, 1);
        let period = initial
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .period_ms;
        assert!(period > 0 && initial.max_stacks > 1);
        for _ in 0..(period / 50 + 2) {
            run.app.update();
        }
        assert_eq!(
            aura(&run).unwrap().stacks,
            2,
            "one native period earns one real stack"
        );
        for frame in 0..(u32::from(initial.max_stacks) * (period / 50 + 1)) {
            // Real direct hits refresh the training dummy's separate combat timer.
            if frame % 40 == 0 {
                assert!(crate::spell_cast::try_cast_triggered(
                    run.app.world_mut(),
                    actor,
                    361500,
                    Some(target),
                    1
                ));
            }
            run.app.update();
        }
        let capped = aura(&run).unwrap();
        assert_eq!(
            capped.stacks, capped.max_stacks,
            "native cumulative aura owns the cap"
        );
        crate::combat_refs::end_all_combat(run.app.world_mut(), actor);
        run.app.update();
        // SimC76f8118 sc_evoker.cpp9926–9939/10841–10849: real combat state owns the aura.
        assert!(
            aura(&run).is_none(),
            "actual combat exit removes earned Siege stacks"
        );
    }
}
