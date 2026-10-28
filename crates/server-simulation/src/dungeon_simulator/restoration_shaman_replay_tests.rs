use std::path::Path;

#[test]
#[ignore = "replays the MID2 Restoration Shaman against the SimC dummy trace"]
fn restoration_shaman_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(264, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("RESTORATION_SHAMAN_REPORT={report}");
}
