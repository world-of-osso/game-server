//! Transport-free dungeon qualification, driven by the server test executable.
mod batch;
mod campaign;
mod diagnostics;
mod graph;
mod movement;
mod runtime;
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
