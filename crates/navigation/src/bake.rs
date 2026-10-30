//! Recast voxelization of the LOS/ground collector's world collision triangles.
use crate::*;
use glam::{Vec3, Vec3Swizzles};
use recastnavigation_sys::*;

pub const VERTEX_CAPACITY_ERROR: &str = "RECAST_VERTEX_CAPACITY_EXCEEDED";
pub const POLYGON_CAPACITY_ERROR: &str = "RECAST_POLYGON_NEIGHBOUR_CAPACITY_EXCEEDED";
const BORDER_CELLS: i32 = RADIUS_CELLS + 3;
const HEIGHT_CELLS: i32 = ceil_cells(AGENT_HEIGHT, CELL_HEIGHT);
const CLIMB_CELLS: i32 = (AGENT_CLIMB / CELL_HEIGHT) as i32;
const RADIUS_CELLS: i32 = ceil_cells(AGENT_RADIUS, CELL_SIZE);

const fn ceil_cells(value: f32, size: f32) -> i32 {
    let ratio = value / size;
    let floor = ratio as i32;
    floor + (ratio > floor as f32) as i32
}

/// A WoW tile contains 4×4 Detour tiles to keep peak voxelization memory bounded.
/// `triangles` includes neighboring geometry: border erosion must see across tile seams.
pub fn bake_tile(triangles: &[[Vec3; 3]], tile: (u32, u32)) -> Result<Vec<u8>, String> {
    let geometry = los::bake::CollectedGeometry {
        triangles: triangles.to_vec(),
        obstacles: Vec::new(),
    };
    bake_collected(&geometry, &[], tile)
}

pub fn bake_collected(
    geometry: &los::bake::CollectedGeometry,
    water: &[(Vec3, Vec3)],
    tile: (u32, u32),
) -> Result<Vec<u8>, String> {
    bake_collected_on_grid(geometry, water, tile, SUBDIVISIONS)
}

pub fn bake_collected_on_grid(
    geometry: &los::bake::CollectedGeometry,
    water: &[(Vec3, Vec3)],
    tile: (u32, u32),
    subdivisions: u32,
) -> Result<Vec<u8>, String> {
    let subtile_size = grid_subtile_size(subdivisions)?;
    let triangles = &geometry.triangles;
    let obstacles: std::collections::HashSet<_> =
        geometry.obstacles.iter().map(triangle_key).collect();
    if triangles.iter().flatten().any(|p| !p.is_finite()) {
        return Err("non-finite collision vertex".into());
    }
    let (x, z) = tile_bounds(tile);
    let mut packets = Vec::new();
    for row in 0..subdivisions {
        for col in 0..subdivisions {
            let min = Vec3::new(
                x + col as f32 * subtile_size,
                0.0,
                z + row as f32 * subtile_size,
            );
            let coords = (
                (63 - tile.1) * subdivisions + col,
                tile.0 * subdivisions + row,
            );
            let layers = bake_subtile(triangles, &obstacles, water, min, coords, subdivisions)
                .map_err(|error| format!("tile {tile:?}, Detour subtile {coords:?}: {error}"))?;
            packets.extend(layers);
        }
    }
    if packets.is_empty() {
        return Ok(Vec::new());
    }
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&(packets.len() as u32).to_le_bytes());
    for packet in packets {
        bytes.extend_from_slice(&(packet.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&packet);
    }
    Ok(bytes)
}

struct RasterInput {
    cells: i32,
    vertices: Vec<f32>,
    indices: Vec<i32>,
    areas: Vec<u8>,
    min: [f32; 3],
    max: [f32; 3],
}

