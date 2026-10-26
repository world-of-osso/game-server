//! Actual native Swipe keeps its native primary-only rank2 contribution.
use super::marksmanship_mechanics_tests::cast;
use super::*;

#[test]
#[ignore = "requires private spec255 player inputs and native world DB"]
fn survival_native_rank_two_swipe_amplifies_only_actual_primary_damage() {
    // SimC76f8118 sc_hunter.cpp6195–6196: rank2's second native effect
    // applies to chain_target0, not every in-radius recipient.
    for target_index in [0, 1] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(255, 3).unwrap();
        hunter_fixture::provision_pet(run.app.world_mut(), run.actor).unwrap();
        cast(&mut run, 883, true);
        engage(&mut run);
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 1259017));
        cast(&mut run, 186270, false);
        let actor = run.actor;
        let target = run.targets[target_index];
        run.app.world_mut().insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor,
                target,
                1262293,
                "native_swipe_primary",
                32,
            ),
        );
        cast(&mut run, 186270, false);
        assert_actual_swipe_amount(&run, target_index == 0);
    }
}

fn assert_actual_swipe_amount(run: &Run, primary: bool) {
    let bonus =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 1259017, 1);
    let effectiveness = if primary {
        1.0 + f64::from(bonus) / 100.0
    } else {
        1.0
    };
    let capture = run
        .app
        .world()
        .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
        .export();
    let hits: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event["event"] == "result" && event["spell_id"] == 1262293)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "actual native Swipe must resolve once on this real recipient"
    );
    let factors = &hits[0]["factors"];
    // The actual source amount already includes target-specific script factors.
    // Remove only that captured boundary's contribution, then compare the final
    // pre-mitigation amount with this same hit's live stat and crit effects.
    let input = factors["native_modded_amount"].as_f64().unwrap();
    let amount = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1262293)
        .expect("actual Swipe must expose its observed source factors");
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
    let expected = (input * other_done * effectiveness).round();
    assert!(input > 0.0 && factors["actual_amount"].as_f64().unwrap() > 0.0);
    println!(
        "NATIVE_SWIPE primary={primary} source={input} native_bonus={bonus} actual={actual} expected={expected}"
    );
    assert!(
        (actual - expected).abs() <= 1.0,
        "actual Swipe primary={primary} amount={actual} must include native source bonus: expected={expected}, factors={factors}"
    );
}
