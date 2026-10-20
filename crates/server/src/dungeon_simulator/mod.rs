//! Transport-free dungeon qualification, driven by the server test executable.
mod graph;

fn run_first_pack() -> Result<(), String> {
    Err("transport-free one-pack gameplay runner is not implemented".into())
}

#[test]
fn one_pack_runs_real_gameplay_without_transport() {
    run_first_pack().expect("one pack cleared by real gameplay in a plain Bevy App");
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