impl RasterInput {
    fn clipped(
        triangles: &[[Vec3; 3]],
        obstacles: &std::collections::HashSet<[u32; 9]>,
        origin: Vec3,
        subdivisions: u32,
    ) -> Self {
        let border = BORDER_CELLS as f32 * CELL_SIZE;
        let mut min = origin - Vec3::new(border, 0.0, border);
        let size = TILE_SIZE / subdivisions as f32;
        let mut max = origin + Vec3::new(size + border, 0.0, size + border);
        min.y = f32::MAX;
        max.y = f32::MIN;
        let mut input = Self {
            cells: (TILE_CELLS / subdivisions) as i32,
            vertices: Vec::new(),
            indices: Vec::new(),
            areas: Vec::new(),
            min: [0.0; 3],
            max: [0.0; 3],
        };
        for tri in triangles {
            let tmin = tri[0].min(tri[1]).min(tri[2]);
            let tmax = tri[0].max(tri[1]).max(tri[2]);
            let overlaps = tmax.xz().cmpge(min.xz()).all() && tmin.xz().cmple(max.xz()).all();
            if !overlaps {
                continue;
            }
            min.y = min.y.min(tmin.y);
            max.y = max.y.max(tmax.y);
            let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
            // Ground collision is two-sided, as shared::ground::WmoGroupCollision is.
            let walkable = normal.y.abs() >= AGENT_SLOPE.to_radians().cos()
                && !obstacles.contains(&triangle_key(tri));
            input.areas.push(if walkable {
                RC_WALKABLE_AREA
            } else {
                RC_NULL_AREA
            });
            for point in tri {
                input.indices.push((input.vertices.len() / 3) as i32);
                input.vertices.extend_from_slice(&point.to_array());
            }
        }
        min.y -= CELL_HEIGHT;
        max.y += AGENT_HEIGHT;
        input.min = min.to_array();
        input.max = max.to_array();
        input
    }

    /// Split over-height input around its walkable surfaces. Each layer also
    /// receives the nearby portions of walls and ceilings.
    fn vertical_bands(self) -> Result<Vec<Self>, String> {
        const SPAN_LIMIT: f32 = 8190.0 * CELL_HEIGHT;
        const MAX_LAYERS: usize = 32; // Detour's MAX_NEIS during tile linking.
        if self.max[1] - self.min[1] <= SPAN_LIMIT {
            return Ok(vec![self]);
        }
        let input = self.clip_to_bounds(self.min, self.max);
        let mut intervals: Vec<_> = input
            .vertices
            .chunks_exact(9)
            .zip(&input.areas)
            .filter(|(_, area)| **area == RC_WALKABLE_AREA)
            .map(|(tri, _)| {
                (
                    tri[1].min(tri[4]).min(tri[7]),
                    tri[1].max(tri[4]).max(tri[7]),
                )
            })
            .collect();
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let padding = AGENT_HEIGHT + AGENT_CLIMB + 2.0 * CELL_HEIGHT;
        let mut components: Vec<(f32, f32)> = Vec::new();
        for (min, max) in intervals {
            if let Some(last) = components.last_mut()
                && min <= last.1 + padding
            {
                last.1 = last.1.max(max);
            } else {
                components.push((min, max));
            }
        }
        let mut groups: Vec<(f32, f32)> = Vec::new();
        for (min, max) in components {
            if max - min + 2.0 * padding > SPAN_LIMIT {
                return Err(format!(
                    "walkable geometry has no safe split within Recast span limit: {min}..{max}"
                ));
            }
            if let Some(last) = groups.last_mut()
                && max - last.0 + 2.0 * padding <= SPAN_LIMIT
            {
                last.1 = max;
            } else {
                groups.push((min, max));
            }
        }
        if groups.len() > MAX_LAYERS {
            return Err(format!(
                "vertical geometry requires {} Detour layers (maximum {MAX_LAYERS})",
                groups.len()
            ));
        }
        Ok(groups
            .into_iter()
            .map(|(min, max)| {
                let mut lower = input.min;
                let mut upper = input.max;
                lower[1] = min - padding;
                upper[1] = max + padding;
                input.clip_to_bounds(lower, upper)
            })
            .collect())
    }

    fn clip_to_bounds(&self, min: [f32; 3], max: [f32; 3]) -> Self {
        let mut result = Self {
            cells: self.cells,
            vertices: Vec::new(),
            indices: Vec::new(),
            areas: Vec::new(),
            min,
            max,
        };
        for (tri, &area) in self.vertices.chunks_exact(9).zip(&self.areas) {
            let mut polygon = vec![
                Vec3::from_slice(&tri[0..3]),
                Vec3::from_slice(&tri[3..6]),
                Vec3::from_slice(&tri[6..9]),
            ];
            for axis in 0..3 {
                polygon = clip_polygon(polygon, axis, min[axis], true);
                polygon = clip_polygon(polygon, axis, max[axis], false);
            }
            for i in 1..polygon.len().saturating_sub(1) {
                let base = result.vertices.len() / 3;
                for point in [polygon[0], polygon[i], polygon[i + 1]] {
                    result.vertices.extend_from_slice(&point.to_array());
                }
                result
                    .indices
                    .extend([base as i32, base as i32 + 1, base as i32 + 2]);
                result.areas.push(area);
            }
        }
        result
    }
}

