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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::math::{Affine3A, Vec3};
use bevy::prelude::*;
use shared::ground::{
    WmoCollision, WmoGroupCollision, global_wmo_placement_position, placement_position,
    placement_rotation,
};

use crate::liquid::LiquidSample;
use crate::terrain_height::{TerrainHeightMap, parse_adt_heights};

/// Overrides the client asset cache the ground loads from.
const GROUND_DIR_ENV: &str = "GAME_SERVER_GROUND_DIR";
/// Eastern Kingdoms: the server does not start without its ground.
const REQUIRED_GROUND_MAP: u32 = 0;
/// `<map id>\t<path>\t<absent|encrypted>`: ground files local CASC cannot produce
/// (scripts/extract_ground.py); a spawned map missing only these still starts.
const KNOWN_GROUND_GAPS: &str = include_str!("../ground_gaps.tsv");
const TILE_SIZE: f32 = shared::navmesh::TILE_SIZE;
const MAP_TILES: usize = 64;
/// MAID entry: root, obj0, obj1, tex0, lod, map texture, map normal, minimap FDIDs.
const MAID_ENTRY_SIZE: usize = 32;
const MODF_ENTRY_SIZE: usize = 64;
const MODF_FLAG_HAS_SCALE: u16 = 0x4;
const MODF_FLAG_FILEDATAID: u16 = 0x8;
/// wowdev WDT MPHD `wdt_uses_global_map_obj`: the map is one WMO, placed by the WDT MODF.
const MPHD_FLAG_GLOBAL_WMO: u32 = 0x1;
/// `GridMap::getMinHeight` / `TerrainInfo::GetMinHeight` without flight bounds (no MFBO, or
/// no grid at all as on a WMO-only map).
const DEFAULT_MIN_HEIGHT: f32 = -500.0;
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

struct TileGround {
    terrain: TerrainHeightMap,
    wmos: Vec<WmoCollision>,
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
    data_dir: PathBuf,
    /// `Map.db2` WdtFileDataID by map id.
    wdts: HashMap<u32, u32>,
    /// `None`: the map's WDT is not available.
    maps: HashMap<u32, Option<MapGround>>,
}

impl GroundMap {
    /// Maps load lazily from `data_dir` (`terrain/<fdid>.wdt|adt`, `models/<fdid>.wmo`).
    pub fn new(data_dir: &Path, wdts: HashMap<u32, u32>) -> Self {
        Self {
            data_dir: data_dir.to_path_buf(),
            wdts,
            maps: HashMap::new(),
        }
    }

    /// A map 0 holding only `terrain` for `tile`, without WMOs or lazy loading.
    #[cfg(test)]
    pub(crate) fn with_terrain(tile: (u32, u32), terrain: TerrainHeightMap) -> Self {
        let mut map = MapGround::default();
        map.tiles.insert(
            tile,
            Some(TileGround {
                terrain,
                wmos: Vec::new(),
                flight_min: None,
            }),
        );
        Self {
            maps: HashMap::from([(0, Some(map))]),
            ..default()
        }
    }

    /// Opens map `map_id` now; the error names the file that is missing or unreadable.
    pub fn open_now(&mut self, map_id: u32) -> Result<(), String> {
        let &wdt = self
            .wdts
            .get(&map_id)
            .ok_or_else(|| format!("map {map_id} has no WDT in Map.db2"))?;
        let map = MapGround::open(&self.data_dir, wdt)?;
        self.maps.insert(map_id, Some(map));
        Ok(())
    }

