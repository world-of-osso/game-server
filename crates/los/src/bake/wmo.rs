//! WMO opaque render batches and doodad sets.

use glam::Vec3;

use super::adt::file_to_bevy;
use super::chunks::{Result, f32_at, f32x3_at, find_reversed, records, u16_at, u16s, u32_at, u32s};

const MOMT_SIZE: usize = 64;
const MOGP_HEADER_SIZE: usize = 0x44;
/// MOGP flag: antiportal occluder, not drawn.
const MOGP_ANTIPORTAL: u32 = 0x0400_0000;
const MOBA_SIZE: usize = 24;
/// MOBA flag: the material id is the `u16` at 0x0A instead of the byte at 0x17.
const MOBA_MATERIAL_ID_LARGE: u8 = 0x2;
/// MOMT blend mode of an opaque material (`EGxBlend_Opaque`); 1 is alpha-tested, the rest
/// blend.
const BLEND_OPAQUE: u32 = 0;
const MODS_SIZE: usize = 32;
const MODD_SIZE: usize = 40;

pub struct WmoDoodad {
    pub fdid: u32,
    /// WMO file coordinates.
    pub position: [f32; 3],
    /// Rotation in WMO file coordinates (x, y, z, w).
    pub rotation: [f32; 4],
    pub scale: f32,
}

pub struct WmoRoot {
    pub group_fdids: Vec<u32>,
    /// MOMT blend mode per material.
    blend_modes: Vec<u32>,
    /// MODS `(start, count)` per doodad set.
    doodad_sets: Vec<(usize, usize)>,
    doodads: Vec<Option<WmoDoodad>>,
}

impl WmoRoot {
    pub fn parse(root: &[u8]) -> Result<Self> {
        let mohd = find_reversed(root, b"MOHD").ok_or("WMO root has no MOHD")?;
        let group_count = u32_at(mohd, 4)? as usize;
        let mut group_fdids = u32s(find_reversed(root, b"GFID").unwrap_or_default());
        group_fdids.truncate(group_count);
        let blend_modes = find_reversed(root, b"MOMT")
            .unwrap_or_default()
            .as_chunks::<MOMT_SIZE>()
            .0
            .iter()
            .map(|material| u32_at(material, 8))
            .collect::<Result<_>>()?;
        let doodad_sets = find_reversed(root, b"MODS")
            .unwrap_or_default()
            .as_chunks::<MODS_SIZE>()
            .0
            .iter()
            .map(|set| Ok((u32_at(set, 20)? as usize, u32_at(set, 24)? as usize)))
            .collect::<Result<_>>()?;
        let names = u32s(find_reversed(root, b"MODI").unwrap_or_default());
        let doodads = find_reversed(root, b"MODD")
            .unwrap_or_default()
            .as_chunks::<MODD_SIZE>()
            .0
            .iter()
            .map(|entry| {
                let name_index = (u32_at(entry, 0)? & 0x00FF_FFFF) as usize;
                let Some(&fdid) = names.get(name_index).filter(|&&fdid| fdid != 0) else {
                    return Ok(None);
                };
                Ok(Some(WmoDoodad {
                    fdid,
                    position: f32x3_at(entry, 4)?,
                    rotation: [
                        f32_at(entry, 16)?,
                        f32_at(entry, 20)?,
                        f32_at(entry, 24)?,
                        f32_at(entry, 28)?,
                    ],
                    scale: f32_at(entry, 32)?,
                }))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            group_fdids,
            blend_modes,
            doodad_sets,
            doodads,
        })
    }

    /// Doodads of set 0 (always shown) and of `set`.
    pub fn doodads(&self, set: u16) -> impl Iterator<Item = &WmoDoodad> {
        let mut sets = vec![0usize];
        if set != 0 {
            sets.push(set as usize);
        }
        sets.into_iter()
            .filter_map(|set| self.doodad_sets.get(set).copied())
            .flat_map(move |(start, count)| self.doodads.iter().skip(start).take(count).flatten())
    }

    /// Triangles of the group's opaque render batches in WMO-local Bevy space, and the
    /// triangle count of its alpha-tested or blended batches.
    pub fn group_triangles(&self, group: &[u8]) -> Result<(Vec<[Vec3; 3]>, usize)> {
        let mogp = find_reversed(group, b"MOGP").ok_or("WMO group has no MOGP")?;
        if mogp.len() < MOGP_HEADER_SIZE {
            return Err(format!("MOGP too small: {} bytes", mogp.len()));
        }
        if u32_at(mogp, 8)? & MOGP_ANTIPORTAL != 0 {
            return Ok((Vec::new(), 0));
        }
        let body = &mogp[MOGP_HEADER_SIZE..];
        let vertices: Vec<Vec3> = records(
            find_reversed(body, b"MOVT").unwrap_or_default(),
            (
                find_reversed(body, b"MOVT").unwrap_or_default().len() / 12,
                0,
            ),
            12,
            |data, offset| f32x3_at(data, offset).map(file_to_bevy),
        )?;
        let indices = u16s(find_reversed(body, b"MOVI").unwrap_or_default());
        let mut opaque = Vec::new();
        let mut dropped = 0;
        for batch in find_reversed(body, b"MOBA")
            .unwrap_or_default()
            .as_chunks::<MOBA_SIZE>()
            .0
            .iter()
        {
            let start = u32_at(batch, 0x0C)? as usize;
            let count = usize::from(u16_at(batch, 0x10)?);
            let material = if batch[0x16] & MOBA_MATERIAL_ID_LARGE != 0 {
                usize::from(u16_at(batch, 0x0A)?)
            } else {
                usize::from(batch[0x17])
            };
            let blend = *self.blend_modes.get(material).ok_or_else(|| {
                format!(
                    "MOBA material {material} past {} MOMT",
                    self.blend_modes.len()
                )
            })?;
            if blend != BLEND_OPAQUE {
                dropped += count / 3;
                continue;
            }
            let batch_indices = indices.get(start..start + count).ok_or_else(|| {
                format!("MOBA indices {start}+{count} past {} MOVI", indices.len())
            })?;
            for tri in batch_indices.as_chunks::<3>().0.iter() {
                let corner = |index: u16| {
                    vertices
                        .get(usize::from(index))
                        .copied()
                        .ok_or_else(|| format!("MOVI index {index} past {} MOVT", vertices.len()))
                };
                opaque.push([corner(tri[0])?, corner(tri[1])?, corner(tri[2])?]);
            }
        }
        Ok((opaque, dropped))
    }
}
