use crate::*;
use glam::Vec3;
use recastnavigation_sys::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[path = "polygons.rs"]
mod polygons;
pub use polygons::read_walkable_polygons;

#[path = "smooth.rs"]
mod smooth;
#[path = "visibility.rs"]
mod visibility;

struct Corridor {
    start: [f32; 3],
    end: [f32; 3],
    polygons: Vec<dtPolyRef>,
}

const MAX_PATH: usize = 2048;
const MAX_NODES: i32 = 8192;
// TC a352b1fa PathGenerator.cpp:141 nominal Y5; no Y50 retry or shortcuts.
const NEAREST_EXTENTS: [f32; 3] = [0.8, 5.0, 0.8];

/// Owns a mesh and its mutable query scratch space; callers serialize queries with &mut.
/// Artifacts are trusted outputs of nav-bake, not network-supplied Detour buffers.
pub struct NavigationMap {
    dir: PathBuf,
    loaded: HashSet<(u32, u32)>,
    mesh: *mut dtNavMesh,
    query: *mut dtNavMeshQuery,
}

// SAFETY: pointers are exclusively owned, no thread-affine state; use requires &mut.
// No Sync implementation: the server wraps maps in a Mutex.
unsafe impl Send for NavigationMap {}

impl NavigationMap {
    pub fn open(dir: &Path) -> Result<Self, String> {
        // SAFETY: allocators return owned pointers; Drop frees them on init errors.
        let map = unsafe {
            Self {
                dir: dir.to_path_buf(),
                loaded: HashSet::new(),
                mesh: dtAllocNavMesh(),
                query: dtAllocNavMeshQuery(),
            }
        };
        if map.mesh.is_null() || map.query.is_null() {
            return Err("Detour allocation failed".into());
        }
        let params = dtNavMeshParams {
            orig: [-32.0 * TILE_SIZE, 0.0, -32.0 * TILE_SIZE],
            tileWidth: SUBTILE_SIZE,
            tileHeight: SUBTILE_SIZE,
            maxTiles: 65536,
            maxPolys: 65536,
        };
        // SAFETY: pointers are non-null, params lives through init and is copied.
        unsafe {
            status((*map.mesh).init(&params), "mesh initialization")?;
            status(
                (*map.query).init(map.mesh, MAX_NODES),
                "query initialization",
            )?;
        }
        Ok(map)
    }

