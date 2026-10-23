//! Source-qualified generic-map graph and terminal-run proof; explicit environment only.
#[test]
#[ignore = "explicit Ruby2521/Halls2527 source topology and terminal qualification"]
fn ruby_or_halls_native_graph_runs_to_a_terminal_state() {
    let selected: u16 = std::env::var("DUNGEON_SIMULATOR_MAP")
        .expect("select Ruby2521 or Halls2527")
        .parse()
        .unwrap();
    assert!([2521, 2527].contains(&selected));
    let graph = super::graph::read_graph().expect("source-qualified follower topology");
    assert_eq!(graph.map_id, selected);
    assert_eq!(graph.difficulty_id(), 205);
    assert!(!graph.boss_entries().is_empty());
    assert!(!graph.route.is_empty());
    let report = super::campaign::run(1).expect("native source-map simulation initialization");
    super::runtime::write_report(&report).expect("retain actual terminal trace");
    assert!(
        ["cleared", "wiped", "blocked", "stalled", "timeout"].contains(&report.status.as_str()),
        "unexpected source-map terminal status {}: {}",
        report.status,
        report.detail
    );
}
