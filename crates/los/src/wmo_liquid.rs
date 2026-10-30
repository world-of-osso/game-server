//! Authored WMO group liquids, shared by runtime queries and navigation baking.
use glam::{Affine3A, Vec3};
pub use shared::movement::SWIM_DEPTH;
use wow_wmo::parser::{self, WmoGroupHeader, WmoLiquid, WmoRootFlags};

const UNIT_SIZE: f32 = 100.0 / 3.0 / 8.0;
const MOGP_HEADER_SIZE: usize = 0x44;

/// Read every group named by a WMO root from the extracted model cache.
pub fn load_model(data: &std::path::Path, root_fdid: u32) -> Result<Vec<LiquidTriangle>, String> {
    let read = |fdid| {
        let path = data.join("models").join(format!("{fdid}.wmo"));
        std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))
    };
    let root = read(root_fdid)?;
    let parsed = parser::load_wmo_root(&root)?;
    let mut triangles = Vec::new();
    for fdid in parsed
        .group_file_data_ids
        .into_iter()
        .take(parsed.n_groups as usize)
    {
        let group = read(fdid)?;
        triangles.extend(
            group_triangles(parsed.flags, &group)
                .map_err(|error| format!("group {fdid}: {error}"))?,
        );
    }
    Ok(triangles)
}

#[derive(Clone, Debug)]
pub struct LiquidTriangle {
    pub vertices: [Vec3; 3],
    pub liquid_type: u16,
}

/// Parse active surface triangles in model-local Bevy coordinates.
pub fn group_triangles(root: WmoRootFlags, bytes: &[u8]) -> Result<Vec<LiquidTriangle>, String> {
    let mogp = parser::find_mogp(bytes)?;
    let group = parser::parse_mogp_header(mogp)?;
    let body = mogp.get(MOGP_HEADER_SIZE..).ok_or("short MOGP")?;
    let Some(payload) = wow_chunk::find_reversed_chunk(body, b"MLIQ")? else {
        return Ok(Vec::new());
    };
    let liquid = parser::parse_mliq(payload)?;
    let Some(liquid_type) = group_liquid_type(root, &group) else {
        return Ok(Vec::new());
    };
    triangles(&liquid, liquid_type)
}

fn group_liquid_type(root: WmoRootFlags, group: &WmoGroupHeader) -> Option<u16> {
    let liquid = group.group_liquid;
    let basic = |value: u32| match value & 3 {
        0 if group.flags & 0x8_0000 != 0 => 14,
        0 => 13,
        1 => 14,
        2 => 19,
        _ => 20,
    };
    let id = if root.use_liquid_type_dbc_id {
        if liquid < 21 {
            basic(liquid.wrapping_sub(1))
        } else {
            liquid
        }
    } else if liquid == 15 {
        0
    } else if liquid < 20 {
        basic(liquid)
    } else {
        liquid + 1
    };
    u16::try_from(id).ok().filter(|&id| id != 0)
}

fn triangles(liquid: &WmoLiquid, liquid_type: u16) -> Result<Vec<LiquidTriangle>, String> {
    let h = &liquid.header;
    let (xv, yv, xt, yt) = (h.x_verts, h.y_verts, h.x_tiles, h.y_tiles);
    if xv <= 0 || yv <= 0 || xt < 0 || yt < 0 || xt >= xv || yt >= yv {
        return Err("invalid MLIQ grid dimensions".into());
    }
    let (xv, xt, yt) = (xv as usize, xt as usize, yt as usize);
    let mut result = Vec::new();
    for y in 0..yt {
        for x in 0..xt {
            let tile = liquid.tiles.get(y * xt + x).ok_or("missing MLIQ tile")?;
            if tile.liquid_type == 15 {
                continue;
            }
            let vertex = |dx: usize, dy: usize| -> Result<Vec3, String> {
                let height = liquid
                    .vertices
                    .get((y + dy) * xv + x + dx)
                    .ok_or("missing MLIQ vertex")?
                    .height;
                Ok(Vec3::new(
                    h.position[0] + (x + dx) as f32 * UNIT_SIZE,
                    height,
                    -(h.position[1] + (y + dy) as f32 * UNIT_SIZE),
                ))
            };
            let a = vertex(0, 0)?;
            let b = vertex(1, 0)?;
            let c = vertex(1, 1)?;
            let d = vertex(0, 1)?;
            result.push(LiquidTriangle {
                vertices: [a, b, c],
                liquid_type,
            });
            result.push(LiquidTriangle {
                vertices: [a, c, d],
                liquid_type,
            });
        }
    }
    Ok(result)
}

impl LiquidTriangle {
    pub fn transformed(&self, transform: Affine3A) -> Self {
        Self {
            vertices: self.vertices.map(|v| transform.transform_point3(v)),
            liquid_type: self.liquid_type,
        }
    }

    /// Intersect the surface with a vertical line in world space.
    pub fn level_at(&self, x: f32, z: f32) -> Option<f32> {
        let [a, b, c] = self.vertices;
        let det = (b.z - c.z) * (a.x - c.x) + (c.x - b.x) * (a.z - c.z);
        if det.abs() < 1e-6 {
            return None;
        }
        let u = ((b.z - c.z) * (x - c.x) + (c.x - b.x) * (z - c.z)) / det;
        let v = ((c.z - a.z) * (x - c.x) + (a.x - c.x) * (z - c.z)) / det;
        let w = 1.0 - u - v;
        if u < -1e-5 || v < -1e-5 || w < -1e-5 {
            return None;
        }
        Some(u * a.y + v * b.y + w * c.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_wmo::parser::{WmoLiquidHeader, WmoLiquidTile, WmoLiquidVertex};

    #[test]
    fn active_tiles_transform_and_sample_heights() {
        let liquid = WmoLiquid {
            header: WmoLiquidHeader {
                x_verts: 3,
                y_verts: 2,
                x_tiles: 2,
                y_tiles: 1,
                position: [0.0, 0.0, 0.0],
                material_id: 0,
            },
            vertices: [1.0, 2.0, 3.0, 1.0, 2.0, 3.0]
                .into_iter()
                .map(|height| WmoLiquidVertex {
                    raw: [0; 4],
                    height,
                })
                .collect(),
            tiles: [0, 15]
                .into_iter()
                .map(|liquid_type| WmoLiquidTile {
                    liquid_type,
                    fishable: false,
                    shared: false,
                })
                .collect(),
        };
        let local = triangles(&liquid, 13).unwrap();
        assert_eq!(local.len(), 2);
        let transform = Affine3A::from_translation(Vec3::new(10.0, 5.0, 20.0))
            * Affine3A::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let world: Vec<_> = local.iter().map(|t| t.transformed(transform)).collect();
        let p = transform.transform_point3(Vec3::new(UNIT_SIZE * 0.25, 0.0, -UNIT_SIZE * 0.25));
        let height = world.iter().find_map(|t| t.level_at(p.x, p.z)).unwrap();
        assert!((height - 6.25).abs() < 0.001);
        let hidden = transform.transform_point3(Vec3::new(UNIT_SIZE * 1.5, 0.0, -UNIT_SIZE * 0.5));
        assert!(
            world
                .iter()
                .all(|t| t.level_at(hidden.x, hidden.z).is_none())
        );
        assert!(world.iter().all(|t| t.liquid_type == 13));
    }
}
