//! Complete native walk paths for headless ground movement and live movement decisions.
use bevy::prelude::*;
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    path::Path,
    sync::{Mutex, OnceLock},
    time::Instant,
};

#[derive(Resource)]
pub(super) struct NativeMovement {
    state: Mutex<State>,
    measure: bool,
}

struct State {
    map: navigation::NavigationMap,
    stats: BTreeMap<&'static str, (u64, u64, u128)>,
    // NavigationMap loads its tiles lazily; retain the temporary files.
    _arena_tiles: Option<tempfile::TempDir>,
}

/// Install a flat Recast-baked arena through the same Detour walker as dungeon movement.
pub(super) fn install_parity_arena(world: &mut World) -> Result<(), String> {
    static TILES: OnceLock<Result<Vec<((u32, u32), Vec<u8>)>, String>> = OnceLock::new();
    let tiles = TILES
        .get_or_init(bake_parity_arena)
        .as_ref()
        .map_err(Clone::clone)?;
    let directory =
        tempfile::tempdir().map_err(|error| format!("create parity navmesh: {error}"))?;
    for ((row, col), bytes) in tiles {
        let path = directory.path().join(format!("{row}_{col}.nav"));
        std::fs::write(&path, bytes)
            .map_err(|error| format!("write {}: {error}", path.display()))?;
    }
    let map = navigation::NavigationMap::open(directory.path())?;
    world.insert_resource(NativeMovement {
        state: Mutex::new(State {
            map,
            stats: BTreeMap::new(),
            _arena_tiles: Some(directory),
        }),
        measure: std::env::var_os("DUNGEON_SIMULATOR_WALK_STATS").is_some(),
    });
    Ok(())
}

fn bake_parity_arena() -> Result<Vec<((u32, u32), Vec<u8>)>, String> {
    // The origin is on both WoW tile seams. Cover all four neighboring tiles so
    // normal melee, pet and forced movement can cross the origin in any direction.
    let a = Vec3::new(-100.0, 0.0, -100.0);
    let b = Vec3::new(-100.0, 0.0, 100.0);
    let c = Vec3::new(100.0, 0.0, 100.0);
    let d = Vec3::new(100.0, 0.0, -100.0);
    let floor = [[a, b, c], [a, c, d]];
    [(31, 31), (31, 32), (32, 31), (32, 32)]
        .into_iter()
        .map(|tile| navigation::bake::bake_tile(&floor, tile).map(|bytes| (tile, bytes)))
        .collect()
}

impl NativeMovement {
    pub(super) fn open(root: &Path, map_id: u32) -> Result<Self, String> {
        let path = root.join(map_id.to_string());
        let map = navigation::NavigationMap::open(&path)
            .map_err(|error| format!("open native movement map {}: {error}", path.display()))?;
        Ok(Self {
            state: Mutex::new(State {
                map,
                stats: BTreeMap::new(),
                _arena_tiles: None,
            }),
            measure: std::env::var_os("DUNGEON_SIMULATOR_WALK_STATS").is_some(),
        })
    }

    /// Resolve a moving tank's position onto a surface the owner can reach.
    pub(super) fn cohesion_destination(
        &self,
        from: Vec3,
        tank: Vec3,
        range: f32,
    ) -> Result<Vec3, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "native movement map lock poisoned")?;
        let mut radius = 0.0_f32;
        loop {
            let samples = if radius == 0.0 { 1 } else { 16 };
            for sample in 0..samples {
                let angle = sample as f32 * std::f32::consts::TAU / samples as f32;
                let candidate = tank + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
                let Some(surface) = state.map.floor_polygon(candidate)? else {
                    continue;
                };
                if surface.distance(tank) > range {
                    continue;
                }
                match complete_walk(&mut state.map, from, surface) {
                    Ok(_) => return Ok(surface),
                    Err(error) if error.starts_with("unreachable target") => continue,
                    Err(error) => return Err(error),
                }
            }
            if radius >= range {
                break;
            }
            radius = (radius + 2.0).min(range);
        }
        Err(format!(
            "no reachable cohesion approach from {from:?} toward {tank:?} within {range} yards"
        ))
    }

    pub(super) fn walk_legs(
        &self,
        from: Vec3,
        to: Vec3,
        label: &'static str,
    ) -> Result<VecDeque<(Vec3, f32)>, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "native movement map lock poisoned")?;
        let start = self.measure.then(Instant::now);
        let result = complete_walk(&mut state.map, from, to);
        if let Some(start) = start {
            let entry = state.stats.entry(label).or_default();
            entry.0 += 1;
            entry.1 += u64::from(result.is_err());
            entry.2 += start.elapsed().as_nanos();
        }
        let points = result.map_err(|error| format!("native walk unreachable: {error}"))?;
        let mut previous = from;
        Ok(points
            .into_iter()
            .map(|point| {
                let distance = previous.distance(point);
                previous = point;
                (point, distance)
            })
            .collect())
    }
}

fn complete_walk(
    map: &mut navigation::NavigationMap,
    mut from: Vec3,
    to: Vec3,
) -> Result<Vec<Vec3>, String> {
    let mut plan = navigation::NavigationPlan::default();
    let mut endpoints = HashSet::new();
    let mut points = Vec::new();
    loop {
        let segment = map.walking_path_with_plan(from, to, &mut plan)?;
        let endpoint = *segment.points.last().ok_or("empty navigation segment")?;
        points.extend(segment.points);
        if segment.complete {
            return Ok(points);
        }
        if endpoint == from || !endpoints.insert(endpoint.to_array().map(f32::to_bits)) {
            return Err(
                "unreachable target (partial corridor made no progress or repeated)".into(),
            );
        }
        from = endpoint;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn murder_row_cohesion_reaches_near_an_off_mesh_tank_around_a_wall() {
        let root = super::super::graph::extraction::asset_root().join("navmesh");
        let movement = NativeMovement::open(&root, 2813).unwrap();
        let from = Vec3::new(8959.077, 22.708261, 4785.4277);
        let tank = Vec3::new(8955.481, 20.498585, 4835.8545);
        let destination = movement.cohesion_destination(from, tank, 10.0).unwrap();
        assert!(destination.distance(tank) <= 10.0, "{destination:?}");
        assert!(
            movement
                .walk_legs(from, destination, "cohesion-test")
                .unwrap()
                .len()
                > 1
        );
    }

    #[test]
    #[ignore = "requires map 2923 baked navmesh"]
    fn voidscar_portal_walk_assembles_partial_corridors() {
        let root = std::path::PathBuf::from(std::env::var_os("GAME_SERVER_GROUND_DIR").unwrap());
        let movement = NativeMovement::open(&root.join("navmesh"), 2923).unwrap();
        let start = Vec3::new(4539.897, -156.55693, 437.53464);
        let portal = Vec3::new(4615.12, -103.6306, 430.05);
        let legs = movement.walk_legs(start, portal, "voidscar").unwrap();
        assert!(legs.len() > 3);
        assert!(legs.back().unwrap().0.distance(portal) < 0.01);
    }
}

impl Drop for NativeMovement {
    fn drop(&mut self) {
        if self.measure
            && let Ok(state) = self.state.get_mut()
        {
            for (label, (queries, failures, elapsed_ns)) in &state.stats {
                eprintln!(
                    "NATIVE_WALK_STATS label={label} queries={queries} failures={failures} elapsed_ns={elapsed_ns}"
                );
            }
        }
    }
}