fn clip_polygon(polygon: Vec<Vec3>, axis: usize, plane: f32, keep_above: bool) -> Vec<Vec3> {
    let mut result = Vec::new();
    let Some(mut previous) = polygon.last().copied() else {
        return result;
    };
    let inside = |p: Vec3| {
        if keep_above {
            p[axis] >= plane
        } else {
            p[axis] <= plane
        }
    };
    for current in polygon {
        if inside(current) != inside(previous) {
            let t = (plane - previous[axis]) / (current[axis] - previous[axis]);
            result.push(previous.lerp(current, t));
        }
        if inside(current) {
            result.push(current);
        }
        previous = current;
    }
    result
}

/// Owns every Recast allocation, including on a failed pipeline step.
struct RecastBuild {
    context: *mut rcContext,
    heightfield: *mut rcHeightfield,
    compact: *mut rcCompactHeightfield,
    contours: *mut rcContourSet,
    mesh: *mut rcPolyMesh,
    detail: *mut rcPolyMeshDetail,
}

impl RecastBuild {
    fn allocate() -> Result<Self, String> {
        // SAFETY: upstream allocators return newly owned allocations or null.
        let build = unsafe {
            Self {
                context: CreateContext(false),
                heightfield: rcAllocHeightfield(),
                compact: rcAllocCompactHeightfield(),
                contours: rcAllocContourSet(),
                mesh: rcAllocPolyMesh(),
                detail: rcAllocPolyMeshDetail(),
            }
        };
        let failed = [
            build.context.is_null(),
            build.heightfield.is_null(),
            build.compact.is_null(),
            build.contours.is_null(),
            build.mesh.is_null(),
            build.detail.is_null(),
        ]
        .into_iter()
        .any(|missing| missing);
        if failed {
            return Err("Recast allocation failed".into());
        }
        Ok(build)
    }

    fn rasterize(&self, input: &RasterInput) -> Result<(), String> {
        let side = input.cells + 2 * BORDER_CELLS;
        // SAFETY: buffers are complete triangle soup; indices address those vertices.
        unsafe {
            require(
                rcCreateHeightfield(
                    self.context,
                    self.heightfield,
                    side,
                    side,
                    input.min.as_ptr(),
                    input.max.as_ptr(),
                    CELL_SIZE,
                    CELL_HEIGHT,
                ),
                "heightfield",
            )?;
            require(
                rcRasterizeTriangles(
                    self.context,
                    input.vertices.as_ptr(),
                    (input.vertices.len() / 3) as i32,
                    input.indices.as_ptr(),
                    input.areas.as_ptr(),
                    input.areas.len() as i32,
                    self.heightfield,
                    CLIMB_CELLS,
                ),
                "rasterization",
            )?;
            // Do not promote steep/vertical obstacle faces to walkable just because
            // their top lies inside the 1.6-yard grounded step budget.
            rcFilterLedgeSpans(self.context, HEIGHT_CELLS, CLIMB_CELLS, self.heightfield);
            rcFilterWalkableLowHeightSpans(self.context, HEIGHT_CELLS, self.heightfield);
            require(
                rcBuildCompactHeightfield(
                    self.context,
                    HEIGHT_CELLS,
                    CLIMB_CELLS,
                    self.heightfield,
                    self.compact,
                ),
                "compact heightfield",
            )?;
            require(
                rcErodeWalkableArea(self.context, RADIUS_CELLS, self.compact),
                "radius erosion",
            )?;
        }
        Ok(())
    }

