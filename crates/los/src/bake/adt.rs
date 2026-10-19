//! Server placement and terrain-mesh policy over shared ADT/WDT readers.
use super::chunks::Result;
use crate::TILE_SIZE;
use glam::{EulerRot, Quat, Vec3};
use wow_adt::{adt, adt_obj, wdt};
const UNIT_SIZE: f32 = adt::UNIT_SIZE;

#[derive(Clone, Copy, Default)]
pub struct TileFiles {
    pub root: u32,
    pub obj0: u32,
}

pub fn wdt_tiles(data: &[u8]) -> Result<Vec<TileFiles>> {
    if wdt::parse_wdt_mphd_flags(data)?.raw & 1 != 0 {
        return Err("global-WMO map: no tiles".into());
    }
    if let Some(slots) = wdt::parse_wdt_maid_file_ids(data)? {
        return Ok(slots
            .into_iter()
            .map(|slot| TileFiles {
                root: slot.root,
                obj0: slot.obj0,
            })
            .collect());
    }
    let tiles = wdt::parse_wdt_tiles(data)?;
    if !tiles.active.is_empty() {
        return Err("WDT has active terrain but no MAID".into());
    }
    Ok(vec![TileFiles::default(); 64 * 64])
}

pub fn global_wmo(data: &[u8]) -> Result<Option<Placement>> {
    wdt::parse_wdt_global_wmo(data)?
        .map(|entry| {
            Ok(Placement {
                model: model_ref(entry.fdid, entry.path)?,
                unique_id: entry.unique_id,
                position: entry.position,
                rotation: entry.rotation,
                scale: entry.scale,
                doodad_set: entry.doodad_set,
            })
        })
        .transpose()
}

fn model_ref(fdid: Option<u32>, path: Option<String>) -> Result<ModelRef> {
    if let Some(fdid) = fdid {
        return Ok(ModelRef::Fdid(fdid));
    }
    path.map(|path| ModelRef::Path(path.to_lowercase().replace('\\', "/")))
        .ok_or_else(|| "placement has no FDID or model name".into())
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

/// Preserve the server's FDID-only WMO placement selection and MDDF name policy.
pub fn placements(data: &[u8]) -> Result<(Vec<Placement>, Vec<Placement>)> {
    let parsed = adt_obj::load_adt_obj0(data)?;
    let doodads = parsed
        .doodads
        .into_iter()
        .map(|entry| {
            Ok(Placement {
                model: model_ref(entry.fdid, entry.path)?,
                unique_id: entry.unique_id,
                position: entry.position,
                rotation: entry.rotation,
                scale: entry.scale,
                doodad_set: 0,
            })
        })
        .collect::<Result<_>>()?;
    let wmos = parsed
        .wmos
        .into_iter()
        .filter_map(|entry| {
            entry.fdid.map(|fdid| Placement {
                model: ModelRef::Fdid(fdid),
                unique_id: entry.unique_id,
                position: entry.position,
                rotation: entry.rotation,
                scale: entry.scale,
                doodad_set: entry.doodad_set,
            })
        })
        .collect();
    Ok((doodads, wmos))
}

/// Original quad fan, hole selection, winding and floating-point arithmetic.
pub fn terrain_triangles(root: &[u8]) -> Result<Vec<[Vec3; 3]>> {
    let mut triangles = Vec::new();
    for chunk in adt::parse_terrain_height_chunks(root)? {
        let [corner_x, corner_y, base_z] = chunk.origin;
        let heights = chunk.heights;
        let holes = chunk
            .holes_high_res
            .unwrap_or_else(|| widen_low_res_holes(chunk.holes_low_res));
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
