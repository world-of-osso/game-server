//! Transport-free dungeon qualification, driven by the server test executable.
use std::path::Path;

fn run_first_pack() -> Result<(), String> {
    Err("transport-free one-pack gameplay runner is not implemented".into())
}

#[test]
fn one_pack_runs_real_gameplay_without_transport() {
    run_first_pack().expect("one pack cleared by real gameplay in a plain Bevy App");
}

#[test]
fn frozen_stockade_graph_covers_native_formations() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/dungeons/stockade-34.json");
    let text = std::fs::read_to_string(&path).expect("frozen Stockade pack graph");
    let graph: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(graph["map_id"], 34);
    let nodes = graph["packs"].as_array().unwrap();
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
            let mut members: Vec<u32> = node["spawns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|spawn| spawn["guid"].as_u64().unwrap() as u32)
                .collect();
            members.sort_unstable();
            (node["id"].as_u64().unwrap() as u32, members)
        })
        .collect();
    for members in expected.values_mut() {
        members.sort_unstable();
    }
    assert_eq!(
        actual, expected,
        "source formations, never proximity clusters"
    );
    assert!(!graph["edges"].as_array().unwrap().is_empty());
}
