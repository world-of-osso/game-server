//! Azure Sweep shares Azure Strike's once-per-cast native Essence Burst producer.
use super::*;

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn evoker_sweep_burst_actual_three_target_sweeps_earn_one_azure_use_per_cast() {
    for selected in [false, true] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[0];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 375721));
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 376872));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 375721,
            );
        }
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            375087,
            crate::spell_cast::CastTargets {
                unit: run.actor,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
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
        let stacks = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(run.actor)
                .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 359618))
                .map_or(0, |aura| aura.stacks)
        };
        assert_eq!(
            stacks(&run),
            0,
            "real Surge setup does not fabricate Essence Burst"
        );
        for used in 1..=2 {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, 375087),
                "actual Dragonrage must still guarantee the selected native Azure roll"
            );
            assert_eq!(
                literal_rotation::execution::resolve_action(run.app.world(), run.actor, 362969)
                    .unwrap(),
                Some(1265872),
                "actual earned Azure Sweep is the native action"
            );
            crate::spell_cast::creature_cast_with_targets(
                run.app.world_mut(),
                run.actor,
                1265872,
                crate::spell_cast::CastTargets {
                    unit: primary,
                    primary: Some(primary),
                    destination: None,
                },
            )
            .unwrap();
            preparation::finish_pending_setup(&mut run).unwrap();
            // sc_evoker.cpp5603–5628/5671–5678: one base execute, not one roll per victim.
            assert_eq!(
                stacks(&run),
                if selected { used } else { 0 },
                "actual three-target Sweep{used} must use selected Azure, never Ruby, exactly once"
            );
        }
    }
}
