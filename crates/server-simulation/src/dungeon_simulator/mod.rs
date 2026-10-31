//! Transport-free dungeon qualification, driven by the server test executable.
mod addon_tests;
mod batch;
mod campaign;
mod catalogs;
mod chain_heal_target_tests;
mod diagnostics;
mod discipline_priest_replay;
pub(crate) mod dps_parity;
mod flight;
mod friendly_cone_tests;
mod graph;
mod healer_benchmark_tests;
mod healer_coefficient_tests;
pub(crate) use server::test_support::healer_intake;
mod healer_intake_tests;
mod healer_metrics;
mod healer_metrics_tests;
mod healer_party_trace;
mod healer_replay;
mod healer_report;
mod healer_simc_metrics;
mod healer_simc_replay;
mod holy_paladin_replay_tests;
mod holy_paladin_simc;
mod holy_priest_replay;
mod merithra_proc_tests;
mod merithra_replay_tests;
mod mistweaver_replay;
mod movement;
mod preservation_empower_tests;
mod preservation_evoker_replay;
mod replay_cli_tests;
mod restoration_druid_replay;
mod restoration_shaman_replay;
mod runtime;
mod tank_replay;
mod tank_replay_tests;
pub(crate) use graph::Space;

#[test]
fn one_pack_runs_real_gameplay_without_transport() {
    runtime::run_first_pack().expect("one pack cleared by real gameplay in a plain Bevy App");
}

#[test]
fn full_stockade_clears_all_three_bosses() {
    let bosses = runtime::run_stockade(1).expect("full Stockade route completes");
    assert_eq!(bosses, 3, "every Stockade boss must die in the same run");
}

#[test]
fn frozen_stockade_graph_covers_native_formations() {
    let graph = graph::read_graph().expect("frozen Stockade pack graph");
    assert_eq!(graph.map_id, 34);
    let nodes = &graph.packs;
    let data = crate::test_data::world_data();
    let mut expected = std::collections::BTreeMap::<u32, Vec<u32>>::new();
    for &index in &data.spawns_by_map[&34] {
        let spawn = &data.spawns[index];
        if !spawn.spawns_at(crate::maps::store::DIFFICULTY_NORMAL) {
            continue;
        }
        let pack = data
            .formations
            .get(&spawn.guid)
            .map_or(spawn.guid, |formation| formation.leader_guid);
        expected.entry(pack).or_default().push(spawn.guid);
    }
    let actual: std::collections::BTreeMap<u32, Vec<u32>> = nodes
        .iter()
        .map(|node| {
            let mut members: Vec<u32> = node.spawns.iter().map(|spawn| spawn.guid).collect();
            members.sort_unstable();
            (node.id, members)
        })
        .collect();
    for members in expected.values_mut() {
        members.sort_unstable();
    }
    assert_eq!(
        actual, expected,
        "source formations, never proximity clusters"
    );
    assert!(!graph.edges.is_empty());
}
