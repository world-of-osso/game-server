//! Server-side player ground: ADT terrain and WMO floors per map, judged by the
//! shared rule the client predicts with (`shared::ground`,
//! game-engine docs/specs/wmo-floor-collision.md).
//!
//! Maps load on first use from the client's asset cache (`GAME_SERVER_GROUND_DIR`, else
//! `game-engine/data` beside this checkout): the
//! map's WDT (`Map.db2` WdtFileDataID) either names one global WMO (MPHD flag 0x1, a
//! WMO-only dungeon such as the Stockade) or, through MAID, each tile's root and
//! `_obj0` ADT FileDataIDs; the `_obj0` MODF places WMOs by FileDataID, and each WMO
//! root's GFID names its group files.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use wow_adt::adt::parse_mfbo_min_heights;
#[cfg(test)]
use wow_adt::adt_obj::MODF_FLAG_HAS_SCALE;
use wow_adt::adt_obj::{MODF_ENTRY_SIZE, MODF_FLAG_FILEDATAID};
use wow_adt::wdt::MPHD_FLAG_GLOBAL_WMO;
use wow_adt::{adt_obj, wdt};
use wow_chunk::find_reversed_chunk as find_chunk;
#[cfg(test)]
use wow_chunk::read_u32;
use wow_wmo::parser::parse_root_group_file_ids as group_file_data_ids;

use bevy::math::{Affine3A, Vec3};
use bevy::prelude::*;
use shared::ground::{
    WmoCollision, WmoGroupCollision, global_wmo_placement_position, placement_position,
    placement_rotation,
};

use crate::liquid::LiquidSample;
use crate::terrain_height::{TerrainHeightMap, parse_adt_heights};
use los::wmo_liquid::LiquidTriangle;

/// Overrides the client asset cache the ground loads from.
const GROUND_DIR_ENV: &str = "GAME_SERVER_GROUND_DIR";
/// `map id<TAB>file` of the spawned maps' ground files local CASC cannot provide
/// (`scripts/extract_ground.py --failed`): the server starts without them.
const KNOWN_GROUND_GAPS: &str = include_str!("ground_gaps.tsv");
/// What `GroundMap::missing_files` reports for a map without a WDT.
const NO_WDT: &str = "no WDT FileDataID";
const TILE_SIZE: f32 = shared::navmesh::TILE_SIZE;
const MAP_TILES: usize = 64;
/// Bounded startup-only filesystem concurrency; never changes ECS scheduling.
const GROUND_VALIDATION_WORKERS: usize = 4;
/// `GridMap::getMinHeight` / `TerrainInfo::GetMinHeight` without flight bounds (no MFBO, or
/// no grid at all as on a WMO-only map).
const DEFAULT_MIN_HEIGHT: f32 = -500.0;
/// TrinityCore GridDefines.h MAX_FALL_DISTANCE, used by MotionMaster::MoveFall.
const MAX_FALL_DISTANCE: f32 = 250_000.0;
/// TrinityCore `SIZE_OF_GRIDS`, `CENTER_GRID_ID`, `MAP_HALFSIZE`.
const SIZE_OF_GRIDS: f32 = 533.3333;
const CENTER_GRID_ID: i32 = 32;
const MAP_HALFSIZE: f32 = 32.0 * SIZE_OF_GRIDS;

/// What supports a player's feet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroundProbe {
    /// No terrain for the tile under the feet: physics leaves the player alone.
    Unloaded,
    /// Nothing within step reach: the player falls.
    Unsupported,
    Supported(f32),
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct Floor {
    pub z: f32,
    pub source: &'static str,
}

#[derive(Clone, Debug)]
pub struct MissingGeometry(pub String);

/// Strict offline geometry reader: missing input never becomes an empty floor set.
pub struct FloorGeometry {
    ground: GroundMap,
    strict_tiles: HashMap<(u32, u32, u32), Result<Vec<WmoCollision>, String>>,
}

impl FloorGeometry {
    pub fn new(data_dir: &Path, wdts: HashMap<u32, u32>) -> Self {
        Self {
            ground: GroundMap::new(data_dir, wdts),
            strict_tiles: HashMap::new(),
        }
    }

    pub fn candidate_floors(
        &mut self,
        map_id: u32,
        x: f32,
        y: f32,
    ) -> Result<Vec<Floor>, MissingGeometry> {
        self.read_candidate_floors(map_id, x, y)
            .map_err(MissingGeometry)
    }

    fn read_candidate_floors(&mut self, map_id: u32, x: f32, y: f32) -> Result<Vec<Floor>, String> {
        let (bx, _, bz) = crate::terrain_height::wow_to_bevy(x, y, 0.0);
        if !self.ground.maps.contains_key(&map_id) {
            let wdt = self
                .ground
                .wdts
                .get(&map_id)
                .ok_or_else(|| format!("map {map_id}: no WDT"))?;
            let map = MapGround::open(&self.ground.data_dir, *wdt)?;
            self.ground.maps.insert(map_id, Some(map));
        }
        let map = self
            .ground
            .maps
            .get_mut(&map_id)
            .and_then(Option::as_mut)
            .ok_or("missing map geometry")?;
        let mut floors = Vec::new();
        if let Some(wmo) = &map.global_wmo {
            append_wmo_floors(&mut floors, std::slice::from_ref(wmo), bx, bz);
        } else {
            let tile_id = bevy_to_tile(bx, bz);
            let tile = map
                .tile(tile_id)
                .ok_or_else(|| format!("map {map_id}: missing terrain tile {tile_id:?}"))?;
            if let Some(z) = tile.terrain.height_at_bevy(bx, bz) {
                floors.push(Floor {
                    z,
                    source: "terrain",
                });
            }
            let nearby = nearby_source_tiles(map, tile_id);
            for coordinates in nearby {
                let key = (map_id, coordinates.0, coordinates.1);
                if !self.strict_tiles.contains_key(&key) {
                    let result = load_strict_tile_wmos(map, coordinates);
                    self.strict_tiles.insert(key, result);
                }
                let wmos = self.strict_tiles[&key].as_ref().map_err(Clone::clone)?;
                append_wmo_floors(&mut floors, wmos, bx, bz);
            }
        }
        let m2_dir = self.ground.data_dir.join(format!("los/{map_id}/m2"));
        let tile_id = bevy_to_tile(bx, bz);
        validate_m2_tiles(map, &m2_dir, tile_id)?;
        let m2 = self
            .ground
            .m2_floors
            .entry(map_id)
            .or_insert_with(|| los::LosMap::new(&m2_dir));
        floors.extend(
            m2.floor_heights(bx, bz)?
                .into_iter()
                .map(|z| Floor { z, source: "m2" }),
        );
        Ok(floors)
    }
}

fn append_wmo_floors(floors: &mut Vec<Floor>, wmos: &[WmoCollision], x: f32, z: f32) {
    let mut heights = Vec::new();
    for wmo in wmos {
        wmo.floor_heights(x, z, 100_000.0, -100_000.0, &mut heights);
    }
    floors.extend(heights.into_iter().map(|z| Floor { z, source: "wmo" }));
}

