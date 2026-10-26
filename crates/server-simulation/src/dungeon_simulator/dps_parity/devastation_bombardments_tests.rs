//! Actual Mass Disintegrate producer and explicitly configured SimC external attempts.
use super::*;

fn cast_mass_source(run: &mut Run, primary: Entity, earned: bool) {
    if earned {
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
        preparation::finish_pending_setup(run).unwrap();
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 436336));
    }
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
}

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn devastation_bombardments_actual_mass_applies_only_its_selected_primary() {
    for (selected, earned) in [(false, true), (true, false), (true, true)] {
        let mut run = prepare_literal(1467, 3).unwrap();
        engage(&mut run);
        let primary = run.targets[1];
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 434300));
        if !selected {
            crate::auras::remove_auras(
                run.app.world_mut(),
                run.actor,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 434300,
            );
        }
        cast_mass_source(&mut run, primary, earned);
        for _ in 0..40 {
            run.app.update();
        }
        // SimC76f8118 sc_evoker.cpp6588–6591: earned Mass marks only chain_target0.
        for target in &run.targets {
            let active = run
                .app
                .world()
                .get::<crate::auras::ActiveAuras>(*target)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == 434473 && aura.caster == run.actor)
                });
            assert_eq!(
                active,
                selected && earned && *target == primary,
                "native Bombardments must require an actual earned Mass primary: selected={selected}, earned={earned}, target={target:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires exact Devastation native profile and DB"]
fn devastation_bombardments_parity_external_attempts_keep_the_native_target_icd() {
    let mut run = prepare_literal(1467, 3).unwrap();
    // Install the actual parity driver/options, not a fabricated damage/proc event.
    let _driver = devastation_rotation::prepare_driver(&mut run).unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    engage(&mut run);
    let primary = run.targets[0];
    cast_mass_source(&mut run, primary, true);
    let mut seen = 0;
    let mut times = Vec::new();
    for _ in 0..100 {
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[seen..] {
            if event.source == Some(run.actor.to_bits())
                && event.target == Some(primary.to_bits())
                && event.spell_id == Some(434481)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
            {
                times.push(crate::spell_cast::now(run.app.world()));
            }
        }
        seen = events.len();
    }
    assert!(
        times.len() >= 2,
        "configured SimC external attempts must execute actual native Bombardments: {times:?}"
    );
    let driver = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(443788)
        .unwrap();
    let icd = f64::from(
        driver
            .aura_options
            .as_ref()
            .unwrap()
            .proc_category_recovery_ms,
    ) / 1000.0;
    assert!(icd > 0.0);
    for pair in times.windows(2) {
        assert!(
            pair[1] - pair[0] >= icd - 0.001,
            "external attempts must share the native443788 target ICD: {times:?}, ICD={icd}"
        );
    }
}
