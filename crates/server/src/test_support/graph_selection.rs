//! Source-backed simulator dispatch without a compiled map-name registry.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Deserialize)]
pub(super) struct Requirements {
    pub difficulty_id: u32,
    pub boss_order: Vec<u32>,
}

pub(super) fn requirements(map_id: u32) -> Result<&'static Requirements, String> {
    static REQUIREMENTS: OnceLock<BTreeMap<u32, Requirements>> = OnceLock::new();
    let sources = REQUIREMENTS.get_or_init(|| {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../content/dungeons/route-requirements.json"
        )))
        .expect("source route requirements must be valid JSON")
    });
    let source = sources
        .get(&map_id)
        .ok_or_else(|| format!("map{map_id} lacks source difficulty/boss requirements"))?;
    if source.difficulty_id == 0 || source.boss_order.is_empty() {
        return Err(format!(
            "map{map_id} requires an explicit difficulty and boss roster"
        ));
    }
    Ok(source)
}

fn graph_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/dungeons")
}

fn read_graph_paths(directory: &Path) -> Result<Vec<PathBuf>, String> {
    std::fs::read_dir(directory)
        .map_err(|error| {
            format!(
                "read frozen graph directory {}: {error}",
                directory.display()
            )
        })?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read frozen graph entry: {error}"))
}

fn find_graph_path(directory: &Path, map_id: u32) -> Result<PathBuf, String> {
    let suffix = format!("-{map_id}.json");
    let matches: Vec<_> = read_graph_paths(directory)?
        .into_iter()
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().ends_with(&suffix))
        })
        .collect();
    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => Err(format!(
            "map{map_id} has no frozen graph ending in {suffix}"
        )),
        _ => Err(format!(
            "map{map_id} has ambiguous frozen graphs: {matches:?}"
        )),
    }
}

pub(super) fn read_selected_graph() -> Result<(u32, PathBuf), String> {
    let selected = match std::env::var("DUNGEON_SIMULATOR_MAP") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => "34".into(),
        Err(error) => return Err(format!("read DUNGEON_SIMULATOR_MAP: {error}")),
    };
    let map_id = selected
        .parse::<u32>()
        .map_err(|error| format!("invalid DUNGEON_SIMULATOR_MAP={selected}: {error}"))?;
    let path = find_graph_path(&graph_directory(), map_id)?;
    Ok((map_id, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatches_existing_and_new_graphs_from_source_data() {
        for (map_id, difficulty, count) in [(34, 1, 3), (2515, 205, 4), (2451, 205, 7)] {
            let path = find_graph_path(&graph_directory(), map_id).unwrap();
            let graph: super::super::Graph =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            assert_eq!(graph.map_id, map_id);
            assert_eq!(graph.difficulty_id(), difficulty);
            assert_eq!(graph.boss_entries().len(), count);
        }
        assert!(requirements(0).is_err());
    }

    #[test]
    fn accepts_arbitrary_names_and_rejects_missing_or_ambiguous_graphs() {
        let directory =
            std::env::temp_dir().join(format!("frozen-graphs-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        assert!(find_graph_path(&directory, 2451).is_err());
        let source = find_graph_path(&graph_directory(), 2451).unwrap();
        let first = directory.join("different-name-2451.json");
        std::fs::copy(&source, &first).unwrap();
        assert_eq!(find_graph_path(&directory, 2451).unwrap(), first);
        std::fs::copy(&source, directory.join("another-name-2451.json")).unwrap();
        assert!(find_graph_path(&directory, 2451).is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
