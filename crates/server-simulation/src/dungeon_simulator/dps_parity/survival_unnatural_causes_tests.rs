//! Unnatural Causes follows native eligibility for real Hunter damage.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

fn target_factor(spell: u32, pet_source: bool, selected: bool, execute: bool) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    engage(&mut run);
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 459527,
        );
    }
    if execute {
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(run.targets[0])
            .unwrap();
        health.current = health.max * 0.1;
    }
    let caster = if pet_source { pet } else { run.actor };
    run.app.insert_resource(FactorCapture::new(
        caster,
        run.targets[0],
        spell,
        "unnatural_causes",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        caster,
        spell,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..50 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event"]
                == if spell == 265157 {
                    "direct_amount"
                } else {
                    "periodic_amount"
                }
        })
        .expect("actual native damage")["applied_multipliers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["factor"] == "giantkiller")
        .unwrap()["value"]
        .as_f64()
        .unwrap()
}

#[test]
#[ignore = "requires native Survival255 profile and world data"]
fn survival_unnatural_causes_uses_native_mask_and_execute_values() {
    // SimC76f8118 sc_hunter.cpp1482–1509/2640–2664: actual selected talent
    // amplifies eligible damage, while bugs=true excludes direct Wildfire.
    assert!((target_factor(1253171, false, false, false) - 1.0).abs() < 0.001);
    assert!((target_factor(1253171, false, true, false) - 1.10).abs() < 0.001);
    assert!((target_factor(1253171, false, true, true) - 1.15).abs() < 0.001);
    assert!((target_factor(1253138, true, true, false) - 1.10).abs() < 0.001);
    assert!((target_factor(265157, false, true, false) - 1.0).abs() < 0.001);
}
