//! Offline walkability visualization through the same validated tile codec as queries.
use super::{decode_packets, validate_packet};
use crate::WALK_FLAG;
use recastnavigation_sys::{dtMeshHeader, dtPoly};
use std::mem::size_of;
use std::path::Path;

pub fn read_walkable_polygons(dir: &Path) -> Result<Vec<Vec<[f32; 3]>>, String> {
    let entries = std::fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let mut polygons = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().is_none_or(|extension| extension != "nav") {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        if bytes.is_empty() {
            continue;
        }
        for packet in decode_packets(&bytes)? {
            polygons.extend(packet_polygons(packet)?);
        }
    }
    if polygons.is_empty() {
        return Err(format!("No walkable polygons in {}", dir.display()));
    }
    Ok(polygons)
}

fn packet_polygons(bytes: &[u8]) -> Result<Vec<Vec<[f32; 3]>>, String> {
    validate_packet(bytes)?;
    // SAFETY: the shared validator established a complete trusted Detour buffer;
    // file byte alignment is not assumed.
    let header = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<dtMeshHeader>()) };
    let vertex_start = (size_of::<dtMeshHeader>() + 3) & !3;
    let polygon_start = vertex_start + header.vertCount as usize * 12;
    let vertices: Vec<[f32; 3]> = bytes[vertex_start..polygon_start]
        .chunks_exact(12)
        .map(read_vertex)
        .collect();
    let mut output = Vec::new();
    for index in 0..header.polyCount as usize {
        let start = polygon_start + index * size_of::<dtPoly>();
        // SAFETY: validated array lengths; unaligned POD read within this packet.
        let polygon = unsafe { std::ptr::read_unaligned(bytes[start..].as_ptr().cast::<dtPoly>()) };
        if polygon.flags & WALK_FLAG == 0 || polygon.areaAndtype >> 6 != 0 {
            continue;
        }
        output.push(polygon_vertices(&polygon, &vertices)?);
    }
    Ok(output)
}

fn read_vertex(bytes: &[u8]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let offset = axis * 4;
        f32::from_ne_bytes(bytes[offset..offset + 4].try_into().unwrap())
    })
}

fn polygon_vertices(polygon: &dtPoly, vertices: &[[f32; 3]]) -> Result<Vec<[f32; 3]>, String> {
    let count = usize::from(polygon.vertCount);
    let indices = polygon
        .verts
        .get(..count)
        .ok_or("invalid polygon vertex count")?;
    indices
        .iter()
        .map(|&index| {
            let point = *vertices
                .get(usize::from(index))
                .ok_or("invalid polygon vertex index")?;
            if !point.iter().all(|value| value.is_finite()) {
                return Err("non-finite navigation vertex".into());
            }
            Ok(point)
        })
        .collect()
}