fn validate_m2_tiles(map: &MapGround, dir: &Path, (row, col): (u32, u32)) -> Result<(), String> {
    for r in row.saturating_sub(1)..=(row + 1).min(63) {
        for c in col.saturating_sub(1)..=(col + 1).min(63) {
            let required = map.source.as_ref().map_or((r, c) == (row, col), |source| {
                source.tiles[c as usize * MAP_TILES + r as usize].root != 0
            });
            let path = dir.join(format!("{r}_{c}.los"));
            if required && !path.is_file() {
                return Err(format!("missing M2 tile {}", path.display()));
            }
        }
    }
    Ok(())
}

fn nearby_source_tiles(map: &MapGround, (row, col): (u32, u32)) -> Vec<(u32, u32)> {
    let source = map.source.as_ref().expect("terrain map has a tile source");
    let mut tiles = Vec::new();
    for r in row.saturating_sub(1)..=(row + 1).min(63) {
        for c in col.saturating_sub(1)..=(col + 1).min(63) {
            if source.tiles[c as usize * MAP_TILES + r as usize].obj0 != 0 {
                tiles.push((r, c));
            }
        }
    }
    tiles
}

fn load_strict_tile_wmos(
    map: &mut MapGround,
    (row, col): (u32, u32),
) -> Result<Vec<WmoCollision>, String> {
    let source = map.source.as_ref().ok_or("missing tile source")?;
    let files = source.tiles[col as usize * MAP_TILES + row as usize];
    let data_dir = source.data_dir.clone();
    let data = read_client_file(&data_dir, "terrain", files.obj0, "adt")?;
    let Some(modf) = find_chunk(&data, b"MODF")? else {
        return Ok(Vec::new());
    };
    if modf.len() % MODF_ENTRY_SIZE != 0 {
        return Err("truncated MODF".into());
    }
    modf.chunks_exact(MODF_ENTRY_SIZE)
        .map(|entry| {
            let placement = parse_modf_entry(entry)?;
            if !map.wmo_models.contains_key(&placement.fdid) {
                map.wmo_models
                    .insert(placement.fdid, load_wmo_groups(&data_dir, placement.fdid)?);
            }
            Ok(WmoCollision::new(
                placement.world_from_local(),
                map.wmo_models[&placement.fdid].clone(),
            ))
        })
        .collect()
}

struct TileGround {
    terrain: TerrainHeightMap,
    wmos: Option<Vec<WmoCollision>>,
    liquids: Option<Vec<LiquidTriangle>>,
    /// Root ADT MFBO minimum corner heights (flight bounds), in file order.
    flight_min: Option<[i16; 9]>,
}

/// Per-tile ADT FileDataIDs from the WDT MAID.
#[derive(Clone, Copy, Default)]
struct TileFiles {
    root: u32,
    obj0: u32,
}

struct TileSource {
    data_dir: PathBuf,
    /// Indexed `col * 64 + row`, as `(row, col)` tiles are.
    tiles: Vec<TileFiles>,
}

/// Ground geometry of every map players have visited.
#[derive(Resource, Default)]
pub struct GroundMap {
    /// Explicit qualification-arena geometry, not a missing-map fallback.
    flat_arena_height: Option<f32>,
    data_dir: PathBuf,
    /// `Map.db2` WdtFileDataID by map id.
    wdts: HashMap<u32, u32>,
    /// `None`: the map's WDT is not available.
    maps: HashMap<u32, Option<MapGround>>,
    /// Dedicated authored M2 hull bake, cached independently of ADT/WMO ground.
    m2_floors: HashMap<u32, los::LosMap>,
    m2_solids: HashMap<u32, los::LosMap>,
    m2_solid_checks: HashMap<u32, Result<(), String>>,
    live_hulls: crate::game_objects::collision::LiveHulls,
    #[cfg(test)]
    entry_gate: Option<std::sync::mpsc::Sender<std::sync::mpsc::Sender<()>>>,
}

impl GroundMap {
    /// A declared flat encounter floor, independent of production map assets.
    pub fn flat_arena(height: f32) -> Self {
        assert!(height.is_finite(), "arena floor must be finite");
        Self {
            flat_arena_height: Some(height),
            ..default()
        }
    }

    /// Maps load lazily from `data_dir` (`terrain/<fdid>.wdt|adt`, `models/<fdid>.wmo`).
    pub fn new(data_dir: &Path, wdts: HashMap<u32, u32>) -> Self {
        Self {
            flat_arena_height: None,
            data_dir: data_dir.to_path_buf(),
            wdts,
            maps: HashMap::new(),
            m2_floors: HashMap::new(),
            m2_solids: HashMap::new(),
            m2_solid_checks: HashMap::new(),
            live_hulls: HashMap::new(),
            #[cfg(test)]
            entry_gate: None,
        }
    }

    /// A map 0 holding only `terrain` for `tile`, without WMOs or lazy loading.
    #[cfg(test)]
    pub(crate) fn with_terrain(tile: (u32, u32), terrain: TerrainHeightMap) -> Self {
        Self::with_terrain_on(0, tile, terrain)
    }

    #[cfg(test)]
    pub(crate) fn with_terrain_on(
        map_id: u32,
        tile: (u32, u32),
        terrain: TerrainHeightMap,
    ) -> Self {
        let mut map = MapGround::default();
        map.tiles.insert(
            tile,
            Some(TileGround {
                terrain,
                wmos: Some(Vec::new()),
                liquids: Some(Vec::new()),
                flight_min: None,
            }),
        );
        Self {
            maps: HashMap::from([(map_id, Some(map))]),
            ..default()
        }
    }

    /// Explicit terrain-area fixtures grouped by actual map/tile; no production probe override.
    #[cfg(test)]
    pub(crate) fn with_area_samples(samples: &[(u32, Vec3, u32)]) -> Self {
        let mut tiles: HashMap<(u32, (u32, u32)), Vec<(f32, f32, u32)>> = HashMap::new();
        for &(map, position, area) in samples {
            let tile = bevy_to_tile(position.x, position.z);
            let (x, y, _) = crate::terrain_height::bevy_to_wow(position.x, position.y, position.z);
            tiles.entry((map, tile)).or_default().push((x, y, area));
        }
        let mut ground = Self::default();
        for ((map, tile), points) in tiles {
            let terrain = crate::terrain_height::test_helpers::terrain_areas(&points);
            let map = ground
                .maps
                .entry(map)
                .or_insert_with(|| Some(MapGround::default()));
            map.as_mut().unwrap().tiles.insert(
                tile,
                Some(TileGround {
                    terrain,
                    wmos: Some(Vec::new()),
                    liquids: Some(Vec::new()),
                    flight_min: None,
                }),
            );
        }
        ground
    }