    /// Ground under `feet` (Bevy space) on map `map_id`, loading the map and its tile on
    /// first use.
    pub fn ground_at(&mut self, map_id: u32, feet: Vec3) -> GroundProbe {
        if !self.maps.contains_key(&map_id) {
            let loaded = self.open_map(map_id);
            self.maps.insert(map_id, loaded);
        }
        match self.maps.get_mut(&map_id) {
            Some(Some(map)) => map.ground_at(feet),
            _ => GroundProbe::Unloaded,
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
    /// `None`: the tile's terrain is not available.
    tiles: HashMap<(u32, u32), Option<TileGround>>,
    /// WMO group collision by root FileDataID, shared by the tiles a WMO spans.
    wmo_models: HashMap<u32, Vec<Arc<WmoGroupCollision>>>,
}

impl MapGround {
    fn open(data_dir: &Path, wdt_fdid: u32) -> Result<Self, String> {
        let wdt = read_client_file(data_dir, "terrain", wdt_fdid, "wdt")?;
        let mphd = find_chunk(&wdt, b"MPHD")?.ok_or("WDT has no MPHD")?;
        if read_u32(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO != 0 {
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
        Ok(Self {
            global_wmo: Some(WmoCollision::new(world_from_local, groups)),
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
            return None;
        }
        let tile = self.tile(bevy_to_tile(feet.x, feet.z))?;
        let (wow_x, wow_y, _) = crate::terrain_height::bevy_to_wow(feet.x, feet.y, feet.z);
        tile.terrain.liquid_at(wow_x, wow_y)
    }

    fn tile(&mut self, tile: (u32, u32)) -> Option<&TileGround> {
        if !self.tiles.contains_key(&tile) {
            let loaded = self.load_tile(tile);
            self.tiles.insert(tile, loaded);
        }
        self.tiles.get(&tile)?.as_ref()
    }

    fn ground_at(&mut self, feet: Vec3) -> GroundProbe {
        if let Some(wmo) = &self.global_wmo {
            return match shared::ground::ground_at(feet, None, std::slice::from_ref(wmo)) {
                Some(ground) => GroundProbe::Supported(ground.height),
                None => GroundProbe::Unsupported,
            };
        }
        let Some(tile) = self.tile(bevy_to_tile(feet.x, feet.z)) else {
            return GroundProbe::Unloaded;
        };
        let terrain = tile.terrain.height_at_bevy(feet.x, feet.z);
        match shared::ground::ground_at(feet, terrain, &tile.wmos) {
            Some(ground) => GroundProbe::Supported(ground.height),
            None => GroundProbe::Unsupported,
        }
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
        let wmos = match load_wmo_placements(&data_dir, files.obj0) {
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
            "Loaded ground tile ({row}, {col}): {} WMOs in {:?}",
            wmos.len(),
            started.elapsed()
        );
        Some(TileGround {
            terrain,
            wmos,
            flight_min,
        })
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
        .map(|mfbo| -> Result<[i16; 9], String> {
            let mut mins = [0; 9];
            for (index, min) in mins.iter_mut().enumerate() {
                *min = read_u16(mfbo, 18 + index * 2)? as i16;
            }
            Ok(mins)
        })
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
    let flags = read_u16(entry, 56)?;
    if flags & MODF_FLAG_FILEDATAID == 0 {
        return Err(format!(
            "MODF entry names its WMO by path (flags {flags:#x})"
        ));
    }
    let scale = if flags & MODF_FLAG_HAS_SCALE != 0 {
        f32::from(read_u16(entry, 62)?) / 1024.0
    } else {
        1.0
    };
    Ok(WmoPlacement {
        fdid: read_u32(entry, 0)?,
        position: read_vec3(entry, 8)?,
        rotation: read_vec3(entry, 20)?,
        scale,
    })
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

/// The root's MOHD group count of GFID group FileDataIDs.
fn group_file_data_ids(root: &[u8]) -> Result<Vec<u32>, String> {
    let mohd = find_chunk(root, b"MOHD")?.ok_or("WMO root has no MOHD")?;
    let group_count = read_u32(mohd, 4)? as usize;
    let gfid = find_chunk(root, b"GFID")?.ok_or("WMO root has no GFID")?;
    (0..group_count)
        .map(|index| read_u32(gfid, index * 4))
        .collect()
}

fn parse_maid(wdt: &[u8]) -> Result<Vec<TileFiles>, String> {
    let maid = find_chunk(wdt, b"MAID")?.ok_or("WDT has no MAID")?;
    (0..MAP_TILES * MAP_TILES)
        .map(|index| {
            let base = index * MAID_ENTRY_SIZE;
            Ok(TileFiles {
                root: read_u32(maid, base)?,
                obj0: read_u32(maid, base + 4)?,
            })
        })
        .collect()
}

fn read_client_file(data_dir: &Path, dir: &str, fdid: u32, ext: &str) -> Result<Vec<u8>, String> {
    if fdid == 0 {
        return Err(format!("no {dir} FileDataID"));
    }
    let path = data_dir.join(format!("{dir}/{fdid}.{ext}"));
    std::fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))
}

/// Payload of the first chunk tagged `tag` (tags are stored reversed).
fn find_chunk<'a>(data: &'a [u8], tag: &[u8; 4]) -> Result<Option<&'a [u8]>, String> {
    let mut rest = data;
    while rest.len() >= 8 {
        let chunk_tag = [rest[3], rest[2], rest[1], rest[0]];
        let size = read_u32(rest, 4)? as usize;
        let payload = rest
            .get(8..8 + size)
            .ok_or_else(|| format!("{} chunk truncated", String::from_utf8_lossy(&chunk_tag)))?;
        if &chunk_tag == tag {
            return Ok(Some(payload));
        }
        rest = &rest[8 + size..];
    }
    Ok(None)
}

fn read_u16(data: &[u8], offset: usize) -> Result<u16, String> {
    data.get(offset..offset + 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .ok_or_else(|| format!("read_u16 out of bounds at {offset:#x}"))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, String> {
    data.get(offset..offset + 4)
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .ok_or_else(|| format!("read_u32 out of bounds at {offset:#x}"))
}

fn read_vec3(data: &[u8], offset: usize) -> Result<[f32; 3], String> {
    Ok([
        f32::from_bits(read_u32(data, offset)?),
        f32::from_bits(read_u32(data, offset + 4)?),
        f32::from_bits(read_u32(data, offset + 8)?),
    ])
}

/// Ground files of spawned maps that are missing but listed in `KNOWN_GROUND_GAPS`.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct KnownGroundGaps {
    /// Missing file count by map id.
    pub(crate) by_map: BTreeMap<u32, usize>,
    /// Spawned maps without a WDT (`Map.db2` WdtFileDataID 0: transports).
    pub(crate) without_wdt: usize,
}

/// Every file `MapGround` reads for each spawned map must be in `data_dir`, unless
/// `known_gaps` lists its path; the error names the first map and file that is missing.
pub(crate) fn check_spawned_ground(
    data_dir: &Path,
    wdts: &HashMap<u32, u32>,
    spawned_maps: &BTreeSet<u32>,
    known_gaps: &HashSet<&str>,
) -> Result<KnownGroundGaps, String> {
    let mut gaps = KnownGroundGaps::default();
    for &map_id in spawned_maps {
        let wdt = wdts.get(&map_id).copied().unwrap_or(0);
        if wdt == 0 {
            gaps.without_wdt += 1;
            continue;
        }
        for path in missing_ground_files(data_dir, wdt)? {
            if !known_gaps.contains(path.as_str()) {
                return Err(format!(
                    "No ground on map {map_id}: {} is missing",
                    data_dir.join(&path).display()
                ));
            }
            *gaps.by_map.entry(map_id).or_default() += 1;
        }
    }
    Ok(gaps)
}

/// Paths (under `data_dir`) of the files the ground of the map with WDT `wdt_fdid` reads
/// that are missing: the WDT; its global WMO, or every MAID tile's root and `_obj0` ADT
/// and the WMOs those `_obj0` MODFs place; each WMO root's GFID groups.
fn missing_ground_files(data_dir: &Path, wdt_fdid: u32) -> Result<Vec<String>, String> {
    let mut files = GroundFileCheck {
        data_dir,
        missing: Vec::new(),
        wmos: HashSet::new(),
    };
    let Some(wdt) = files.read("terrain", wdt_fdid, "wdt")? else {
        return Ok(files.missing);
    };
    let mphd = find_chunk(&wdt, b"MPHD")?.ok_or("WDT has no MPHD")?;
    if read_u32(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO != 0 {
        let modf = find_chunk(&wdt, b"MODF")?.ok_or("global-WMO WDT has no MODF")?;
        let entry = modf
            .get(..MODF_ENTRY_SIZE)
            .ok_or("global-WMO WDT MODF is empty")?;
        files.check_wmo(parse_modf_entry(entry)?.fdid)?;
        return Ok(files.missing);
    }
    for tile in parse_maid(&wdt)? {
        if tile.root != 0 {
            files.read_exists("terrain", tile.root, "adt");
        }
        if tile.obj0 != 0 && files.read_exists("terrain", tile.obj0, "adt") {
            for placement in load_wmo_placements(data_dir, tile.obj0)? {
                files.check_wmo(placement.fdid)?;
            }
        }
    }
    Ok(files.missing)
}

struct GroundFileCheck<'a> {
    data_dir: &'a Path,
    missing: Vec<String>,
    /// WMO roots already checked: tiles share them.
    wmos: HashSet<u32>,
}

impl GroundFileCheck<'_> {
    /// Whether `<dir>/<fdid>.<ext>` exists, recording it as missing if not.
    fn read_exists(&mut self, dir: &str, fdid: u32, ext: &str) -> bool {
        let path = format!("{dir}/{fdid}.{ext}");
        let exists = self.data_dir.join(&path).is_file();
        if !exists {
            self.missing.push(path);
        }
        exists
    }

    fn read(&mut self, dir: &str, fdid: u32, ext: &str) -> Result<Option<Vec<u8>>, String> {
        if !self.read_exists(dir, fdid, ext) {
            return Ok(None);
        }
        read_client_file(self.data_dir, dir, fdid, ext).map(Some)
    }

    fn check_wmo(&mut self, root_fdid: u32) -> Result<(), String> {
        if !self.wmos.insert(root_fdid) {
            return Ok(());
        }
        let Some(root) = self.read("models", root_fdid, "wmo")? else {
            return Ok(());
        };
        for group in group_file_data_ids(&root).map_err(|err| format!("WMO {root_fdid}: {err}"))? {
            self.read_exists("models", group, "wmo");
        }
        Ok(())
    }
}

fn known_ground_gaps() -> HashSet<&'static str> {
    KNOWN_GROUND_GAPS
        .lines()
        .filter_map(|line| line.split('\t').nth(1))
        .collect()
}

pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, open_ground_map);
    }
}

