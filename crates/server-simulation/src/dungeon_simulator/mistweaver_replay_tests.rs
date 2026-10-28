use std::path::Path;

#[test]
#[ignore = "replays the MID2 Mistweaver against the SimC dummy trace"]
fn mistweaver_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(270, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert_eq!(report["tank_spec"], 66);
    assert!(
        report["tank_casts"]
            .as_object()
            .unwrap()
            .values()
            .any(|count| count.as_u64().unwrap() > 0)
    );
    assert!(report["tank_active_mitigation_seconds"].as_f64().unwrap() > 0.0);
    assert!(
        metrics["first_death_seconds"]
            .as_f64()
            .is_none_or(|time| time > 4.0)
    );
    // Selected native trait entry 124904 replaces Vivify116670 with Sheilun399491.
    let vivify_casts = report["healer_casts"]["399491"].as_u64().unwrap();
    let vivify_raw = metrics["spell_healing"]["399491"]["raw"].as_u64().unwrap();
    let raw_per_cast = vivify_raw as f64 / vivify_casts as f64;
    assert!((1_000.0..500_000.0).contains(&raw_per_cast), "{report}");
    assert!(report["healer_casts"]["107428"].as_u64().unwrap_or(0) > 0);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("MISTWEAVER_REPORT={report}");
}