    pub(crate) fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Prepare the entry position's terrain and collision caches without borrowing the live world.
    pub(crate) fn prepare_entry(
        &self,
        map_id: u32,
        feet: Vec3,
    ) -> std::sync::mpsc::Receiver<GroundMap> {
        let mut prepared = Self::new(&self.data_dir, self.wdts.clone());
        prepared.flat_arena_height = self.flat_arena_height;
        #[cfg(test)]
        let gate = self.entry_gate.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("entry-ground".into())
            .spawn(move || {
                #[cfg(test)]
                if let Some(gate) = gate {
                    let (release, blocked) = std::sync::mpsc::channel();
                    if gate.send(release).is_ok() {
                        blocked
                            .recv_timeout(std::time::Duration::from_secs(30))
                            .expect("release entry ground worker");
                    }
                }
                prepared.liquid_at(map_id, feet);
                prepared.ground_at(map_id, feet);
                prepared.min_height(map_id, feet);
                // A disconnected client no longer needs its prepared ground.
                let _ = sender.send(prepared);
            })
            .expect("start entry ground worker");
        receiver
    }

    #[cfg(test)]
    pub(crate) fn gate_entry_preparation(
        &mut self,
    ) -> std::sync::mpsc::Receiver<std::sync::mpsc::Sender<()>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        self.entry_gate = Some(sender);
        receiver
    }

    /// Merge worker-loaded tiles without discarding ground already used by other players.
    pub(crate) fn merge_entry_ground(&mut self, prepared: GroundMap) {
        for (map_id, incoming) in prepared.maps {
            match (self.maps.entry(map_id).or_insert(None), incoming) {
                (slot @ None, incoming) => *slot = incoming,
                (Some(current), Some(incoming)) => {
                    for (tile, loaded) in incoming.tiles {
                        let existing = current.tiles.entry(tile).or_insert(None);
                        if loaded.is_some()
                            && existing.as_ref().is_none_or(|tile| tile.wmos.is_none())
                        {
                            *existing = loaded;
                        }
                    }
                    current.wmo_models.extend(incoming.wmo_models);
                }
                (Some(_), None) => {}
            }
        }
        for (map_id, incoming) in prepared.m2_floors {
            if let Some(current) = self.m2_floors.get_mut(&map_id) {
                current.merge_preloaded(incoming);
            } else {
                self.m2_floors.insert(map_id, incoming);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn live_hull_count(&self, map: crate::maps::InMap) -> usize {
        self.live_hulls.get(&map).map_or(0, Vec::len)
    }

    pub(crate) fn set_live_hulls(&mut self, hulls: crate::game_objects::collision::LiveHulls) {
        self.live_hulls = hulls;
    }

    /// Static ground plus enabled live GO collision in this exact map copy.
    pub(crate) fn ground_in(&mut self, map: crate::maps::InMap, feet: Vec3) -> GroundProbe {
        let base = self.ground_at(map.map_id, feet);
        self.add_live_height(map, feet, base)
    }

    /// Creature movement uses map collision height, not player slope admission.
    pub(crate) fn creature_ground_in(
        &mut self,
        map: crate::maps::InMap,
        feet: Vec3,
    ) -> GroundProbe {
        let base = self.creature_ground_at(map.map_id, feet);
        self.add_live_height(map, feet, base)
    }

    /// MoveFall searches beyond the ordinary ground-snap depth.
    pub(crate) fn creature_fall_ground_in(
        &mut self,
        map: crate::maps::InMap,
        feet: Vec3,
    ) -> GroundProbe {
        if let Some(height) = self.flat_arena_height {
            return GroundProbe::Supported(height);
        }
        if !self.maps.contains_key(&map.map_id) {
            let loaded = self.open_map(map.map_id);
            self.maps.insert(map.map_id, loaded);
        }
        let base = match self.maps.get_mut(&map.map_id) {
            Some(Some(ground)) => ground.fall_ground_at(feet),
            _ => GroundProbe::Unloaded,
        };
        let m2 = self
            .m2_floors
            .entry(map.map_id)
            .or_insert_with(|| {
                los::LosMap::new(&self.data_dir.join(format!("los/{}/m2", map.map_id)))
            })
            .floor_heights(feet.x, feet.z);
        let heights = match m2 {
            Ok(heights) => heights,
            Err(error) => {
                error!("M2 fall-floor query on map {}: {error}", map.map_id);
                return GroundProbe::Unsupported;
            }
        };
        let live = self
            .live_hulls
            .get(&map)
            .into_iter()
            .flatten()
            .flat_map(|hull| hull.floor_heights(feet.x, feet.z));
        select_fall_height(feet, base, heights.into_iter().chain(live))
    }

    pub(crate) fn player_solid_ground_in(
        &mut self,
        map: crate::maps::InMap,
        feet: Vec3,
    ) -> GroundProbe {
        let center = self.player_solid_point(map, feet);
        let tolerance = shared::movement::GROUND_SNAP_THRESHOLD;
        if matches!(center, GroundProbe::Supported(height) if height >= feet.y - tolerance)
            || matches!(center, GroundProbe::Unloaded)
        {
            return center;
        }
        let radius = shared::los::body::BODY_RADIUS;
        [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
            .into_iter()
            .filter_map(
                |direction| match self.player_solid_point(map, feet + direction * radius) {
                    GroundProbe::Supported(height) if (height - feet.y).abs() <= tolerance => {
                        Some(height)
                    }
                    _ => None,
                },
            )
            .max_by(f32::total_cmp)
            .map_or(center, |height| GroundProbe::Supported(height.min(feet.y)))
    }

    fn player_solid_point(&mut self, map: crate::maps::InMap, feet: Vec3) -> GroundProbe {
        let base = self.sample_ground_at(map.map_id, feet, false, true);
        self.add_live_height(map, feet, base)
    }

    fn add_live_height(
        &self,
        map: crate::maps::InMap,
        feet: Vec3,
        base: GroundProbe,
    ) -> GroundProbe {
        let live = self
            .live_hulls
            .get(&map)
            .into_iter()
            .flatten()
            .filter_map(|hull| hull.floor_height(feet))
            .max_by(f32::total_cmp);
        match (base, live) {
            (GroundProbe::Supported(base), Some(live)) => GroundProbe::Supported(base.max(live)),
            (_, Some(live)) => GroundProbe::Supported(live),
            (base, None) => base,
        }
    }

    pub(crate) fn live_clear(&self, map: crate::maps::InMap, start: Vec3, end: Vec3) -> bool {
        self.live_hulls
            .get(&map)
            .into_iter()
            .flatten()
            .all(|hull| !hull.blocks(start, end))
    }

    /// Clamp a movement report against live walls, using the realm's 0.5yd clearance.
    pub(crate) fn clamp_live_movement(
        &self,
        map: crate::maps::InMap,
        start: Vec3,
        end: Vec3,
    ) -> Vec3 {
        let offset = Vec3::Y;
        if self.live_clear(map, start + offset, end + offset) {
            return end;
        }
        let (mut clear, mut blocked) = (0.0, 1.0);
        for _ in 0..16 {
            let fraction = (clear + blocked) * 0.5;
            if self.live_clear(map, start + offset, start.lerp(end, fraction) + offset) {
                clear = fraction;
            } else {
                blocked = fraction;
            }
        }
        let distance = start.distance(end);
        start.lerp(end, (clear - 0.5 / distance).max(0.0))
    }

    /// The ground files of map `map_id` the cache does not hold, relative to it
    /// (`terrain/<fdid>.wdt|adt`, `models/<fdid>.wmo`, with why when one does not parse):
    /// its WDT, then each MAID tile's root and `_obj0` ADT and the WMO roots and groups
    /// the WDT or the `_obj0` MODF place. `NO_WDT` for a map without one.
    #[cfg(any(test, feature = "test-support"))]
    pub fn missing_files(&self, map_id: u32) -> Vec<String> {
        missing_ground_files(&self.data_dir, self.wdts.get(&map_id).copied())
    }

    /// Ground under `feet` (Bevy space) on map `map_id`, loading the map and its tile on
    /// first use.
    pub fn ground_at(&mut self, map_id: u32, feet: Vec3) -> GroundProbe {
        self.sample_ground_at(map_id, feet, true, false)
    }

    pub(crate) fn creature_ground_at(&mut self, map_id: u32, feet: Vec3) -> GroundProbe {
        self.sample_ground_at(map_id, feet, false, false)
    }

    fn sample_ground_at(
        &mut self,
        map_id: u32,
        feet: Vec3,
        walkable_only: bool,
        solid_m2: bool,
    ) -> GroundProbe {
        if let Some(height) = self.flat_arena_height {
            return GroundProbe::Supported(height);
        }
        if !self.maps.contains_key(&map_id) {
            let loaded = self.open_map(map_id);
            self.maps.insert(map_id, loaded);
        }
        let base = match self.maps.get_mut(&map_id) {
            Some(Some(map)) => map.ground_at(feet, walkable_only),
            _ => GroundProbe::Unloaded,
        };
        if solid_m2 {
            let check = self.m2_solid_checks.entry(map_id).or_insert_with(|| {
                let result = validate_m2_solid_files(&self.data_dir, map_id);
                if let Err(error) = &result {
                    error!("M2 solid collision on map {map_id}: {error}");
                }
                result
            });
            if check.is_err() {
                return GroundProbe::Unloaded;
            }
        }
        let walkable = self
            .m2_floors
            .entry(map_id)
            .or_insert_with(|| los::LosMap::new(&self.data_dir.join(format!("los/{map_id}/m2"))))
            .floor_height(feet);
        let m2 = if solid_m2 {
            let solid = self
                .m2_solids
                .entry(map_id)
                .or_insert_with(|| {
                    los::LosMap::new(&self.data_dir.join(format!("los/{map_id}/m2-solid")))
                })
                .solid_height(feet);
            walkable.and_then(|floor| {
                solid.map(|height| floor.into_iter().chain(height).max_by(f32::total_cmp))
            })
        } else {
            walkable
        };
        match m2 {
            Ok(Some(height)) => match base {
                GroundProbe::Supported(base_height) => {
                    GroundProbe::Supported(height.max(base_height))
                }
                _ => GroundProbe::Supported(height),
            },
            Ok(None) => base,
            Err(error) => {
                error!("M2 floor query on map {map_id}: {error}");
                GroundProbe::Unsupported
            }
        }
    }

    /// `Map::GetMinHeight` under `feet` on map `map_id`: below it a player is out of
    /// bounds. `None` where the tile's terrain is not available.
    pub fn min_height(&mut self, map_id: u32, feet: Vec3) -> Option<f32> {
        if !self.maps.contains_key(&map_id) {
            let loaded = self.open_map(map_id);
            self.maps.insert(map_id, loaded);
        }
        self.maps.get_mut(&map_id)?.as_mut()?.min_height(feet)
    }

    /// The terrain liquid over `feet` on map `map_id` (`TerrainInfo::GetLiquidStatus`'s
    /// grid lookup); `None` where there is none or the tile is not available.
    pub fn liquid_at(&mut self, map_id: u32, feet: Vec3) -> Option<LiquidSample> {
        if !self.maps.contains_key(&map_id) {
            let loaded = self.open_map(map_id);
            self.maps.insert(map_id, loaded);
        }
        self.maps.get_mut(&map_id)?.as_mut()?.liquid_at(feet)
    }

    /// Authored terrain area under `feet`, from the map's WDT/ADT MCNK header.
    pub fn area_at(&mut self, map_id: u32, feet: Vec3) -> Option<u32> {
        if !self.maps.contains_key(&map_id) {
            let loaded = self.open_map(map_id);
            self.maps.insert(map_id, loaded);
        }
        let map = self.maps.get_mut(&map_id)?.as_mut()?;
        let tile = map.tile(bevy_to_tile(feet.x, feet.z))?;
        let (x, y, _) = crate::terrain_height::bevy_to_wow(feet.x, feet.y, feet.z);
        tile.terrain.area_at(x, y)
    }

    fn open_map(&self, map_id: u32) -> Option<MapGround> {
        let Some(&wdt) = self.wdts.get(&map_id) else {
            warn!("Map {map_id} has no WDT; no ground");
            return None;
        };
        match MapGround::open(&self.data_dir, wdt) {
            Ok(map) => {
                info!("Ground of map {map_id} loads from WDT {wdt}");
                Some(map)
            }
            Err(err) => {
                error!("No ground on map {map_id}: {err}");
                None
            }
        }
    }
}

/// Ground geometry of one map: its global WMO, or the ADT tiles players have visited.
#[derive(Default)]
struct MapGround {
    source: Option<TileSource>,
    global_wmo: Option<WmoCollision>,
    global_liquids: Vec<LiquidTriangle>,
    liquid_models: HashMap<u32, Vec<LiquidTriangle>>,
    /// `None`: the tile's terrain is not available.
    tiles: HashMap<(u32, u32), Option<TileGround>>,
    /// WMO group collision by root FileDataID, shared by the tiles a WMO spans.
    wmo_models: HashMap<u32, Vec<Arc<WmoGroupCollision>>>,
}

impl MapGround {
    fn open(data_dir: &Path, wdt_fdid: u32) -> Result<Self, String> {
        let wdt = read_client_file(data_dir, "terrain", wdt_fdid, "wdt")?;
        let mphd = find_chunk(&wdt, b"MPHD")?.ok_or("WDT has no MPHD")?;
        if wdt::parse_mphd_flags(mphd)?.raw & MPHD_FLAG_GLOBAL_WMO != 0 {
            return Self::open_global_wmo(data_dir, &wdt);
        }
        Ok(Self {
            source: Some(TileSource {
                data_dir: data_dir.to_path_buf(),
                tiles: parse_maid(&wdt)?,
            }),
            ..default()
        })
    }

    /// The WDT MODF's single entry, placed from the world origin
    /// (`global_wmo_placement_position`).
    fn open_global_wmo(data_dir: &Path, wdt: &[u8]) -> Result<Self, String> {
        let modf = find_chunk(wdt, b"MODF")?.ok_or("global-WMO WDT has no MODF")?;
        let entry = modf
            .get(..MODF_ENTRY_SIZE)
            .ok_or("global-WMO WDT MODF is empty")?;
        let placement = parse_modf_entry(entry)?;
        let groups = load_wmo_groups(data_dir, placement.fdid)?;
        let world_from_local = Affine3A::from_scale_rotation_translation(
            Vec3::splat(placement.scale),
            placement_rotation(placement.rotation),
            global_wmo_placement_position(placement.position),
        );
        let global_liquids = load_wmo_liquids(data_dir, placement.fdid)?
            .iter()
            .map(|triangle| triangle.transformed(world_from_local))
            .collect();
        Ok(Self {
            global_wmo: Some(WmoCollision::new(world_from_local, groups)),
            global_liquids,
            ..default()
        })
    }

    fn min_height(&mut self, feet: Vec3) -> Option<f32> {
        if self.global_wmo.is_some() {
            return Some(DEFAULT_MIN_HEIGHT);
        }
        let tile = self.tile(bevy_to_tile(feet.x, feet.z))?;
        let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(feet.x, feet.y, feet.z);
        Some(tile.flight_min.map_or(DEFAULT_MIN_HEIGHT, |mins| {
            grid_min_height(wow_x, wow_y, &mins)
        }))
    }

    fn liquid_at(&mut self, feet: Vec3) -> Option<LiquidSample> {
        if self.global_wmo.is_some() {
            let sample = sample_wmo_liquid(&self.global_liquids, feet)?;
            let wmo = self.global_wmo.as_ref().expect("global WMO is loaded");
            return Some(with_liquid_floor(
                sample,
                feet,
                None,
                std::slice::from_ref(wmo),
            ));
        }
        let coordinates = bevy_to_tile(feet.x, feet.z);
        if self.tile(coordinates)?.liquids.is_none() {
            let liquids = self.load_tile_liquids(coordinates);
            self.tiles.get_mut(&coordinates)?.as_mut()?.liquids = Some(liquids);
        }
        let tile = self.load_collision_tile(coordinates)?;
        let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(feet.x, feet.y, feet.z);
        let terrain = tile.terrain.liquid_at(wow_x, wow_y);
        let wmo =
            sample_wmo_liquid(tile.liquids.as_deref().unwrap_or_default(), feet).map(|sample| {
                with_liquid_floor(
                    sample,
                    feet,
                    tile.terrain.height_at_bevy(feet.x, feet.z),
                    tile.wmos.as_deref().unwrap_or_default(),
                )
            });
        [terrain, wmo].into_iter().flatten().min_by(|a, b| {
            (a.level - feet.y)
                .abs()
                .total_cmp(&(b.level - feet.y).abs())
        })
    }

    fn load_tile_liquids(&mut self, coordinates: (u32, u32)) -> Vec<LiquidTriangle> {
        let source = self.source.as_ref().expect("terrain map has a tile source");
        let data_dir = source.data_dir.clone();
        let files: Vec<u32> = nearby_source_tiles(self, coordinates)
            .into_iter()
            .map(|(row, col)| source.tiles[col as usize * MAP_TILES + row as usize].obj0)
            .collect();
        let mut surfaces = Vec::new();
        for fdid in files {
            let placements = match load_wmo_placements(&data_dir, fdid) {
                Ok(placements) => placements,
                Err(error) => {
                    warn!("WMO liquid placements {fdid}: {error}");
                    continue;
                }
            };
            for placement in placements {
                if !self.liquid_models.contains_key(&placement.fdid) {
                    match load_wmo_liquids(&data_dir, placement.fdid) {
                        Ok(liquids) => {
                            self.liquid_models.insert(placement.fdid, liquids);
                        }
                        Err(error) => {
                            warn!("WMO {} liquid: {error}", placement.fdid);
                            continue;
                        }
                    }
                }
                surfaces.extend(
                    self.liquid_models[&placement.fdid]
                        .iter()
                        .map(|triangle| triangle.transformed(placement.world_from_local())),
                );
            }
        }
        surfaces
    }

    fn tile(&mut self, tile: (u32, u32)) -> Option<&TileGround> {
        if !self.tiles.contains_key(&tile) {
            let loaded = self.load_tile(tile);
            self.tiles.insert(tile, loaded);
        }
        self.tiles.get(&tile)?.as_ref()
    }

    fn ground_at(&mut self, feet: Vec3, walkable_only: bool) -> GroundProbe {
        if let Some(wmo) = &self.global_wmo {
            return probe_collision_height(feet, None, std::slice::from_ref(wmo), walkable_only);
        }
        let Some(tile) = self.load_collision_tile(bevy_to_tile(feet.x, feet.z)) else {
            return GroundProbe::Unloaded;
        };
        let terrain = tile.terrain.height_at_bevy(feet.x, feet.z);
        let wmos = tile.wmos.as_ref().expect("collision tile has loaded WMOs");
        probe_collision_height(feet, terrain, wmos, walkable_only)
    }

    fn fall_ground_at(&mut self, feet: Vec3) -> GroundProbe {
        if let Some(wmo) = &self.global_wmo {
            return probe_fall_height(feet, None, std::slice::from_ref(wmo));
        }
        let Some(tile) = self.load_collision_tile(bevy_to_tile(feet.x, feet.z)) else {
            return GroundProbe::Unloaded;
        };
        let terrain = tile.terrain.height_at_bevy(feet.x, feet.z);
        let wmos = tile.wmos.as_ref().expect("collision tile has loaded WMOs");
        probe_fall_height(feet, terrain, wmos)
    }

    fn load_collision_tile(&mut self, coordinates: (u32, u32)) -> Option<&TileGround> {
        if self.tile(coordinates)?.wmos.is_none() {
            let wmos = self.load_tile_wmos(coordinates);
            self.tiles
                .get_mut(&coordinates)
                .and_then(Option::as_mut)
                .expect("terrain tile stays loaded while its collision loads")
                .wmos = Some(wmos);
        }
        self.tiles[&coordinates].as_ref()
    }

    fn load_tile(&mut self, (row, col): (u32, u32)) -> Option<TileGround> {
        let source = self.source.as_ref()?;
        let files = source.tiles[col as usize * MAP_TILES + row as usize];
        let data_dir = source.data_dir.clone();
        let started = std::time::Instant::now();
        let (terrain, flight_min) = match load_terrain(&data_dir, files.root) {
            Ok(terrain) => terrain,
            Err(err) => {
                warn!("Ground tile ({row}, {col}) has no terrain: {err}");
                return None;
            }
        };
        info!(
            "Loaded ground terrain tile ({row}, {col}) in {:?}",
            started.elapsed()
        );
        Some(TileGround {
            terrain,
            wmos: None,
            liquids: None,
            flight_min,
        })
    }

    fn load_tile_wmos(&mut self, (row, col): (u32, u32)) -> Vec<WmoCollision> {
        let source = self
            .source
            .as_ref()
            .expect("terrain tile has its ADT source");
        let files = source.tiles[col as usize * MAP_TILES + row as usize];
        let data_dir = source.data_dir.clone();
        let started = std::time::Instant::now();
        let wmos: Vec<_> = match load_wmo_placements(&data_dir, files.obj0) {
            Ok(placements) => placements
                .into_iter()
                .filter_map(|placement| self.place_wmo(&data_dir, placement))
                .collect(),
            Err(err) => {
                warn!("Ground tile ({row}, {col}) has no WMO placements: {err}");
                Vec::new()
            }
        };
        info!(
            "Loaded ground collision tile ({row}, {col}): {} WMOs in {:?}",
            wmos.len(),
            started.elapsed()
        );
        wmos
    }

    fn place_wmo(&mut self, data_dir: &Path, placement: WmoPlacement) -> Option<WmoCollision> {
        if !self.wmo_models.contains_key(&placement.fdid) {
            match load_wmo_groups(data_dir, placement.fdid) {
                Ok(groups) => {
                    self.wmo_models.insert(placement.fdid, groups);
                }
                Err(err) => {
                    warn!("WMO {} has no floor collision: {err}", placement.fdid);
                    return None;
                }
            }
        }
        let groups = self.wmo_models[&placement.fdid].clone();
        Some(WmoCollision::new(placement.world_from_local(), groups))
    }
}

fn validate_m2_solid_files(data: &Path, map_id: u32) -> Result<(), String> {
    let root = data.join(format!("los/{map_id}"));
    let walkable = root.join("m2");
    let entries = match std::fs::read_dir(&walkable) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", walkable.display())),
    };
    let solid = root.join("m2-solid");
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", walkable.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "los") {
            let companion = solid.join(entry.file_name());
            if !companion.is_file() {
                return Err(format!("missing {}", companion.display()));
            }
        }
    }
    Ok(())
}

