//! Seeded-run aggregation distinguishes wipes from other failed clears.
use super::campaign::Report;

pub(super) fn summarize(_reports: &[Report]) -> serde_json::Value {
    serde_json::json!({})
}

#[test]
fn batch_summary_counts_wipes_per_reached_pack_and_retains_stalls() {
    let report = |seed, status: &str, failed_pack, packs: &[u32]| Report {
        seed,
        status: status.into(),
        failed_pack,
        bosses_killed: Vec::new(),
        simulated_seconds: 10.0,
        wall_seconds: 1.0,
        detail: String::new(),
        samples: Vec::new(),
        pulls: packs
            .iter()
            .map(|&pack| super::campaign::PullDecision {
                at: 1.0,
                pack,
                decision: "Allow".into(),
            })
            .collect(),
    };
    let reports = [
        report(1, "cleared", None, &[10, 20]),
        report(2, "wiped", Some(20), &[10, 20]),
        report(3, "stalled", Some(10), &[10]),
    ];
    let summary = summarize(&reports);
    assert_eq!(summary["runs"], 3);
    assert_eq!(summary["cleared"], 1);
    assert_eq!(summary["wiped"], 1);
    assert_eq!(
        summary["failed_runs"], 2,
        "no-wipe stalls must not masquerade as clears"
    );
    assert_eq!(summary["packs"]["10"]["runs_reaching"], 3);
    assert_eq!(summary["packs"]["10"]["wipes"], 0);
    assert_eq!(summary["packs"]["10"]["failures"], 1);
    assert_eq!(summary["packs"]["20"]["runs_reaching"], 2);
    assert_eq!(summary["packs"]["20"]["wipe_rate"], 0.5);
}
