use super::*;

#[test]
fn parses_explicit_dungeon_key_and_five_spec_party() {
    let options = Options::parse("2515", "2", "66,105,262,63,253").unwrap();
    assert_eq!(
        (options.map_id, options.level, options.specs),
        (2515, 2, [66, 105, 262, 63, 253])
    );
    assert!(Options::parse("2515", "0", "66,105,262,63,253").is_err());
    assert!(Options::parse("2515", "2", "66,105").is_err());
    assert!(Options::parse("bad", "2", "66,105,262,63,253").is_err());
}

#[test]
fn unsupported_dungeon_produces_an_explicit_incomplete_record() {
    let graph = super::super::graph::read_graph().unwrap();
    let options = Options {
        map_id: graph.map_id + 1,
        level: 2,
        specs: [66, 105, 262, 63, 253],
    };
    let record = super::super::campaign::run_keystone(7, options);
    assert_eq!(record.outcome.status, "aborted");
    assert!(record.outcome.reason.contains("does not match graph"));
    assert_eq!(record.map_id, options.map_id);
    assert_eq!(record.requested_specs, options.specs);
    assert!(record.route.is_none());
    assert!(record.outcome.timer_ms.is_none());
}

#[test]
#[ignore = "explicit full native headless keystone run; use scripts/run-keystone-simulator.py"]
fn run_headless_keystone() {
    let options = Options::from_environment().expect("explicit keystone runner inputs");
    let seed: u64 = std::env::var("KEYSTONE_SIMULATOR_SEED")
        .expect("keystone seed")
        .parse()
        .expect("numeric keystone seed");
    let output = std::path::PathBuf::from(
        std::env::var_os("KEYSTONE_SIMULATOR_OUTPUT").expect("keystone record path"),
    );
    let record = super::super::campaign::run_keystone(seed, options);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).expect("create record directory");
    }
    std::fs::write(
        &output,
        serde_json::to_vec_pretty(&record).expect("serialize keystone record"),
    )
    .expect("write keystone record");
    println!(
        "KEYSTONE status={} map={} key={} record={} reason={}",
        record.outcome.status,
        record.map_id,
        record.key_level,
        output.display(),
        record.outcome.reason
    );
    assert_eq!(
        record.outcome.status, "completed",
        "native keystone did not complete; inspect the record"
    );
}
