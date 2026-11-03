//! Lava Burst earns MotE only after its Fusion children launch.
use super::elemental_mechanics_tests::{cast, fixture, own_aura, settle};
use crate::spell_cast::effects::factor_capture::FactorCapture;

fn fusion_factors(preexisting_mote: bool) -> (f64, f64, f64) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    let actor = run.actor;
    let target = run.targets[0];
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 260734 && aura.caster == actor,
    );
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        1219480,
        Some(actor),
        1
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        actor,
        191877,
        Some(actor),
        1
    ));
    if preexisting_mote {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            260734,
            Some(actor),
            1
        ));
    }
    settle(&mut run);
    assert!(
        own_aura(&run, 191877).is_some(),
        "real Power of the Maelstrom"
    );
    run.app
        .insert_resource(FactorCapture::new(actor, target, 117014, "fusion", 8));
    cast(&mut run, 51505);
    for _ in 0..40 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let factor = |spell| {
        capture["events"].as_array().unwrap().iter()
            .find(|event| event["spell_id"] == spell && event["event"] == "direct_amount")
            .unwrap_or_else(|| panic!("actual Fusion impact {spell}: {capture}"))["applied_multipliers"].as_array().unwrap().iter()
            .find(|value| value["factor"] == "cast_effectiveness").unwrap()["value"].as_f64().unwrap()
    };
    let fusion = factor(117014);
    let overload = factor(120588);
    let aura = own_aura(&run, 260734).expect("Lava Burst earns MotE after Fusion");
    let percent = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount as f64;
    cast(&mut run, 188196);
    assert!(
        own_aura(&run, 260734).is_none(),
        "the next eligible foreground Bolt consumes MotE"
    );
    (fusion, overload, percent)
}

#[test]
#[ignore = "requires native262 profile and world data"]
fn fusion_snapshots_old_mote_but_not_the_mote_earned_by_its_lava_burst() {
    let (new_fusion, new_overload, percent) = fusion_factors(false);
    let (old_fusion, old_overload, _) = fusion_factors(true);
    let multiplier = 1.0 + percent / 100.0;
    assert!(
        (new_fusion - 0.4).abs() < 0.001,
        "new MotE must not affect Fusion: {new_fusion}"
    );
    assert!(
        (new_overload - 0.4).abs() < 0.001,
        "new MotE must not affect its overload: {new_overload}"
    );
    assert!(
        (old_fusion - 0.4 * multiplier).abs() < 0.001,
        "preexisting MotE affects Fusion: {old_fusion}"
    );
    assert!(
        (old_overload - 0.4 * multiplier).abs() < 0.001,
        "preexisting MotE affects its overload: {old_overload}"
    );
}