    fn polygonize(&self, water: &[(Vec3, Vec3)]) -> Result<(), String> {
        // SAFETY: the previous pipeline step populated compact; every output is owned.
        unsafe {
            for (min, max) in water {
                rcMarkBoxArea(
                    self.context,
                    min.to_array().as_ptr(),
                    max.to_array().as_ptr(),
                    WATER_AREA,
                    self.compact,
                );
            }
            require(
                rcBuildDistanceField(self.context, self.compact),
                "distance field",
            )?;
            require(
                rcBuildRegions(self.context, self.compact, BORDER_CELLS, 8 * 8, 20 * 20),
                "regions",
            )?;
            require(
                rcBuildContours(
                    self.context,
                    self.compact,
                    0.5,
                    48,
                    self.contours,
                    rcBuildContoursFlags_RC_CONTOUR_TESS_WALL_EDGES as i32,
                ),
                "contours",
            )?;
            if !rcBuildPolyMesh(self.context, self.contours, 6, self.mesh) {
                return Err(self.polygon_mesh_failure());
            }
            // Detour adds one to neighbour indices and reserves the high bit for portals.
            if (*self.mesh).npolys >= 0x8000 {
                return Err(format!(
                    "{POLYGON_CAPACITY_ERROR}: {} polygons",
                    (*self.mesh).npolys
                ));
            }
            if (*self.mesh).npolys > 0 {
                require(
                    rcBuildPolyMeshDetail(
                        self.context,
                        self.mesh,
                        self.compact,
                        CELL_SIZE * 6.0,
                        CELL_HEIGHT,
                        self.detail,
                    ),
                    "detail mesh",
                )?;
            }
        }
        Ok(())
    }

    fn polygon_mesh_failure(&self) -> String {
        // SAFETY: contour construction succeeded; Recast owns these arrays.
        let (contours, mesh) = unsafe { (&*self.contours, &*self.mesh) };
        let contours = if contours.nconts > 0 {
            unsafe { std::slice::from_raw_parts(contours.conts, contours.nconts as usize) }
        } else {
            &[]
        };
        let vertices: i32 = contours
            .iter()
            .filter(|c| c.nverts >= 3)
            .map(|c| c.nverts)
            .sum();
        let triangles: i32 = contours
            .iter()
            .filter(|c| c.nverts >= 3)
            .map(|c| c.nverts - 2)
            .sum();
        let unique_vertices = count_merged_contour_vertices(contours);
        let limit = if vertices >= 0xfffe {
            VERTEX_CAPACITY_ERROR
        } else {
            "internal polygonization/allocation failure (not contour vertex capacity)"
        };
        format!(
            "Recast polygon mesh failed: {limit}; contour_vertices={vertices}, deduplicated_vertices={unique_vertices}, triangle_capacity={triangles}, contours={}, mesh_vertices={}, mesh_polygons={}, nvp=6",
            contours.len(),
            mesh.nverts,
            mesh.npolys,
        )
    }

    fn encode_detour(&self, coords: (u32, u32), layer: usize) -> Result<Vec<u8>, String> {
        // SAFETY: polygonization completed; slices use upstream's allocation counts.
        let (mesh, detail) = unsafe { (&*self.mesh, &*self.detail) };
        if mesh.npolys == 0 {
            return Ok(Vec::new());
        }
        let flags = vec![WALK_FLAG; mesh.npolys as usize];
        let mut params: dtNavMeshCreateParams = unsafe { std::mem::zeroed() };
        params.verts = mesh.verts;
        params.vertCount = mesh.nverts;
        params.polys = mesh.polys;
        params.polyFlags = flags.as_ptr();
        params.polyAreas = mesh.areas;
        params.polyCount = mesh.npolys;
        params.nvp = mesh.nvp;
        params.detailMeshes = detail.meshes;
        params.detailVerts = detail.verts;
        params.detailVertsCount = detail.nverts;
        params.detailTris = detail.tris;
        params.detailTriCount = detail.ntris;
        params.tileX = coords.0 as i32;
        params.tileY = coords.1 as i32;
        params.tileLayer = layer as i32;
        params.bmin = mesh.bmin;
        params.bmax = mesh.bmax;
        params.walkableHeight = AGENT_HEIGHT;
        params.walkableRadius = AGENT_RADIUS;
        params.walkableClimb = AGENT_CLIMB;
        params.cs = CELL_SIZE;
        params.ch = CELL_HEIGHT;
        params.buildBvTree = true;
        let mut data = std::ptr::null_mut();
        let mut size = 0;
        // SAFETY: params references owned Recast arrays; Detour copies them.
        unsafe {
            require(
                dtCreateNavMeshData(&mut params, &mut data, &mut size),
                "Detour serialization",
            )?;
            let bytes = std::slice::from_raw_parts(data, size as usize).to_vec();
            dtFree(data.cast());
            Ok(bytes)
        }
    }
}