fn probe_collision_height<'a>(
    feet: Vec3,
    terrain: Option<f32>,
    wmos: impl IntoIterator<Item = &'a WmoCollision>,
    walkable_only: bool,
) -> GroundProbe {
    let support = if walkable_only {
        shared::ground::ground_at(feet, terrain, wmos)
    } else {
        shared::ground::collision_height(feet, terrain, wmos)
    };
    match support {
        Some(ground) => GroundProbe::Supported(ground.height),
        None => GroundProbe::Unsupported,
    }
}

fn probe_fall_height(feet: Vec3, terrain: Option<f32>, wmos: &[WmoCollision]) -> GroundProbe {
    let mut floors = Vec::new();
    for wmo in wmos {
        wmo.collision_heights(
            feet.x,
            feet.z,
            feet.y + shared::ground::STEP_UP_HEIGHT,
            feet.y - MAX_FALL_DISTANCE,
            &mut floors,
        );
    }
    shared::ground::select_ground(feet.y, terrain, floors)
        .map_or(GroundProbe::Unsupported, |ground| {
            GroundProbe::Supported(ground.height)
        })
}

fn select_fall_height(
    feet: Vec3,
    base: GroundProbe,
    heights: impl Iterator<Item = f32>,
) -> GroundProbe {
    let highest = heights
        .filter(|height| {
            *height <= feet.y + shared::ground::STEP_UP_HEIGHT
                && *height >= feet.y - MAX_FALL_DISTANCE
        })
        .max_by(f32::total_cmp);
    match (base, highest) {
        (GroundProbe::Supported(base), Some(height)) => GroundProbe::Supported(base.max(height)),
        (_, Some(height)) => GroundProbe::Supported(height),
        (base, None) => base,
    }
}

