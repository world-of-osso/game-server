//! Actual native reports must expose every seed and their arithmetic mean.
use super::*;

#[test]
#[ignore = "native Ret300s report shape and multi-seed mean require private parity inputs"]
fn dps_parity_ret_multiseed_report_matches_individual_damage_rates() {
    let report = measure(70, 1).expect("native Ret seed measurements");
    let seeds = report["seed"]
        .as_array()
        .expect("report.seed must list the measured seeds");
    assert!(!seeds.is_empty());
    if std::env::var_os("DPS_PARITY_SEEDS").is_none() {
        assert_eq!(report["seed"], json!([70448, 1, 2]));
    }
    let samples = report["seed_dps"]
        .as_array()
        .expect("each measured seed must retain its actual DPS");
    assert_eq!(samples.len(), seeds.len());
    let mut total = 0.0;
    for (seed, sample) in seeds.iter().zip(samples) {
        assert_eq!(&sample["seed"], seed);
        let dps = sample["dps"].as_f64().expect("numeric native seed DPS");
        assert!(dps > 0.0, "each seed must deal real damage");
        total += dps;
    }
    let mean = total / samples.len() as f64;
    let headline = report["combat"]["dps"].as_f64().unwrap();
    assert!(
        (headline - mean).abs() < 1e-6,
        "headline{headline} must be the actual seed mean{mean}"
    );
}
