use std::path::Path;

#[test]
#[ignore = "replays the approved MID2 Holy Paladin against the SimC dummy trace"]
fn holy_paladin_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(65, Path::new(&trace), 300, 70448).unwrap();
    println!(
        "HOLY_SUMMARY casts={} healing={} death={}",
        report["healer_casts"],
        report["metrics"]["spell_healing"],
        report["metrics"]["first_death_seconds"]
    );
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert_eq!(report["tank_spec"], 66);
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(
        report["tank_casts"]
            .as_object()
            .unwrap()
            .values()
            .any(|x| x.as_u64().unwrap() > 0)
    );
    assert!(
        report["healer_casts"]["20473"].as_u64().unwrap_or(0) > 0,
        "{report}"
    );
    assert!(effective > 0.0 && raw >= effective, "{report}");
    assert!(
        metrics["idle_gcd_pct"].as_f64().unwrap() <= 10.0,
        "Holy Paladin idled over 10%: {report}"
    );
    assert!(
        ["20271", "35395", "24275", "26573"]
            .iter()
            .any(|spell| report["healer_casts"][*spell].as_u64().unwrap_or(0) > 0),
        "Holy Paladin did not cast damage filler: {report}"
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("HOLY_PALADIN_REPORT={report}");
}