/// Walks one map's ground files as `MapGround` loads them, without parsing terrain or
/// collision.
struct FileCheck<'a> {
    data_dir: &'a Path,
    missing: Vec<String>,
    wmos: HashSet<u32>,
}

impl FileCheck<'_> {
    fn read(&mut self, dir: &str, fdid: u32, ext: &str) -> Option<Vec<u8>> {
        let data = std::fs::read(self.data_dir.join(format!("{dir}/{fdid}.{ext}"))).ok();
        if data.is_none() {
            self.missing.push(format!("{dir}/{fdid}.{ext}"));
        }
        data
    }

    fn exists(&mut self, dir: &str, fdid: u32, ext: &str) {
        let path = format!("{dir}/{fdid}.{ext}");
        if !self.data_dir.join(&path).is_file() {
            self.missing.push(path);
        }
    }

    fn unparsed(&mut self, dir: &str, fdid: u32, ext: &str, err: String) {
        self.missing.push(format!("{dir}/{fdid}.{ext}: {err}"));
    }

    fn map(&mut self, wdt_fdid: u32) {
        let Some(wdt) = self.read("terrain", wdt_fdid, "wdt") else {
            return;
        };
        match wdt_ground(&wdt) {
            Ok(WdtGround::GlobalWmo(fdid)) => self.wmo(fdid),
            Ok(WdtGround::Tiles(tiles)) => tiles.into_iter().for_each(|tile| self.tile(tile)),
            Err(err) => self.unparsed("terrain", wdt_fdid, "wdt", err),
        }
    }

    fn tile(&mut self, tile: TileFiles) {
        if tile.root != 0 {
            self.exists("terrain", tile.root, "adt");
        }
        if tile.obj0 == 0 {
            return;
        }
        let Some(obj0) = self.read("terrain", tile.obj0, "adt") else {
            return;
        };
        match placed_wmos(&obj0) {
            Ok(wmos) => wmos.into_iter().for_each(|fdid| self.wmo(fdid)),
            Err(err) => self.unparsed("terrain", tile.obj0, "adt", err),
        }
    }

    fn wmo(&mut self, fdid: u32) {
        if !self.wmos.insert(fdid) {
            return;
        }
        let Some(root) = self.read("models", fdid, "wmo") else {
            return;
        };
        match group_file_data_ids(&root) {
            Ok(groups) => groups
                .into_iter()
                .for_each(|group| self.exists("models", group, "wmo")),
            Err(err) => self.unparsed("models", fdid, "wmo", err),
        }
    }
}

