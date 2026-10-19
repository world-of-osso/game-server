//! Server opaque-batch and doodad-set policy over shared WMO readers.
use glam::Vec3;
use wow_wmo::parser;

use super::adt::file_to_bevy;
use super::chunks::{Result, find_reversed};

const MOGP_HEADER_SIZE: usize = 0x44;
const MOGP_ANTIPORTAL: u32 = 0x0400_0000;
const BLEND_OPAQUE: u32 = 0;

pub struct WmoDoodad {
    pub fdid: u32,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: f32,
}

pub struct WmoRoot {
    pub group_fdids: Vec<u32>,
    blend_modes: Vec<u32>,
    doodad_sets: Vec<(usize, usize)>,
    doodads: Vec<Option<WmoDoodad>>,
}

impl WmoRoot {
    pub fn parse(root: &[u8]) -> Result<Self> {
        let parsed = parser::load_wmo_root(root)?;
        let mut group_fdids = parsed.group_file_data_ids;
        group_fdids.truncate(parsed.n_groups as usize);
        let doodads = parsed
            .doodad_defs
            .into_iter()
            .map(|entry| {
                let &fdid = parsed
                    .doodad_file_ids
                    .get(entry.name_offset as usize)
                    .filter(|&&fdid| fdid != 0)?;
                Some(WmoDoodad {
                    fdid,
                    position: entry.position,
                    rotation: entry.rotation,
                    scale: entry.scale,
                })
            })
            .collect();
        Ok(Self {
            group_fdids,
            blend_modes: parsed
                .materials
                .into_iter()
                .map(|material| material.blend_mode)
                .collect(),
            doodad_sets: parsed
                .doodad_sets
                .into_iter()
                .map(|set| (set.start_doodad as usize, set.n_doodads as usize))
                .collect(),
            doodads,
        })
    }

    /// Existing selection: set 0 plus the requested set, preserving entry order.
    pub fn doodads(&self, set: u16) -> impl Iterator<Item = &WmoDoodad> {
        let mut sets = vec![0usize];
        if set != 0 {
            sets.push(set as usize);
        }
        sets.into_iter()
            .filter_map(|set| self.doodad_sets.get(set).copied())
            .flat_map(move |(start, count)| self.doodads.iter().skip(start).take(count).flatten())
    }

    pub fn group_triangles(&self, group: &[u8]) -> Result<(Vec<[Vec3; 3]>, usize)> {
        let mogp = find_reversed(group, b"MOGP").ok_or("WMO group has no MOGP")?;
        if mogp.len() < MOGP_HEADER_SIZE {
            return Err(format!("MOGP too small: {} bytes", mogp.len()));
        }
        if parser::parse_mogp_header(mogp)?.flags & MOGP_ANTIPORTAL != 0 {
            return Ok((Vec::new(), 0));
        }
        let body = &mogp[MOGP_HEADER_SIZE..];
        let vertices: Vec<Vec3> =
            parser::parse_vec3_array(find_reversed(body, b"MOVT").unwrap_or_default())?
                .into_iter()
                .map(file_to_bevy)
                .collect();
        let indices = parser::parse_u16_array(find_reversed(body, b"MOVI").unwrap_or_default());
        let batches = parser::parse_moba(find_reversed(body, b"MOBA").unwrap_or_default())?;
        let mut opaque = Vec::new();
        let mut dropped = 0;
        for batch in batches {
            let start = batch.start_index as usize;
            let count = usize::from(batch.count);
            let material = usize::from(batch.material_id);
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
            for triangle in batch_indices.as_chunks::<3>().0 {
                let corner = |index: u16| {
                    vertices
                        .get(usize::from(index))
                        .copied()
                        .ok_or_else(|| format!("MOVI index {index} past {} MOVT", vertices.len()))
                };
                opaque.push([
                    corner(triangle[0])?,
                    corner(triangle[1])?,
                    corner(triangle[2])?,
                ]);
            }
        }
        Ok((opaque, dropped))
    }
}