impl Drop for RecastBuild {
    fn drop(&mut self) {
        // SAFETY: all pointers are individually owned; upstream free accepts null.
        unsafe {
            rcFreePolyMeshDetail(self.detail);
            rcFreePolyMesh(self.mesh);
            rcFreeContourSet(self.contours);
            rcFreeCompactHeightfield(self.compact);
            rcFreeHeightField(self.heightfield);
            if !self.context.is_null() {
                DeleteContext(self.context);
            }
        }
    }
}

fn count_merged_contour_vertices(contours: &[rcContour]) -> usize {
    let mut unique = std::collections::HashMap::<(u16, u16), Vec<u16>>::new();
    for contour in contours.iter().filter(|c| c.nverts >= 3) {
        // SAFETY: successful contour construction owns four ints per vertex.
        let verts =
            unsafe { std::slice::from_raw_parts(contour.verts, contour.nverts as usize * 4) };
        for vertex in verts.chunks_exact(4) {
            let (x, y, z) = (vertex[0] as u16, vertex[1] as u16, vertex[2] as u16);
            let heights = unique.entry((x, z)).or_default();
            // Match pinned Recast addVertex's coordinate/height deduplication.
            if !heights.iter().rev().any(|height| height.abs_diff(y) <= 2) {
                heights.push(y);
            }
        }
    }
    unique.values().map(Vec::len).sum()
}

fn require(ok: bool, step: &str) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(format!("Recast {step} failed"))
    }
}

fn triangle_key(tri: &[Vec3; 3]) -> [u32; 9] {
    let mut key = [0; 9];
    for (i, point) in tri.iter().enumerate() {
        key[i * 3..i * 3 + 3].copy_from_slice(&point.to_array().map(f32::to_bits));
    }
    key
}