fn missing_ground_files(data_dir: &Path, wdt: Option<u32>) -> Vec<String> {
    let mut check = FileCheck {
        data_dir,
        missing: Vec::new(),
        wmos: HashSet::new(),
    };
    match wdt {
        Some(wdt) if wdt != 0 => check.map(wdt),
        _ => check.missing.push(NO_WDT.into()),
    }
    check.missing
}

enum WdtGround {
    /// The WDT MODF's WMO (MPHD flag 0x1).
    GlobalWmo(u32),
    Tiles(Vec<TileFiles>),
}

fn wdt_ground(wdt: &[u8]) -> Result<WdtGround, String> {
    let mphd = find_chunk(wdt, b"MPHD")?.ok_or("WDT has no MPHD")?;
    if wdt::parse_mphd_flags(mphd)?.raw & MPHD_FLAG_GLOBAL_WMO == 0 {
        return parse_maid(wdt).map(WdtGround::Tiles);
    }
    let modf = find_chunk(wdt, b"MODF")?.ok_or("global-WMO WDT has no MODF")?;
    let entry = modf
        .get(..MODF_ENTRY_SIZE)
        .ok_or("global-WMO WDT MODF is empty")?;
    Ok(WdtGround::GlobalWmo(parse_modf_entry(entry)?.fdid))
}

