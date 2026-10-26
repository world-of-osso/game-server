//! Native first-hit factors, with actual Blade funding and optional ordinary Wrath.
use super::single_spell::{SpellProbe, capture_cast};
use super::*;

#[test]
#[ignore = "native Ret FV factor comparison needs exact private profile/traits"]
fn dps_parity_ret_factor_verdict_records_base_and_wrath_source_stages() {
    for wings in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare(70, 1).expect("actual Ret follower profile");
        assert!(
            run.app
                .world()
                .get::<crate::followers::Follower>(run.actor)
                .is_some()
        );
        engage(&mut run);
        if wings {
            ret_tier_tests::cast_finisher(&mut run, 31884);
        }
        capture_cast(
            &mut run,
            &SpellProbe {
                spec: 70,
                spell: 184575,
                damage_spell: 184575,
                action: "blade_of_justice",
                ticks: 1,
            },
        )
        .expect("actual native Blade priming");
        let mut receipt = capture_cast(
            &mut run,
            &SpellProbe {
                spec: 70,
                spell: 383328,
                damage_spell: 383328,
                action: "final_verdict",
                ticks: 1,
            },
        )
        .expect("actual naturally funded Final Verdict");
        receipt["priming"] = json!({"blade_of_justice": 1, "ordinary_avenging_wrath": wings});
        assert_eq!(receipt["casts"], 1);
        assert_eq!(receipt["captured_hits"], 1);
        let formula = receipt["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 383328)
            .expect("actual native Final Verdict formula");
        assert!(formula["factors"]["a_power"].as_f64().unwrap() > 0.0);
        assert!(formula["factors"]["a_mod"].as_f64().unwrap() > 0.0);
        let mode = if wings { "wrath" } else { "base" };
        let path = inputs::directory()
            .unwrap()
            .join(format!("70.final_verdict.{mode}.native.json"));
        std::fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        eprintln!("RET_FV_FACTORS mode={mode} path={}", path.display());
    }
}
