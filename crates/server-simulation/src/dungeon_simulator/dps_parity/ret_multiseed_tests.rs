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
    let mut expected_actions = std::collections::BTreeMap::<Option<u64>, (f64, f64)>::new();
    for (seed, sample) in seeds.iter().zip(samples) {
        assert_eq!(&sample["seed"], seed);
        let dps = sample["dps"].as_f64().expect("numeric native seed DPS");
        assert!(dps > 0.0, "each seed must deal real damage");
        total += dps;
        let path = inputs::directory()
            .unwrap()
            .join(format!("70.native.1.seed-{}.json", seed.as_u64().unwrap()));
        let individual: Value =
            serde_json::from_slice(&std::fs::read(path).expect("full individual seed report"))
                .unwrap();
        assert_eq!(individual["seed"], *seed);
        assert_eq!(individual["combat"]["dps"], sample["dps"]);
        for action in individual["combat"]["actions"].as_array().unwrap() {
            let totals = expected_actions
                .entry(action["spell_id"].as_u64())
                .or_default();
            totals.0 += action["damage"].as_f64().unwrap();
            totals.1 += action["hits"].as_f64().unwrap();
        }
    }
    let mean = total / samples.len() as f64;
    let headline = report["combat"]["dps"].as_f64().unwrap();
    assert!(
        (headline - mean).abs() < 1e-6,
        "headline{headline} must be the actual seed mean{mean}"
    );
    assert_eq!(report["detail_seed"], seeds[0]);
    let mut observed_actions = std::collections::BTreeMap::<Option<u64>, (f64, f64, f64)>::new();
    for action in report["combat"]["actions"].as_array().unwrap() {
        let totals = observed_actions
            .entry(action["spell_id"].as_u64())
            .or_default();
        totals.0 += action["damage"].as_f64().unwrap();
        totals.1 += action["hits"].as_f64().unwrap();
        totals.2 += action["share_pct"].as_f64().unwrap() * headline / 100.0;
    }
    assert_eq!(observed_actions.len(), expected_actions.len());
    let seconds = report["simulated_seconds"].as_f64().unwrap();
    for (spell, (damage, hits)) in expected_actions {
        let observed = observed_actions[&spell];
        let mean_damage = damage / samples.len() as f64;
        assert!((observed.0 - mean_damage).abs() < 1e-6);
        assert!((observed.1 - hits / samples.len() as f64).abs() < 1e-6);
        assert!(
            (observed.2 - mean_damage / seconds).abs() < 1e-6,
            "gaps.py share-based DPS must match the spell's mean actual damage"
        );
    }
}