/// WMO FileDataIDs an `_obj0` MODF places (entries by path are skipped, as on load).
fn placed_wmos(obj0: &[u8]) -> Result<Vec<u32>, String> {
    let Some(modf) = find_chunk(obj0, b"MODF")? else {
        return Ok(Vec::new());
    };
    Ok(modf
        .as_chunks::<MODF_ENTRY_SIZE>()
        .0
        .iter()
        .filter_map(|entry| parse_modf_entry(entry).ok())
        .map(|placement| placement.fdid)
        .collect())
}

/// `(row, col)` of the map tile containing Bevy `(x, z)`; the row comes from
/// WoW Y (`-z`), the column from WoW X.
pub(crate) fn bevy_to_tile(bx: f32, bz: f32) -> (u32, u32) {
    let center = 32.0 * TILE_SIZE;
    let row = ((center + bz) / TILE_SIZE).floor() as i32;
    let col = ((center - bx) / TILE_SIZE).floor() as i32;
    (row.clamp(0, 63) as u32, col.clamp(0, 63) as u32)
}

struct WmoPlacement {
    fdid: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
}

impl WmoPlacement {
    fn world_from_local(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            placement_rotation(self.rotation),
            placement_position(self.position),
        )
    }
}

/// Root ADT heights and its MFBO minimum plane corners.
fn load_terrain(
    data_dir: &Path,
    fdid: u32,
) -> Result<(TerrainHeightMap, Option<[i16; 9]>), String> {
    let data = read_client_file(data_dir, "terrain", fdid, "adt")?;
    let flight_min = find_chunk(&data, b"MFBO")?
        .map(parse_mfbo_min_heights)
        .transpose()?;
    Ok((parse_adt_heights(&data)?, flight_min))
}

/// GridMap::getMinHeight (GridMap.cpp:547-575) and its MFBO planes (GridMap.cpp:200-239): the
/// grid is cut into eight triangles around its centre, each a plane through three of the nine
/// MFBO minimum corners; the height is that plane's under WoW `(x, y)`.
fn grid_min_height(x: f32, y: f32, mins: &[i16; 9]) -> f32 {
    const INDICES: [[usize; 3]; 8] = [
        [3, 0, 4],
        [0, 1, 4],
        [1, 2, 4],
        [2, 5, 4],
        [5, 8, 4],
        [8, 7, 4],
        [7, 6, 4],
        [6, 3, 4],
    ];
    const BOUND_GRID_COORDS: [[f32; 2]; 9] = [
        [0.0, 0.0],
        [0.0, -266.66666],
        [0.0, -533.33331],
        [-266.66666, 0.0],
        [-266.66666, -266.66666],
        [-266.66666, -533.33331],
        [-533.33331, 0.0],
        [-533.33331, -266.66666],
        [-533.33331, -533.33331],
    ];
    // Trinity::ComputeGridCoordSimple.
    let grid_x = (CENTER_GRID_ID * 2 - 1) - (CENTER_GRID_ID as f32 - x / SIZE_OF_GRIDS) as i32;
    let grid_y = (CENTER_GRID_ID * 2 - 1) - (CENTER_GRID_ID as f32 - y / SIZE_OF_GRIDS) as i32;
    let double_grid_x = (-(x - MAP_HALFSIZE) / (SIZE_OF_GRIDS / 2.0)).floor() as i32;
    let double_grid_y = (-(y - MAP_HALFSIZE) / (SIZE_OF_GRIDS / 2.0)).floor() as i32;
    let gx = x - (grid_x - CENTER_GRID_ID + 1) as f32 * SIZE_OF_GRIDS;
    let gy = y - (grid_y - CENTER_GRID_ID + 1) as f32 * SIZE_OF_GRIDS;
    let quarter = match (double_grid_y & 1 != 0, double_grid_x & 1 != 0) {
        (true, true) => 4 + usize::from(gx <= gy),
        (true, false) => 2 + usize::from(-SIZE_OF_GRIDS - gx > gy),
        (false, true) => 6 + usize::from(-SIZE_OF_GRIDS - gx <= gy),
        (false, false) => usize::from(gx > gy),
    };
    let corner = |index: usize| {
        let [cx, cy] = BOUND_GRID_COORDS[index];
        Vec3::new(cx, cy, f32::from(mins[index]))
    };
    let [a, b, c] = INDICES[quarter].map(corner);
    // G3D::Ray::intersection(Plane) from (gx, gy, 0) straight up: infinite unless the
    // plane faces down.
    let normal = (b - a).cross(c - a).normalize();
    if normal.z >= 0.0 {
        return f32::INFINITY;
    }
    a.z - (normal.x * (gx - a.x) + normal.y * (gy - a.y)) / normal.z
}

fn load_wmo_placements(data_dir: &Path, obj0_fdid: u32) -> Result<Vec<WmoPlacement>, String> {
    let data = read_client_file(data_dir, "terrain", obj0_fdid, "adt")?;
    let Some(modf) = find_chunk(&data, b"MODF")? else {
        return Ok(Vec::new());
    };
    let mut placements = Vec::new();
    for entry in modf.chunks_exact(MODF_ENTRY_SIZE) {
        match parse_modf_entry(entry) {
            Ok(placement) => placements.push(placement),
            Err(err) => warn!("Skipping MODF entry in {obj0_fdid}: {err}"),
        }
    }
    Ok(placements)
}

fn parse_modf_entry(entry: &[u8]) -> Result<WmoPlacement, String> {
    let flags = adt_obj::modf_flags(entry)?;
    if flags & MODF_FLAG_FILEDATAID == 0 {
        return Err(format!(
            "MODF entry names its WMO by path (flags {flags:#x})"
        ));
    }
    let transform = adt_obj::parse_modf_transform(entry)?;
    Ok(WmoPlacement {
        fdid: transform.name_id,
        position: transform.position,
        rotation: transform.rotation,
        scale: transform.scale,
    })
}

fn load_wmo_liquids(data_dir: &Path, root_fdid: u32) -> Result<Vec<LiquidTriangle>, String> {
    los::wmo_liquid::load_model(data_dir, root_fdid)
}

fn sample_wmo_liquid(triangles: &[LiquidTriangle], feet: Vec3) -> Option<LiquidSample> {
    triangles
        .iter()
        .filter_map(|triangle| {
            triangle.level_at(feet.x, feet.z).map(|level| LiquidSample {
                level,
                depth_level: f32::NEG_INFINITY,
                liquid_type: triangle.liquid_type,
                deep: false,
            })
        })
        .min_by(|a, b| {
            (a.level - feet.y)
                .abs()
                .total_cmp(&(b.level - feet.y).abs())
        })
}

