//! Versioned portal graph artifacts bound to the exact navmesh tile bytes.
use std::io;
use std::path::Path;

use super::PortalGraph;

const VERSION: u32 = 1;
const GRAPH_FILE: &str = "portal-graph.json";

impl PortalGraph {
    /// Write an offline graph only after validating it and hashing every source tile.
    pub fn write_artifact(&self, directory: &Path) -> Result<(), String> {
        let mut graph = self.clone();
        graph.version = VERSION;
        graph.tile_digest = tile_digest(directory)?;
        graph.validate()?;
        let bytes =
            serde_json::to_vec(&graph).map_err(|error| format!("encode portal graph: {error}"))?;
        let output = directory.join(GRAPH_FILE);
        let temporary = directory.join("portal-graph.json.tmp");
        std::fs::write(&temporary, bytes)
            .map_err(|error| format!("{}: {error}", temporary.display()))?;
        std::fs::rename(&temporary, &output)
            .map_err(|error| format!("{}: {error}", output.display()))
    }

    /// Read a graph only when its format, topology, and tile digest match.
    pub fn read_artifact(directory: &Path) -> Result<Self, String> {
        let path = directory.join(GRAPH_FILE);
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let graph: Self = serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if graph.version != VERSION {
            return Err(format!(
                "{}: unsupported graph version {}",
                path.display(),
                graph.version
            ));
        }
        if graph.tile_digest != tile_digest(directory)? {
            return Err(format!("{}: navmesh tile digest mismatch", path.display()));
        }
        graph.validate()?;
        Ok(graph)
    }

    fn validate(&self) -> Result<(), String> {
        for node in &self.nodes {
            if node.cluster as usize >= self.clusters.len()
                || node.position.iter().any(|value| !value.is_finite())
            {
                return Err("invalid portal graph node".into());
            }
            for edge in &node.edges {
                if edge.destination as usize >= self.nodes.len()
                    || !edge.cost.is_finite()
                    || edge.cost < 0.0
                {
                    return Err("invalid portal graph edge".into());
                }
            }
        }
        Ok(())
    }
}

fn tile_digest(directory: &Path) -> Result<[u8; 32], String> {
    let mut paths = std::fs::read_dir(directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "nav"));
    paths.sort();
    if paths.is_empty() {
        return Err(format!("{}: no navmesh tiles", directory.display()));
    }
    let mut digest = blake3::Hasher::new();
    for path in paths {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid navmesh tile filename")?;
        digest.update(&(name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        let mut file =
            std::fs::File::open(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let length = file
            .metadata()
            .map_err(|error| format!("{}: {error}", path.display()))?
            .len();
        digest.update(&length.to_le_bytes());
        io::copy(&mut file, &mut digest).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(*digest.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::{PolygonKey, PortalEdge, PortalNode};

    #[test]
    fn graph_artifact_rejects_changed_and_missing_tiles() {
        let dir = tempfile::tempdir().unwrap();
        let tile = dir.path().join("32_32.nav");
        std::fs::write(&tile, b"original tile").unwrap();
        let key = PolygonKey {
            tile_x: 0,
            tile_z: 0,
            layer: 0,
            polygon: 2,
        };
        let graph = PortalGraph {
            clusters: vec![key],
            nodes: vec![PortalNode {
                polygon: key,
                cluster: 0,
                position: [1.0, 2.0, 3.0],
                edges: vec![PortalEdge {
                    destination: 0,
                    cost: 0.0,
                }],
            }],
            ..Default::default()
        };
        graph.write_artifact(dir.path()).unwrap();
        let loaded = PortalGraph::read_artifact(dir.path()).unwrap();
        assert_eq!(loaded.nodes[0].position, [1.0, 2.0, 3.0]);
        std::fs::write(&tile, b"changed tile").unwrap();
        assert!(
            PortalGraph::read_artifact(dir.path())
                .unwrap_err()
                .contains("digest mismatch")
        );
        std::fs::remove_file(tile).unwrap();
        assert!(PortalGraph::read_artifact(dir.path()).is_err());
    }
}
