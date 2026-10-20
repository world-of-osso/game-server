//! Seeded-run aggregation distinguishes wipes from other failed clears.
use super::campaign::Report;

pub(super) fn summarize(reports: &[Report]) -> serde_json::Value {
    let mut packs = std::collections::BTreeMap::<u32, (usize, usize, usize)>::new();
    for report in reports {
        let reached: std::collections::BTreeSet<_> = report
            .pulls
            .iter()
            .map(|pull| pull.pack)
            .chain(report.failed_pack)
            .collect();
        for pack in reached {
            let counts = packs.entry(pack).or_default();
            counts.0 += 1;
            counts.1 += usize::from(report.failed_pack == Some(pack) && report.status == "wiped");
            counts.2 += usize::from(report.failed_pack == Some(pack) && report.status != "cleared");
        }
    }
    let packs: std::collections::BTreeMap<_, _> = packs.into_iter().map(|(pack, (reaching, wipes, failures))| {
        (pack, serde_json::json!({"runs_reaching": reaching, "wipes": wipes, "failures": failures,
            "wipe_rate": wipes as f64 / reaching as f64, "failure_rate": failures as f64 / reaching as f64}))
    }).collect();
    let count = |status| {
        reports
            .iter()
            .filter(|report| report.status == status)
            .count()
    };
    serde_json::json!({"runs": reports.len(), "seeds": reports.iter().map(|report| report.seed).collect::<Vec<_>>(),
        "cleared": count("cleared"), "wiped": count("wiped"), "stalled": count("stalled"),
        "blocked": count("blocked"), "timed_out": count("timeout"),
        "failed_runs": reports.len() - count("cleared"), "packs": packs})
}

#[test]
#[ignore = "explicit seeded dungeon batch; execute through scripts/run-dungeon-simulator.py"]
fn run_stockade_batch() {
    let runs: u64 = std::env::var("DUNGEON_SIMULATOR_RUNS")
        .expect("batch run count")
        .parse()
        .expect("positive run count");
    let seed: u64 = std::env::var("DUNGEON_SIMULATOR_SEED")
        .expect("batch initial seed")
        .parse()
        .expect("u64 seed");
    assert!(runs > 0, "batch run count must be positive");
    let mut reports = Vec::new();
    let mut catalogs = None;
    for offset in 0..runs {
        let report = super::campaign::run_cached(
            seed.checked_add(offset).expect("seed range exceeds u64"),
            &mut catalogs,
        )
        .expect("initialize native dungeon simulation");
        super::runtime::write_report(&report).expect("write actual run trace");
        super::runtime::print_report(&report);
        reports.push(report);
    }
    let summary = summarize(&reports);
    let path = super::runtime::output_directory().join("summary.json");
    std::fs::write(&path, serde_json::to_vec_pretty(&summary).unwrap())
        .expect("write seeded batch summary");
    println!("BATCH {}", serde_json::to_string(&summary).unwrap());
    assert_eq!(
        summary["cleared"], runs,
        "batch completed, but not every dungeon cleared; inspect summary.json"
    );
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
        performance: serde_json::Value::Null,
        terminal_units: serde_json::Value::Null,
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