fn with_liquid_floor(
    mut liquid: LiquidSample,
    feet: Vec3,
    terrain: Option<f32>,
    wmos: &[WmoCollision],
) -> LiquidSample {
    let ceiling = feet.y.min(liquid.level) + 0.05;
    let mut floors = Vec::new();
    for wmo in wmos {
        wmo.collision_heights(
            feet.x,
            feet.z,
            ceiling,
            ceiling - MAX_FALL_DISTANCE,
            &mut floors,
        );
    }
    liquid.depth_level = floors
        .into_iter()
        .chain(terrain)
        .filter(|height| *height <= ceiling)
        .max_by(f32::total_cmp)
        .unwrap_or(f32::NEG_INFINITY);
    liquid
}

fn load_wmo_groups(data_dir: &Path, root_fdid: u32) -> Result<Vec<Arc<WmoGroupCollision>>, String> {
    let root = read_client_file(data_dir, "models", root_fdid, "wmo")?;
    group_file_data_ids(&root)?
        .into_iter()
        .map(|fdid| {
            let data = read_client_file(data_dir, "models", fdid, "wmo")?;
            WmoGroupCollision::parse(&data)
                .map(Arc::new)
                .map_err(|err| format!("group {fdid}: {err}"))
        })
        .collect()
}

fn parse_maid(wdt: &[u8]) -> Result<Vec<TileFiles>, String> {
    let maid = find_chunk(wdt, b"MAID")?.ok_or("WDT has no MAID")?;
    Ok(wdt::parse_maid_root_obj0(maid)?
        .into_iter()
        .map(|(root, obj0)| TileFiles { root, obj0 })
        .collect())
}

fn read_client_file(data_dir: &Path, dir: &str, fdid: u32, ext: &str) -> Result<Vec<u8>, String> {
    if fdid == 0 {
        return Err(format!("no {dir} FileDataID"));
    }
    let path = data_dir.join(format!("{dir}/{fdid}.{ext}"));
    std::fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))
}

pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, open_ground_map);
    }
}

/// Where the ground loads from: `GAME_SERVER_GROUND_DIR`, else the client's asset cache
/// beside this checkout (not the working directory).
pub(crate) fn ground_data_dir() -> PathBuf {
    std::env::var_os(GROUND_DIR_ENV).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        PathBuf::from,
    )
}

/// After `WorldDataPlugin` has loaded the `MapStore` and the spawns (Startup). Without
/// ground every creature keeps its spawn height wherever it runs, so the server refuses
/// to start while a map with creature spawns misses a ground file other than the
/// `KNOWN_GROUND_GAPS`.
pub(crate) fn open_ground_map(
    mut commands: Commands,
    maps: Res<crate::maps::MapStore>,
    world: Res<crate::world_data::WorldData>,
) {
    let started = std::time::Instant::now();
    let data_dir = ground_data_dir();
    let wdts = maps
        .maps
        .values()
        .map(|entry| (entry.id, entry.wdt_file_data_id))
        .collect();
    let ground = GroundMap::new(&data_dir, wdts);
    let spawned = world.spawns_by_map.keys().map(|&map| u32::from(map));
    let check = check_spawned_ground(&ground, spawned, &known_ground_gaps());
    if !check.missing.is_empty() {
        panic!(
            "No ground from {} (set {GROUND_DIR_ENV}): {} files missing: {}",
            data_dir.display(),
            check.missing.len(),
            check.missing[..check.missing.len().min(20)].join(", ")
        );
    }
    if !check.known_gaps.is_empty() {
        warn!(
            "Ground known gaps: {} files of maps {:?} are not in local CASC (ground_gaps.tsv); creatures there keep their spawn height",
            check.known_gap_files, check.known_gaps
        );
    }
    info!(
        "Ground loads from {} (validation {:?})",
        data_dir.display(),
        started.elapsed()
    );
    commands.insert_resource(ground);
}

fn known_ground_gaps() -> HashSet<(u32, String)> {
    KNOWN_GROUND_GAPS
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(map, file)| {
            (
                map.parse().expect("ground_gaps.tsv map id"),
                file.to_string(),
            )
        })
        .collect()
}

#[derive(Debug, Default, PartialEq)]
struct SpawnedGroundCheck {
    /// `map <id>: <file>` of each missing file not among the known gaps.
    missing: Vec<String>,
    /// Maps missing only known-gap files, ascending.
    known_gaps: Vec<u32>,
    known_gap_files: usize,
}

fn check_spawned_ground(
    ground: &GroundMap,
    spawned: impl IntoIterator<Item = u32>,
    gaps: &HashSet<(u32, String)>,
) -> SpawnedGroundCheck {
    let results = read_spawned_ground_files(ground, spawned);
    let mut check = SpawnedGroundCheck::default();
    for (map, missing) in results {
        let mut gap = false;
        for file in missing {
            if gaps.contains(&(map, file.clone())) {
                gap = true;
                check.known_gap_files += 1;
            } else {
                check.missing.push(format!("map {map}: {file}"));
            }
        }
        if gap {
            check.known_gaps.push(map);
        }
    }
    check
}

fn read_spawned_ground_files(
    ground: &GroundMap,
    spawned: impl IntoIterator<Item = u32>,
) -> Vec<(u32, Vec<String>)> {
    // Bevy stays single-threaded. Only independent startup file walks overlap.
    let mut maps: Vec<_> = spawned.into_iter().collect();
    maps.sort_unstable();
    let chunk_size = maps.len().div_ceil(GROUND_VALIDATION_WORKERS).max(1);
    let data_dir = &ground.data_dir;
    let wdts = &ground.wdts;
    std::thread::scope(|scope| {
        let workers: Vec<_> = maps
            .chunks(chunk_size)
            .map(|maps| scope.spawn(move || read_map_ground_files(data_dir, wdts, maps)))
            .collect();
        // Join in source-map order so refusal and known-gap diagnostics stay stable.
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("ground validation worker panicked"))
            .collect()
    })
}

fn read_map_ground_files(
    data_dir: &Path,
    wdts: &HashMap<u32, u32>,
    maps: &[u32],
) -> Vec<(u32, Vec<String>)> {
    maps.iter()
        .map(|&map| {
            let started = std::time::Instant::now();
            let missing = missing_ground_files(data_dir, wdts.get(&map).copied());
            debug!("Ground validation map {map}: {:?}", started.elapsed());
            (map, missing)
        })
        .collect()
}

#[cfg(test)]
#[path = "ground/m2_floor_tests.rs"]
mod m2_floor_tests;

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "ground/startup_latency_tests.rs"]
mod startup_latency_tests;

#[cfg(test)]
#[path = "ground/format_metadata_tests.rs"]
mod format_metadata_tests;

#[cfg(test)]
#[path = "ground_achievement_fall_tests.rs"]
mod achievement_fall_tests;

#[cfg(test)]
#[path = "ground_private_runs_tests.rs"]
mod private_runs_tests;

#[cfg(test)]
#[path = "stv_ground_tests.rs"]
mod stv_tests;

#[cfg(test)]
#[path = "zephras_ground_tests.rs"]
mod zephras_tests;

#[cfg(test)]
#[path = "housing_ground_tests.rs"]
mod housing_tests;

#[cfg(test)]
#[path = "ground/mfbo_receipts_tests.rs"]
mod mfbo_receipts_tests;
