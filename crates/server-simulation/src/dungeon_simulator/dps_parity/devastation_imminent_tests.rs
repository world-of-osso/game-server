//! Native Deep Breath earns finite, flat-cost uses; actual Essence Burst preserves them.
use super::*;

fn disintegrate_cost(run: &Run) -> f64 {
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("measured".into(), 356995)].into(),
        ..Default::default()
    };
    let facts = crate::simc_apl_native::NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        crate::simc_apl_native::Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        },
        run.targets.len(),
    );
    server::simc_apl::Facts::value(&facts, "action.measured.cost").unwrap()
}

fn essence(run: &Run) -> f32 {
    run.app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(run.actor)
        .unwrap()
        .get(shared::components::PowerType::Essence)
        .unwrap()
        .current
}

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn devastation_imminent_destruction_actual_breath_earns_flat_uses_preserved_by_essence_burst() {
    for (selected, free) in [(false, false), (true, false), (true, true)] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[0];
        let base = disintegrate_cost(&run);
        assert!(base > 0.0);
        let talent_uses =
            crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 370781, 0)
                as u8;
        assert!(talent_uses > 1);
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 370781,
            );
        }
        let breath =
            literal_rotation::execution::resolve_action(run.app.world(), run.actor, 357210)
                .unwrap();
        assert_eq!(
            breath,
            Some(433874),
            "exact profile uses actual native Maneuverability"
        );
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            run.actor,
            433874,
            crate::spell_cast::CastTargets {
                unit: run.actor,
                primary: Some(primary),
                destination: None,
            },
        )
        .unwrap();
        assert!(
            run.app
                .world()
                .get::<shared::casting::CastState>(run.actor)
                .is_some_and(|cast| cast.spell_id == 433874
                    && cast.cast_type == shared::casting::CastType::Channel),
            "selected Maneuverability plus Command Squadron must retain its actual native channel before last-tick Imminent grant"
        );
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 411055),
            "the first flight cannot grant its last-tick resource uses at launch"
        );
        preparation::finish_pending_setup(&mut run).unwrap();
        let window = |run: &Run| {
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(run.actor)
                .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 411055))
                .cloned()
        };
        assert_eq!(
            window(&run).is_some(),
            selected,
            "actual Deep Breath must earn Imminent Destruction only with the selected talent"
        );
        let amount = window(&run).map_or(0.0, |aura| {
            assert_eq!(
                aura.stacks, talent_uses,
                "native talent supplies initial uses, not the larger aura cap"
            );
            aura.effects
                .iter()
                .find(|effect| effect.index == 0)
                .unwrap()
                .amount
        });
        if free {
            // Earn real Essence Burst through actual Dragonrage and an actual Living Flame.
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
            crate::spell_cast::creature_cast_with_targets(
                run.app.world_mut(),
                run.actor,
                361469,
                crate::spell_cast::CastTargets {
                    unit: primary,
                    primary: Some(primary),
                    destination: None,
                },
            )
            .unwrap();
            preparation::finish_pending_setup(&mut run).unwrap();
            assert!(crate::auras::has_aura(run.app.world(), run.actor, 359618));
        }
        let expected_cost = if free { 0.0 } else { base + f64::from(amount) };
        assert_eq!(
            disintegrate_cost(&run),
            expected_cost,
            "native flat discount is applied once, not multiplied by the remaining use count"
        );
        let before = essence(&run);
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
        assert!(
            (f64::from(before - essence(&run)) - expected_cost).abs() < 0.001,
            "the actual channel must pay its real native Essence cost"
        );
        let remaining = window(&run).map_or(0, |aura| aura.stacks);
        let expected_uses = if !selected {
            0
        } else if free {
            talent_uses
        } else {
            talent_uses - 1
        };
        // SimC76f8118 sc_evoker.cpp3004–3006/3223–3250/6368–6371/10643–10648.
        assert_eq!(
            remaining, expected_uses,
            "ordinary paid Disintegrate spends one Imminent use; actual Essence Burst preserves it"
        );
    }
}