fn bake_subtile(
    triangles: &[[Vec3; 3]],
    obstacles: &std::collections::HashSet<[u32; 9]>,
    water: &[(Vec3, Vec3)],
    origin: Vec3,
    coords: (u32, u32),
    subdivisions: u32,
) -> Result<Vec<Vec<u8>>, String> {
    let input = RasterInput::clipped(triangles, obstacles, origin, subdivisions);
    if input.areas.is_empty() {
        return Ok(Vec::new());
    }
    let mut packets = Vec::new();
    for (layer, input) in input.vertical_bands()?.into_iter().enumerate() {
        let build = RecastBuild::allocate()?;
        build.rasterize(&input)?;
        build.polygonize(water)?;
        let packet = build.encode_detour(coords, layer)?;
        if !packet.is_empty() {
            packets.push(packet);
        }
    }
    Ok(packets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor(x0: f32, x1: f32, y: f32) -> [[Vec3; 3]; 2] {
        let a = Vec3::new(x0, y, 10.0);
        let b = Vec3::new(x0, y, 30.0);
        let c = Vec3::new(x1, y, 30.0);
        let d = Vec3::new(x1, y, 10.0);
        [[a, b, c], [a, c, d]]
    }

    #[test]
    fn vertical_bands_preserve_floors_and_connect_across_layer_indices() {
        let triangles: Vec<_> = floor(10.0, 120.0, -3000.0)
            .into_iter()
            .chain(floor(10.0, 160.0, 97.0))
            .collect();
        let bytes = bake_tile(&triangles, (32, 31)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("32_31.nav"), bytes).unwrap();
        let mut map = crate::NavigationMap::open(dir.path()).unwrap();
        for point in [Vec3::new(20.0, -3000.0, 20.0), Vec3::new(20.0, 97.0, 20.0)] {
            let floor = map
                .floor_polygon(point)
                .unwrap()
                .expect("floor must survive");
            assert!((floor.y - point.y).abs() < 0.5, "{floor:?}");
        }
        let route = map
            .complete_path(Vec3::new(130.0, 97.0, 20.0), Vec3::new(136.0, 97.0, 20.0))
            .unwrap();
        assert!(route.last().unwrap().distance(Vec3::new(136.0, 97.0, 20.0)) < 1.0);
    }

    #[test]
    fn tall_wall_blocks_both_vertical_bands() {
        let a = Vec3::new(40.0, -3001.0, 0.0);
        let b = Vec3::new(40.0, 105.0, 0.0);
        let c = Vec3::new(40.0, 105.0, 40.0);
        let d = Vec3::new(40.0, -3001.0, 40.0);
        let triangles: Vec<_> = floor(10.0, 70.0, -3000.0)
            .into_iter()
            .chain(floor(10.0, 70.0, 97.0))
            .chain([[a, b, c], [a, c, d]])
            .collect();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("32_31.nav"),
            bake_tile(&triangles, (32, 31)).unwrap(),
        )
        .unwrap();
        let mut map = crate::NavigationMap::open(dir.path()).unwrap();
        for height in [-3000.0, 97.0] {
            let left = Vec3::new(20.0, height, 20.0);
            let right = Vec3::new(60.0, height, 20.0);
            assert!(map.floor_polygon(left).unwrap().is_some());
            assert!(map.floor_polygon(right).unwrap().is_some());
            assert!(
                map.complete_path(left, right).is_err(),
                "wall must block at {height}"
            );
        }
    }

    #[test]
    #[ignore = "requires FOLLOWER_NAV_DATA local assets"]
    fn arakara_kikatal_floor_survives_vertical_bake() {
        let data = std::path::PathBuf::from(std::env::var_os("FOLLOWER_NAV_DATA").unwrap());
        let mut baker = los::bake::Baker::for_navigation(&data);
        let tiles = baker.map_tiles(5_280_631).unwrap();
        let tile = (29, 32);
        let (geometry, _) = baker.collect_tile(&tiles, tile).unwrap().unwrap();
        assert!(baker.problems.is_empty(), "{:?}", baker.problems);
        let point = Vec3::new(-485.951, 97.26646, -1168.05);
        let grid = 8;
        let size = TILE_SIZE / grid as f32;
        let (x, z) = tile_bounds(tile);
        let col = ((point.x - x) / size).floor() as u32;
        let row = ((point.z - z) / size).floor() as u32;
        let origin = Vec3::new(x + col as f32 * size, 0.0, z + row as f32 * size);
        let obstacles = geometry.obstacles.iter().map(triangle_key).collect();
        let water = baker
            .collect_water(tiles[(tile.1 * 64 + tile.0) as usize])
            .unwrap();
        let packets = bake_subtile(
            &geometry.triangles,
            &obstacles,
            &water,
            origin,
            ((63 - tile.1) * grid + col, tile.0 * grid + row),
            grid,
        )
        .unwrap();
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&(packets.len() as u32).to_le_bytes());
        for packet in packets {
            bytes.extend_from_slice(&(packet.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&packet);
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("29_32.nav"), bytes).unwrap();
        let mut map = crate::NavigationMap::open(dir.path()).unwrap();
        let floor = map
            .floor_polygon(point)
            .unwrap()
            .expect("Ki'katal floor must survive");
        assert!((floor.y - point.y).abs() < 0.5, "{floor:?}");
    }

    #[test]
    fn floor_bakes_deterministically_and_steep_surface_is_excluded() {
        let floor = [
            [
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(10.0, 0.0, 30.0),
                Vec3::new(30.0, 0.0, 30.0),
            ],
            [
                Vec3::new(10.0, 0.0, 10.0),
                Vec3::new(30.0, 0.0, 30.0),
                Vec3::new(30.0, 0.0, 10.0),
            ],
        ];
        let bytes = bake_tile(&floor, (32, 31)).expect("walkable floor must bake");
        assert!(!bytes.is_empty());
        assert_eq!(bytes, bake_tile(&floor, (32, 31)).unwrap());
        let steep = floor.map(|tri| tri.map(|point| Vec3::new(point.x, point.x * 3.0, point.z)));
        assert!(bake_tile(&steep, (32, 31)).unwrap().is_empty());
    }
}