    /// Native MovePositionToFirstCollision's Detour walkable-surface ray.
    pub fn first_collision(&mut self, start: Vec3, end: Vec3) -> Result<Vec3, String> {
        self.load_around(start, end)?;
        let filter = filter();
        let (reference, start_pos) = self.nearest(start, &filter)?;
        let mut t = 0.0;
        let mut normal = [0.0; 3];
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: initialized query, loaded polygon and bounded output arrays.
        let result = unsafe {
            (*self.query).raycast(
                reference,
                start_pos.as_ptr(),
                end.to_array().as_ptr(),
                &filter,
                &mut t,
                normal.as_mut_ptr(),
                corridor.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "surface raycast")?;
        if result & DT_BUFFER_TOO_SMALL != 0 || count == 0 {
            return Err("incomplete surface raycast".into());
        }
        let start = Vec3::from_array(start_pos);
        let distance = Vec3::new(end.x - start.x, 0.0, end.z - start.z).length();
        let fraction = if t > 1.0 {
            1.0
        } else {
            (t - 0.5 / distance.max(0.5)).max(0.0)
        };
        let endpoint = start.lerp(end, fraction);
        let (_, projected) = self.nearest(endpoint, &filter)?;
        Ok(Vec3::from_array(projected))
    }

    pub fn path(&mut self, start: Vec3, end: Vec3) -> Result<Vec<Vec3>, String> {
        let corridor = self.query_corridor(start, end)?;
        self.pull_corners(&corridor.start, &corridor.end, &corridor.polygons)
    }

    /// TC a352b1fa default PathGenerator surface stepping, only for C++ waypoints.
    pub fn smooth_waypoint_path(&mut self, start: Vec3, end: Vec3) -> Result<Vec<Vec3>, String> {
        let corridor = self.query_corridor(start, end)?;
        self.find_smooth_path(start, end, corridor.polygons)
    }

    fn query_corridor(&mut self, start: Vec3, end: Vec3) -> Result<Corridor, String> {
        if !start.is_finite() || !end.is_finite() {
            return Err("non-finite path endpoint".into());
        }
        self.load_around(start, end)?;
        let filter = filter();
        let (start_ref, start_pos) = self.nearest(start, &filter)?;
        let (end_ref, end_pos) = self.nearest(end, &filter)?;
        let mut corridor = [0; MAX_PATH];
        let mut count = 0;
        // SAFETY: all input/output arrays and refs belong to the loaded mesh.
        let result = unsafe {
            (*self.query).findPath(
                start_ref,
                end_ref,
                start_pos.as_ptr(),
                end_pos.as_ptr(),
                &filter,
                corridor.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
            )
        };
        status(result, "path search")?;
        if result & (DT_PARTIAL_RESULT | DT_BUFFER_TOO_SMALL | DT_OUT_OF_NODES) != 0
            || count == 0
            || corridor[count as usize - 1] != end_ref
        {
            return Err("unreachable target (incomplete corridor)".into());
        }
        self.optimize_visible_corridor(
            Corridor {
                start: start_pos,
                end: end_pos,
                polygons: corridor[..count as usize].to_vec(),
            },
            &filter,
        )
    }

    fn nearest(
        &self,
        point: Vec3,
        filter: &dtQueryFilter,
    ) -> Result<(dtPolyRef, [f32; 3]), String> {
        let mut reference = 0;
        let mut nearest = [0.0; 3];
        // SAFETY: fixed-size inputs/outputs, query initialized against owned mesh.
        unsafe {
            status(
                (*self.query).findNearestPoly(
                    point.to_array().as_ptr(),
                    NEAREST_EXTENTS.as_ptr(),
                    filter,
                    &mut reference,
                    nearest.as_mut_ptr(),
                ),
                "nearest polygon",
            )?;
        }
        if reference == 0 {
            return Err(format!("no walkable polygon near {point:?}"));
        }
        Ok((reference, nearest))
    }

    fn pull_corners(
        &self,
        start: &[f32; 3],
        end: &[f32; 3],
        corridor: &[dtPolyRef],
    ) -> Result<Vec<Vec3>, String> {
        let mut corners = [0.0; MAX_PATH * 3];
        let mut references = [0; MAX_PATH];
        let mut count = 0;
        // Keep height/area transition portals as well as obstacle corners: one giant
        // sloped chord must not cut through a terrain ridge or an indoor staircase.
        // SAFETY: refs validated by findPath; outputs sized for the supplied maximum.
        let result = unsafe {
            (*self.query).findStraightPath(
                start.as_ptr(),
                end.as_ptr(),
                corridor.as_ptr(),
                corridor.len() as i32,
                corners.as_mut_ptr(),
                std::ptr::null_mut(),
                references.as_mut_ptr(),
                &mut count,
                MAX_PATH as i32,
                dtStraightPathOptions_DT_STRAIGHTPATH_ALL_CROSSINGS as i32,
            )
        };
        status(result, "string pulling")?;
        if result & DT_BUFFER_TOO_SMALL != 0 {
            return Err("corner buffer exhausted".into());
        }
        self.project_corner_surfaces(
            &corners[..count as usize * 3],
            &references[..count as usize],
            corridor[corridor.len() - 1],
        )
    }

    /// Project interpolated portals onto their polygon's approximate detail surface.
    /// Actual chase/home feet are corrected by the server's exact ground query.
    fn project_corner_surfaces(
        &self,
        corners: &[f32],
        references: &[dtPolyRef],
        end_ref: dtPolyRef,
    ) -> Result<Vec<Vec3>, String> {
        corners
            .chunks_exact(3)
            .zip(references)
            .map(|(point, &reference)| {
                // Detour marks the terminal point with ref0, belonging to the final poly.
                let reference = if reference == 0 { end_ref } else { reference };
                let mut surface = [0.0; 3];
                // A portal may lie just outside its polygon after float interpolation.
                // Project onto its detail surface, without a strict inside-only query.
                // SAFETY: corridor ref is valid; both arrays have three floats.
                unsafe {
                    status(
                        (*self.query).closestPointOnPoly(
                            reference,
                            point.as_ptr(),
                            surface.as_mut_ptr(),
                            std::ptr::null_mut(),
                        ),
                        "detail corner projection",
                    )?;
                }
                Ok(Vec3::from_array(surface))
            })
            .collect()
    }

    fn load_around(&mut self, start: Vec3, end: Vec3) -> Result<(), String> {
        let a = tile_of(start);
        let b = tile_of(end);
        // Endpoints on a baked map must both have artifacts. Neighbor absence may be
        // authored world bounds, but cannot become a direct-movement substitution.
        self.load_tile(a)?;
        self.load_tile(b)?;
        for row in a.0.min(b.0).saturating_sub(1)..=(a.0.max(b.0) + 1).min(63) {
            for col in a.1.min(b.1).saturating_sub(1)..=(a.1.max(b.1) + 1).min(63) {
                if self.dir.join(format!("{row}_{col}.nav")).is_file() {
                    self.load_tile((row, col))?;
                }
            }
        }
        Ok(())
    }

    fn load_tile(&mut self, tile: (u32, u32)) -> Result<(), String> {
        if self.loaded.contains(&tile) {
            return Ok(());
        }
        let path = self.dir.join(format!("{}_{}.nav", tile.0, tile.1));
        let bytes = std::fs::read(&path).map_err(|err| format!("{}: {err}", path.display()))?;
        if !bytes.is_empty() {
            for packet in decode_packets(&bytes)? {
                self.add_packet(packet)?;
            }
        }
        self.loaded.insert(tile);
        Ok(())
    }

    fn add_packet(&mut self, bytes: &[u8]) -> Result<(), String> {
        validate_packet(bytes)?;
        // SAFETY: validated complete trusted bake buffer; owned allocator memory is
        // transferred to mesh on success and freed here on failure.
        unsafe {
            let data = dtAlloc(bytes.len(), dtAllocHint_DT_ALLOC_PERM).cast::<u8>();
            if data.is_null() {
                return Err("Detour tile allocation failed".into());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
            let result = (*self.mesh).addTile(
                data,
                bytes.len() as i32,
                dtTileFlags_DT_TILE_FREE_DATA as i32,
                0,
                std::ptr::null_mut(),
            );
            if result & DT_FAILURE != 0 {
                dtFree(data.cast());
            }
            status(result, "loading tile")
        }
    }
}

impl Drop for NavigationMap {
    fn drop(&mut self) {
        // SAFETY: unique ownership; frees accept null and mesh owns successful tile data.
        unsafe {
            dtFreeNavMeshQuery(self.query);
            dtFreeNavMesh(self.mesh);
        }
    }
}

fn filter() -> dtQueryFilter {
    let mut costs = [1.0; 64];
    costs[WATER_AREA as usize] = 2.0;
    dtQueryFilter {
        m_areaCost: costs,
        m_includeFlags: WALK_FLAG,
        m_excludeFlags: 0,
    }
}

fn status(value: dtStatus, operation: &str) -> Result<(), String> {
    if value & DT_FAILURE != 0 {
        Err(format!("Detour {operation} failed ({value:#x})"))
    } else {
        Ok(())
    }
}

fn decode_packets(bytes: &[u8]) -> Result<Vec<&[u8]>, String> {
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err("invalid navigation artifact header".into());
    }
    let count = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if count > SUBDIVISIONS * SUBDIVISIONS {
        return Err("too many subtiles".into());
    }
    let mut remaining = &bytes[12..];
    let mut packets = Vec::new();
    for _ in 0..count {
        let length = remaining.get(..4).ok_or("truncated packet length")?;
        let size = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        remaining = &remaining[4..];
        packets.push(remaining.get(..size).ok_or("truncated Detour tile")?);
        remaining = &remaining[size..];
    }
    if !remaining.is_empty() {
        return Err("trailing navigation bytes".into());
    }
    Ok(packets)
}

fn validate_packet(bytes: &[u8]) -> Result<(), String> {
    use std::mem::size_of;
    if bytes.len() < size_of::<dtMeshHeader>() {
        return Err("truncated Detour header".into());
    }
    // SAFETY: enough bytes for this POD header; file bytes may be unaligned.
    let header = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<dtMeshHeader>()) };
    if header.magic != DT_NAVMESH_MAGIC || header.version != DT_NAVMESH_VERSION {
        return Err("incompatible Detour tile".into());
    }
    let arrays = [
        (1, size_of::<dtMeshHeader>()),
        (header.vertCount, 12),
        (header.polyCount, size_of::<dtPoly>()),
        (header.maxLinkCount, size_of::<dtLink>()),
        (header.detailMeshCount, size_of::<dtPolyDetail>()),
        (header.detailVertCount, 12),
        (header.detailTriCount, 4),
        (header.bvNodeCount, size_of::<dtBVNode>()),
        (header.offMeshConCount, size_of::<dtOffMeshConnection>()),
    ];
    let mut expected = 0usize;
    for (count, stride) in arrays {
        let count = usize::try_from(count).map_err(|_| "negative Detour array length")?;
        let size = count.checked_mul(stride).ok_or("oversized Detour array")?;
        expected = expected
            .checked_add((size + 3) & !3)
            .ok_or("oversized Detour tile")?;
    }
    if expected != bytes.len() {
        return Err("Detour tile array lengths do not match packet".into());
    }
    Ok(())
}

