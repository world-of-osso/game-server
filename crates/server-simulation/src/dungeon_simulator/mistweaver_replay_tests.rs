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
    assert!(
        metrics["overheal_pct"].as_f64().unwrap() < 76.72,
        "triage must improve on the original 76.73% overheal: {report}"
    );
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
    assert!(
        report["healer_casts"]["115175"].as_u64().unwrap_or(0) > 0,
        "Soothing Mist never cast: {report}"
    );
    assert!(
        metrics["spell_healing"]["115175"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "Soothing Mist did not heal: {report}"
    );
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    assert_eq!(metrics["tank_health_curve"].as_array().unwrap().len(), 301);
    assert!(metrics["tank_dtps"].as_f64().unwrap() > 0.0);
    assert!(metrics["tank_minimum_health_pct"].as_f64().unwrap() < 100.0);
    assert!(metrics["tank_seconds_below_35_alive"].as_f64().unwrap() >= 0.0);
    println!("MISTWEAVER_REPORT={report}");
}
