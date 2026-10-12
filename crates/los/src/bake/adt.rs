//! WDT tile files, ADT terrain triangles and `_obj0` placements, read as the server's
//! ground does (game-server `crates/server/src/ground.rs`, `terrain_height.rs`).

use glam::{EulerRot, Quat, Vec3};

use super::chunks::{Result, f32_at, f32x3_at, find_reversed, u16_at, u32_at, u32s};
use crate::TILE_SIZE;

const MAID_ENTRY_SIZE: usize = 32;
const MAIN_ENTRY_SIZE: usize = 8;
const MAIN_FLAG_HAS_ADT: u8 = 0x1;
const MPHD_FLAG_GLOBAL_WMO: u32 = 0x1;
const MDDF_ENTRY_SIZE: usize = 36;
/// MDDF flag: `nameId` is a FileDataID, not an MMID index.
const MDDF_FLAG_FILEDATAID: u16 = 0x40;
const MODF_ENTRY_SIZE: usize = 64;
const MODF_FLAG_HAS_SCALE: u16 = 0x4;
const MODF_FLAG_FILEDATAID: u16 = 0x8;
/// Yards per MCNK chunk side and per terrain quad.
const CHUNK_SIZE: f32 = 100.0 / 3.0;
const UNIT_SIZE: f32 = CHUNK_SIZE / 8.0;
const MCNK_HEADER_SIZE: usize = 128;
const MCVT_COUNT: usize = 145;
const MCNK_FLAG_HIGH_RES_HOLES: u32 = 0x10000;

/// A tile's root and `_obj0` ADT FileDataIDs from the WDT MAID.
#[derive(Clone, Copy, Default)]
pub struct TileFiles {
    pub root: u32,
    pub obj0: u32,
}

