//! Server-side player ground: ADT terrain and WMO floors per map tile, judged by
//! the shared rule the client predicts with (`shared::ground`,
//! game-engine docs/specs/wmo-floor-collision.md).
//!
//! Tiles load on first use from the client's asset cache (`../game-engine/data`):
//! the azeroth WDT MAID names each tile's root and `_obj0` ADT FileDataIDs, the
//! `_obj0` MODF places WMOs by FileDataID, and each WMO root's GFID names its
//! group files.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::math::{Affine3A, Vec3};
use bevy::prelude::*;
use shared::ground::{WmoCollision, WmoGroupCollision, placement_position, placement_rotation};

use crate::terrain_height::{TerrainHeightMap, parse_adt_heights};

/// The client's asset cache, relative to the server's working directory.
const CLIENT_DATA_DIR: &str = "../game-engine/data";
/// `world/maps/azeroth/azeroth.wdt`.
const AZEROTH_WDT_FDID: u32 = 775_971;
const TILE_SIZE: f32 = shared::navmesh::TILE_SIZE;
const MAP_TILES: usize = 64;
/// MAID entry: root, obj0, obj1, tex0, lod, map texture, map normal, minimap FDIDs.
const MAID_ENTRY_SIZE: usize = 32;
const MODF_ENTRY_SIZE: usize = 64;
const MODF_FLAG_HAS_SCALE: u16 = 0x4;
const MODF_FLAG_FILEDATAID: u16 = 0x8;

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

/// Ground geometry of the tiles players have visited.
#[derive(Resource, Default)]
pub struct GroundMap {
    source: Option<TileSource>,
    /// `None`: the tile's terrain is not available.
    tiles: HashMap<(u32, u32), Option<TileGround>>,
    /// WMO group collision by root FileDataID, shared by the tiles a WMO spans.
    wmo_models: HashMap<u32, Vec<Arc<WmoGroupCollision>>>,
}

impl GroundMap {
    /// Tiles load lazily from `data_dir` (`terrain/<fdid>.adt`, `models/<fdid>.wmo`).
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        let wdt_path = data_dir.join(format!("terrain/{AZEROTH_WDT_FDID}.wdt"));
        let wdt = std::fs::read(&wdt_path).map_err(|e| format!("{}: {e}", wdt_path.display()))?;
        Ok(Self {
            source: Some(TileSource {
                data_dir: data_dir.to_path_buf(),
                tiles: parse_maid(&wdt)?,
            }),
            ..default()
        })
    }

    /// A map holding only `terrain` for `tile`, without WMOs or lazy loading.
    #[cfg(test)]
    pub(crate) fn with_terrain(tile: (u32, u32), terrain: TerrainHeightMap) -> Self {
        let mut map = Self::default();
        map.tiles.insert(
            tile,
            Some(TileGround {
                terrain,
                wmos: Vec::new(),
            }),
        );
        map
    }

    /// Ground under `feet` (Bevy space), loading its tile on first use.
    pub fn ground_at(&mut self, feet: Vec3) -> GroundProbe {
        let tile = bevy_to_tile(feet.x, feet.z);
        if !self.tiles.contains_key(&tile) {
            let loaded = self.load_tile(tile);
            self.tiles.insert(tile, loaded);
        }
        let Some(Some(tile)) = self.tiles.get(&tile) else {
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
        let terrain = match load_terrain(&data_dir, files.root) {
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
        Some(TileGround { terrain, wmos })
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

fn load_terrain(data_dir: &Path, fdid: u32) -> Result<TerrainHeightMap, String> {
    let data = read_client_file(data_dir, "terrain", fdid, "adt")?;
    parse_adt_heights(&data)
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

pub struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, open_ground_map);
    }
}

fn open_ground_map(mut commands: Commands) {
    let data_dir = Path::new(CLIENT_DATA_DIR);
    match GroundMap::open(data_dir) {
        Ok(map) => {
            info!("Ground tiles load from {}", data_dir.display());
            commands.insert_resource(map);
        }
        Err(err) => {
            error!("No player ground: {err}");
            commands.insert_resource(GroundMap::default());
        }
    }
}

#[cfg(test)]
#[path = "ground_tests.rs"]
mod tests;