#[cfg(all(test, feature = "bake"))]
#[path = "seam_tests.rs"]
mod seam_tests;

#[cfg(all(test, feature = "bake"))]
mod tests {
    use super::*;
    fn ground_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground")
    }
    fn wow(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, z, -y)
    }

    #[test]
    #[ignore = "requires FOLLOWER_RUBY_DATA and FOLLOWER_RUBY_GROUND local authored assets"]
    fn ruby_metalplate_doorway_routes_over_authored_collision() {
        let data = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_DATA").unwrap());
        let ground = PathBuf::from(std::env::var_os("FOLLOWER_RUBY_GROUND").unwrap());
        let db = rusqlite::Connection::open_with_flags(
            std::env::var("FOLLOWER_RUBY_WORLD_DB").unwrap(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let wdt: u32 = db
            .query_row("SELECT WdtFileDataID FROM map WHERE ID=2521", [], |row| {
                row.get(0)
            })
            .unwrap();
        let mut baker = los::bake::Baker::for_navigation(&data);
        let tiles = baker.map_tiles(wdt).unwrap();
        let (geometry, _) = baker.collect_tile(&tiles, (32, 29)).unwrap().unwrap();
        assert!(
            baker.problems.is_empty(),
            "authored collision gaps: {:?}",
            baker.problems
        );
        let baked = crate::bake::bake_collected(&geometry, &[], (32, 29)).unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("32_29.nav"), baked).unwrap();
        let mut nav = NavigationMap::open(dir.path()).unwrap();
        // Opposite actual component edges at WMO4391189/MODD753 metalplate4496145.
        let start = Vec3::new(1591.1461, 127.144, 200.2604);
        let end = Vec3::new(1590.1045, 127.144, 202.60416);
        let path = nav
            .path(start, end)
            .expect("authored metalplate doorway must remain traversable");
        let mut los = los::LosMap::new(&ground.join("los/2521"));
        for segment in path.windows(2) {
            assert!(
                los.clear(segment[0] + Vec3::Y * 2.0, segment[1] + Vec3::Y * 2.0)
                    .unwrap(),
                "doorway route crosses collision: {segment:?}"
            );
        }
        eprintln!("Ruby metalplate doorway route: {path:?}");
        let approach = nav
            .path(
                wow(1386.2, -169.719, 131.0),
                wow(1551.33, -257.688, 126.01431),
            )
            .expect("actual entrance must reach the collected boss floor");
        for segment in approach.windows(2) {
            assert!(
                los.clear(segment[0] + Vec3::Y * 2.0, segment[1] + Vec3::Y * 2.0)
                    .unwrap(),
                "Ruby approach crosses collision: {segment:?}"
            );
        }
        eprintln!(
            "Ruby entrance to collected boss floor: {} corners",
            approach.len()
        );
        nav.path(
            wow(1386.2, -169.719, 131.0),
            wow(1551.33, -257.688, 129.782),
        )
        .expect("TC nominal vertical search must resolve the unchanged authored boss height");
        assert!(
            nav.path(wow(1386.2, -169.719, 131.0), wow(1551.33, -257.688, 133.0))
                .is_err(),
            "a boss-column point beyond the nominal vertical extent must not use the TC larger-search fallback"
        );
    }

    #[test]
    #[ignore = "requires nav-bake real map0 ocean tile23,39"]
    fn modern_ocean_artifact_marks_water_area() {
        let bytes = std::fs::read(ground_dir().join("navmesh/0/23_39.nav")).unwrap();
        let mut water = 0;
        for packet in decode_packets(&bytes).unwrap() {
            validate_packet(packet).unwrap();
            // The serialized .nav area IDs are the external bake contract.
            let header =
                unsafe { std::ptr::read_unaligned(packet.as_ptr().cast::<dtMeshHeader>()) };
            let align = |size: usize| (size + 3) & !3;
            let first =
                align(std::mem::size_of::<dtMeshHeader>()) + align(header.vertCount as usize * 12);
            for index in 0..header.polyCount as usize {
                let offset = first + index * std::mem::size_of::<dtPoly>();
                let poly =
                    unsafe { std::ptr::read_unaligned(packet[offset..].as_ptr().cast::<dtPoly>()) };
                water += usize::from(poly.areaAndtype & 63 == WATER_AREA);
            }
        }
        assert!(
            water > 100,
            "real submerged terrain must retain authored water area, found {water}"
        );
    }

    #[test]
    #[ignore = "requires nav-bake map34 from local Stockade assets"]
    fn stockade_repath_to_other_cell_has_clear_segments() {
        let mut map = NavigationMap::open(&ground_dir().join("navmesh/34")).unwrap();
        let path = map
            .path(wow(128.5, 52.0, -33.86), wow(103.0, 76.0, -34.86))
            .unwrap();
        let mut los = los::LosMap::new(&ground_dir().join("los/34"));
        for pair in path.windows(2) {
            assert!(
                los.clear(pair[0] + Vec3::Y, pair[1] + Vec3::Y).unwrap(),
                "blocked path segment {pair:?}; full {path:?}"
            );
        }
    }

    #[test]
    #[ignore = "requires nav-bake map34 from local Stockade assets"]
    fn stockade_path_routes_around_real_walls() {
        let mut map = NavigationMap::open(&ground_dir().join("navmesh/34")).unwrap();
        let a = wow(115.646, 46.6108, -34.8562);
        let b = wow(128.5, 52.0, -33.86);
        let path = map.path(a, b).expect("Stockade floor route");
        assert!(path.len() > 2, "wall requires corners: {path:?}");
        let mut los = los::LosMap::new(&ground_dir().join("los/34"));
        assert!(
            !los.clear(a + Vec3::Y * 2.0, b + Vec3::Y * 1.0).unwrap(),
            "direct line crosses real wall"
        );
        for segment in path.windows(2) {
            assert!(
                los.clear(segment[0] + Vec3::Y, segment[1] + Vec3::Y)
                    .unwrap(),
                "wall crossed: {segment:?}"
            );
        }
    }
}