/// The WDT's tiles indexed `col * 64 + row`; an error for a global-WMO map.
pub fn wdt_tiles(wdt: &[u8]) -> Result<Vec<TileFiles>> {
    let mphd = find_reversed(wdt, b"MPHD").ok_or("WDT has no MPHD")?;
    if u32_at(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO != 0 {
        return Err("global-WMO map: no tiles".into());
    }
    let Some(maid) = find_reversed(wdt, b"MAID") else {
        return terrainless_legacy_tiles(wdt);
    };
    (0..64 * 64)
        .map(|index| {
            let base = index * MAID_ENTRY_SIZE;
            Ok(TileFiles {
                root: u32_at(maid, base)?,
                obj0: u32_at(maid, base + 4)?,
            })
        })
        .collect()
}

fn terrainless_legacy_tiles(wdt: &[u8]) -> Result<Vec<TileFiles>> {
    let main = find_reversed(wdt, b"MAIN").ok_or("WDT has no MAIN or MAID")?;
    if main.len() != 64 * 64 * MAIN_ENTRY_SIZE {
        return Err("WDT MAIN has invalid size".into());
    }
    let has_terrain = main
        .as_chunks::<MAIN_ENTRY_SIZE>()
        .0
        .iter()
        .any(|entry| entry[0] & MAIN_FLAG_HAS_ADT != 0);
    if has_terrain {
        return Err("WDT has active terrain but no MAID".into());
    }
    Ok(vec![TileFiles::default(); 64 * 64])
}

/// The WDT's global-WMO MODF: either a FileDataID or a legacy MWMO name index.
pub fn global_wmo(wdt: &[u8]) -> Result<Option<Placement>> {
    let mphd = find_reversed(wdt, b"MPHD").ok_or("WDT has no MPHD")?;
    if u32_at(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO == 0 {
        return Ok(None);
    }
    let entry = find_reversed(wdt, b"MODF").ok_or("global WDT has no MODF")?;
    let flags = u16_at(entry, 56)?;
    Ok(Some(Placement {
        model: global_wmo_model(wdt, entry, flags)?,
        unique_id: u32_at(entry, 4)?,
        position: f32x3_at(entry, 8)?,
        rotation: f32x3_at(entry, 20)?,
        scale: if flags & MODF_FLAG_HAS_SCALE != 0 {
            f32::from(u16_at(entry, 62)?) / 1024.0
        } else {
            1.0
        },
        doodad_set: u16_at(entry, 58)?,
    }))
}

fn global_wmo_model(wdt: &[u8], entry: &[u8], flags: u16) -> Result<ModelRef> {
    let name_id = u32_at(entry, 0)?;
    if flags & MODF_FLAG_FILEDATAID != 0 {
        return Ok(ModelRef::Fdid(name_id));
    }
    let names = find_reversed(wdt, b"MWMO").ok_or("legacy global WDT has no MWMO")?;
    let name = names
        .split(|&byte| byte == 0)
        .nth(name_id as usize)
        .filter(|name| !name.is_empty())
        .ok_or("global WDT MODF name index outside MWMO")?;
    let path = std::str::from_utf8(name).map_err(|error| format!("MWMO path: {error}"))?;
    Ok(ModelRef::Path(path.to_lowercase().replace('\\', "/")))
}

/// Bevy world position of an ADT MODF/MDDF placement position
/// (`shared::ground::placement_position`).
pub fn placement_position(raw: [f32; 3]) -> Vec3 {
    let center = 32.0 * TILE_SIZE;
    Vec3::new(center - raw[2], raw[1], raw[0] - center)
}

/// Bevy rotation of an ADT MODF/MDDF placement rotation in degrees
/// (`shared::ground::placement_rotation`).
pub fn placement_rotation(rot: [f32; 3]) -> Quat {
    let bank_x = rot[2].to_radians();
    let heading_y = (rot[1] - 180.0).to_radians();
    let attitude_z = (-rot[0]).to_radians();
    Quat::from_euler(EulerRot::YZX, heading_y, attitude_z, bank_x)
}

/// Model file coordinates (X, Y, Z up) in model-local Bevy space.
pub fn file_to_bevy([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, z, -y)
}

/// A placed object: `model` is a FileDataID (or a name to resolve, for MDDF by path).
#[derive(Clone, Debug)]
pub struct Placement {
    pub model: ModelRef,
    pub unique_id: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: f32,
    /// WMO doodad set (MODF only).
    pub doodad_set: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ModelRef {
    Fdid(u32),
    Path(String),
}

impl Placement {
    pub fn world_from_local(&self) -> glam::Affine3A {
        glam::Affine3A::from_scale_rotation_translation(
            Vec3::splat(self.scale),
            placement_rotation(self.rotation),
            placement_position(self.position),
        )
    }
}

/// `_obj0` MDDF doodads and MODF WMOs. MODF entries by path are skipped, as the server's
/// ground skips them.
pub fn placements(obj0: &[u8]) -> Result<(Vec<Placement>, Vec<Placement>)> {
    let names = doodad_names(obj0);
    let mut doodads = Vec::new();
    if let Some(mddf) = find_reversed(obj0, b"MDDF") {
        for entry in mddf.as_chunks::<MDDF_ENTRY_SIZE>().0.iter() {
            let name_id = u32_at(entry, 0)?;
            let flags = u16_at(entry, 34)?;
            let model = if flags & MDDF_FLAG_FILEDATAID != 0 {
                ModelRef::Fdid(name_id)
            } else {
                let name = names.get(name_id as usize).ok_or_else(|| {
                    format!("MDDF name {name_id} past {} MMID names", names.len())
                })?;
                ModelRef::Path(name.clone())
            };
            doodads.push(Placement {
                model,
                unique_id: u32_at(entry, 4)?,
                position: f32x3_at(entry, 8)?,
                rotation: f32x3_at(entry, 20)?,
                scale: f32::from(u16_at(entry, 32)?) / 1024.0,
                doodad_set: 0,
            });
        }
    }
    let mut wmos = Vec::new();
    if let Some(modf) = find_reversed(obj0, b"MODF") {
        for entry in modf.as_chunks::<MODF_ENTRY_SIZE>().0.iter() {
            let flags = u16_at(entry, 56)?;
            if flags & MODF_FLAG_FILEDATAID == 0 {
                continue;
            }
            let scale = if flags & MODF_FLAG_HAS_SCALE != 0 {
                f32::from(u16_at(entry, 62)?) / 1024.0
            } else {
                1.0
            };
            wmos.push(Placement {
                model: ModelRef::Fdid(u32_at(entry, 0)?),
                unique_id: u32_at(entry, 4)?,
                position: f32x3_at(entry, 8)?,
                rotation: f32x3_at(entry, 20)?,
                scale,
                doodad_set: u16_at(entry, 58)?,
            });
        }
    }
    Ok((doodads, wmos))
}

/// MMDX model paths in MMID order.
fn doodad_names(obj0: &[u8]) -> Vec<String> {
    let (Some(mmdx), Some(mmid)) = (find_reversed(obj0, b"MMDX"), find_reversed(obj0, b"MMID"))
    else {
        return Vec::new();
    };
    u32s(mmid)
        .into_iter()
        .map(|offset| {
            let tail = mmdx.get(offset as usize..).unwrap_or_default();
            let end = tail
                .iter()
                .position(|&byte| byte == 0)
                .unwrap_or(tail.len());
            String::from_utf8_lossy(&tail[..end])
                .to_lowercase()
                .replace('\\', "/")
        })
        .collect()
}

/// The root ADT's terrain triangles in Bevy world space, without hole quads. Each quad is
/// the four-triangle fan around its inner vertex the server's height lookup interpolates.
pub fn terrain_triangles(root: &[u8]) -> Result<Vec<[Vec3; 3]>> {
    let mut triangles = Vec::new();
    for (tag, mcnk) in super::chunks::chunks(root) {
        if &tag != b"KNCM" {
            continue;
        }
        if mcnk.len() < MCNK_HEADER_SIZE {
            return Err(format!("MCNK too small: {} bytes", mcnk.len()));
        }
        let corner_x = f32_at(mcnk, 0x68)?;
        let corner_y = f32_at(mcnk, 0x6c)?;
        let base_z = f32_at(mcnk, 0x70)?;
        let holes = if u32_at(mcnk, 0)? & MCNK_FLAG_HIGH_RES_HOLES != 0 {
            u64::from(u32_at(mcnk, 0x14)?) | u64::from(u32_at(mcnk, 0x18)?) << 32
        } else {
            widen_low_res_holes(u16_at(mcnk, 0x3c)?)
        };
        let mcvt = find_reversed(&mcnk[MCNK_HEADER_SIZE..], b"MCVT").ok_or("MCNK has no MCVT")?;
        let heights: Vec<f32> = (0..MCVT_COUNT)
            .map(|index| f32_at(mcvt, index * 4))
            .collect::<Result<_>>()?;
        // Row steps along WoW X, column along WoW Y, both away from the corner.
        let vertex = |row: f32, col: f32, height: f32| {
            Vec3::new(
                corner_x - row * UNIT_SIZE,
                base_z + height,
                -(corner_y - col * UNIT_SIZE),
            )
        };
        for row in 0..8 {
            for col in 0..8 {
                if holes >> (row * 8 + col) & 1 != 0 {
                    continue;
                }
                let outer = |r: usize, c: usize| vertex(r as f32, c as f32, heights[r * 17 + c]);
                let center = vertex(
                    row as f32 + 0.5,
                    col as f32 + 0.5,
                    heights[row * 17 + 9 + col],
                );
                let (tl, tr) = (outer(row, col), outer(row, col + 1));
                let (bl, br) = (outer(row + 1, col), outer(row + 1, col + 1));
                triangles.extend([
                    [tl, tr, center],
                    [tr, br, center],
                    [br, bl, center],
                    [bl, tl, center],
                ]);
            }
        }
    }
    Ok(triangles)
}

/// The 4x4 low-res hole mask (each bit a 2x2 block of quads) as an 8x8 quad mask
/// (`terrain_height::widen_low_res_holes`).
fn widen_low_res_holes(low: u16) -> u64 {
    (0..64)
        .filter(|bit| low >> ((bit / 8 / 2) * 4 + (bit % 8) / 2) & 1 != 0)
        .fold(0, |mask, bit| mask | 1 << bit)
}
