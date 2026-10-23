//! Spec103 fixture proof; production dungeon control remains outside this adapter.
use super::*;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec103 exact profile/traits"]
fn feral_literal_fixture_records_native_bleeds_for_300s() {
    let count: usize = std::env::var("DPS_PARITY_TARGETS")
        .expect("DPS_PARITY_TARGETS")
        .parse()
        .expect("numeric target count");
    assert!(matches!(count, 1 | 3));
    let report = measure(103, count).expect("native Feral fixture");
    let output = inputs::directory()
        .unwrap()
        .join(format!("103.feral-literal.{count}.json"));
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    assert_eq!(report["driver"], "LITERAL_SIMC_FERAL_76f8118");
    assert!((report["simulated_seconds"].as_f64().unwrap() - 300.0).abs() < 0.001);
    assert!(report["combat"]["dps"].as_f64().unwrap() > 0.0);
}
