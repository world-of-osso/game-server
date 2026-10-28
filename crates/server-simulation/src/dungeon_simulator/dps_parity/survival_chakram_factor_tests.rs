//! Capture real Chakram bounces and their Radiant Edge factors.
use super::marksmanship_mechanics_tests::cast;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires native Survival255 profile and world data"]
fn survival_chakram_bounces_apply_native_radiant_edge_ramp() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(255, 1).unwrap();
    hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
    cast(&mut run, 883, true);
    engage(&mut run);
    cast(&mut run, 1250646, false);
    let percent =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1264903, 0)
            as f64;
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        1266081,
        "chakram",
        7,
    ));
    cast(&mut run, 1264949, false);
    for _ in 0..150 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let hits: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "direct_amount" && event["spell_id"] == 1266081)
        .collect();
    println!("CHAKRAM_FACTOR_CAPTURE {capture}");
    assert_eq!(hits.len(), 7);
    for (index, hit) in hits.iter().enumerate() {
        let factor = hit["applied_multipliers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["factor"] == "cast_effectiveness")
            .unwrap()["value"]
            .as_f64()
            .unwrap();
        let expected = (1.0 + percent / 100.0).powi(index as i32 + 1);
        assert!(
            (factor - expected).abs() < 0.001,
            "bounce{index}: {factor} vs {expected}"
        );
    }
}