/// Where the ground loads from: `GAME_SERVER_GROUND_DIR`, else the client's asset cache
/// beside this checkout (not the working directory).
fn ground_data_dir() -> PathBuf {
    std::env::var_os(GROUND_DIR_ENV).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        PathBuf::from,
    )
}

/// After `WorldDataPlugin` has loaded the `MapStore` and spawns (Startup). Without ground
/// every creature keeps its spawn height wherever it runs, so the server refuses to start
/// unless `REQUIRED_GROUND_MAP` opens and every map with creature spawns has all its
/// ground files, `KNOWN_GROUND_GAPS` aside.
fn open_ground_map(
    mut commands: Commands,
    maps: Res<crate::maps::MapStore>,
    world: Res<crate::world_data::WorldData>,
) {
    let data_dir = ground_data_dir();
    let wdts = maps
        .maps
        .values()
        .map(|entry| (entry.id, entry.wdt_file_data_id))
        .collect();
    let mut ground = GroundMap::new(&data_dir, wdts);
    if let Err(err) = ground.open_now(REQUIRED_GROUND_MAP) {
        panic!(
            "No ground on map {REQUIRED_GROUND_MAP} from {} (set {GROUND_DIR_ENV}): {err}",
            data_dir.display()
        );
    }
    let spawned_maps = world
        .spawns_by_map
        .keys()
        .map(|&map| u32::from(map))
        .collect();
    let gaps = check_spawned_ground(&data_dir, &ground.wdts, &spawned_maps, &known_ground_gaps())
        .unwrap_or_else(|err| panic!("{err} (set {GROUND_DIR_ENV})"));
    let gap_files: usize = gaps.by_map.values().sum();
    warn!(
        "Known ground gap: {gap_files} files of {} spawned maps are absent or encrypted in \
         local CASC (ground_gaps.tsv), so those parts have no ground: {:?}",
        gaps.by_map.len(),
        gaps.by_map
    );
    info!(
        "Ground of {} spawned maps complete; {} spawned maps have no WDT (transports)",
        spawned_maps.len() - gaps.by_map.len() - gaps.without_wdt,
        gaps.without_wdt
    );
    info!("Ground loads from {}", data_dir.display());
    commands.insert_resource(ground);
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
