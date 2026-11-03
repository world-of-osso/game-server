//! Complete native walk paths for headless ground movement and live movement decisions.
use bevy::prelude::*;
use std::{
    collections::{BTreeMap, VecDeque},
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
        let result = state.map.complete_walk_path(from, to);
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
