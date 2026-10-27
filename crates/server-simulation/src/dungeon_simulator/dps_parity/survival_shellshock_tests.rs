//! Actual Boomstick hit uses native Shellshock's execution target count.
use super::marksmanship_mechanics_tests::cast;
use super::*;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_shellshock_modifies_real_single_and_three_target_boomstick_hits() {
    // SimC76f8118 sc_hunter.cpp6268–6281: selected effect1 applies on
    // one target; for multiple targets subtract count times native effect2.
    for count in [1, 3] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(255, count).unwrap();
        engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1252931));
        let actor = run.actor;
        let target = run.targets[0];
        run.app.world_mut().insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor,
                target,
                1261215,
                "actual_boomstick",
                8,
            ),
        );
        cast(&mut run, 1261193, false);
        assert_actual_shellshock(&run, count);
    }
}

fn assert_actual_shellshock(run: &Run, count: usize) {
    let base =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1252931, 0);
    let penalty =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1252931, 1);
    let percent = if count > 1 {
        (base - count as f32 * penalty).max(0.0)
    } else {
        base
    };
    let capture = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let events = capture["events"].as_array().unwrap();
    let hit = events
        .iter()
        .find(|event| event["event"] == "result" && event["spell_id"] == 1261215)
        .expect("actual native Boomstick must resolve on its real primary");
    let amount = events
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1261215)
        .expect("actual Boomstick must expose its observed source factors");
    let factors = &hit["factors"];
    let input = factors["native_modded_amount"].as_f64().unwrap();
    let applied = amount["applied_multipliers"].as_array().unwrap();
    let other_done: f64 = [
        "school_damage_done",
        "versatility",
        "summon_damage",
        "creature_type_damage",
        "mechanic_damage",
        "combo_strikes",
        "ret_damage",
        "shadow_weaving",
    ]
    .iter()
    .map(|name| {
        applied.iter().find(|row| row["factor"] == *name).unwrap()["value"]
            .as_f64()
            .unwrap()
    })
    .product();
    let actual = factors["native_rounded_amount"].as_f64().unwrap();
    let expected = (input * other_done * (1.0 + f64::from(percent) / 100.0)).round();
    assert!(input > 0.0 && factors["actual_amount"].as_f64().unwrap() > 0.0);
    println!(
        "NATIVE_SHELLSHOCK targets={count} percent={percent} actual={actual} expected={expected}"
    );
    assert!(
        (actual - expected).abs() <= 1.0,
        "actual Boomstick target count{count} must apply native Shellshock: actual={actual} expected={expected}"
    );
}
