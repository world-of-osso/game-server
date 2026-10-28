use std::path::Path;
#[test]
#[ignore = "requires MID2 profiles, canonical world database and SimC trace"]
fn holy_priest_heals_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(257, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("NATIVE_HOLY_PRIEST={report}");
}
