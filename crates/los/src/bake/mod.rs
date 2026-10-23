//! Offline bake of a map's tiles from the local CASC extraction (`game-engine/data`:
//! `terrain/<fdid>.wdt|adt`, `models/<fdid>.wmo|m2|skin|skel|anim`). Never reads the CDN.
//!
//! A tile keeps its terrain and the opaque triangles of the WMOs and M2 doodads placed by
//! its own and its neighbours' `_obj0`, each triangle stored in the tile holding its
//! centroid (so a WMO spanning tiles is stored once). Dropped: alpha-tested and blended
//! batches, and doodads whose opaque geometry is thinner than the cutoff on any axis of
//! its placed bounding box (thin poles, fences and low ground clutter alike).

pub mod adt;
pub mod chunks;
pub mod eye;
pub mod m2;
pub mod wmo;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use glam::{Affine3A, Vec3, Vec3Swizzles};

use crate::{LosTile, MAP_TILES, TILE_SIZE, tile_of};
use adt::{ModelRef, Placement, TileFiles};
use chunks::Result;

/// Default doodad thickness cutoff in yards (spec: "start near 1 yd").
pub const DEFAULT_THICKNESS_CUTOFF: f32 = 1.0;

/// Collision geometry retains which triangles cannot support runtime ground.
#[derive(Default)]
pub struct CollectedGeometry {
    pub triangles: Vec<[Vec3; 3]>,
    /// M2 faces that cannot support grounded movement, classified by authored normals.
    pub obstacles: Vec<[Vec3; 3]>,
}
pub type GlobalGeometry = Vec<((u32, u32), CollectedGeometry)>;

/// A model's opaque triangles in model-local Bevy space and their bounds.
pub struct ModelGeometry {
    pub triangles: Vec<[Vec3; 3]>,
    pub min: Vec3,
    pub max: Vec3,
    /// Triangles of alpha-tested or blended batches.
    pub dropped_triangles: usize,
    /// Authored M2 face normals in model-local engine axes, used by collision bakes.
    /// LOS render-face selection remains independent.
    floor_normals: Vec<Vec3>,
}

impl ModelGeometry {
    fn new(triangles: Vec<[Vec3; 3]>, dropped_triangles: usize) -> Self {
        let (min, max) = triangles.iter().flatten().fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(min, max), &point| (min.min(point), max.max(point)),
        );
        Self {
            triangles,
            min,
            max,
            dropped_triangles,
            floor_normals: Vec::new(),
        }
    }

    fn select_m2_faces(&self, world: Affine3A, walkable: bool) -> Vec<[Vec3; 3]> {
        self.triangles
            .iter()
            .zip(&self.floor_normals)
            .filter_map(|(&triangle, &normal)| {
                (m2_face_is_walkable(normal, world) == walkable).then_some(triangle)
            })
            .collect()
    }

    /// Smallest side of the bounding box at `scale`.
    pub fn thickness(&self, scale: f32) -> f32 {
        ((self.max - self.min) * scale).min_element()
    }
}

fn m2_face_is_walkable(normal: Vec3, world: Affine3A) -> bool {
    let world_normal = world.matrix3.mul_vec3(normal.into()).normalize_or_zero();
    world_normal.y >= shared::ground::min_walkable_normal_y()
}

struct WmoModel {
    root: wmo::WmoRoot,
    geometry: ModelGeometry,
}

/// A doodad model dropped as thinner than the cutoff.
#[derive(Clone, Debug)]
pub struct DroppedDoodad {
    pub fdid: u32,
    pub name: String,
    /// Smallest placed bounding-box side over its placements, in yards.
    pub thickness: f32,
    pub placements: usize,
}

#[derive(Default, Debug)]
pub struct TileStats {
    pub terrain_triangles: usize,
    pub wmo_triangles: usize,
    pub doodad_triangles: usize,
    pub alpha_or_blended_triangles: usize,
    pub thin_doodads: usize,
}

pub struct Baker {
    data: PathBuf,
    listfile: Option<rusqlite::Connection>,
    pub cutoff: f32,
    collision: bool,
    m2_only: bool,
    m2s: HashMap<u32, Option<Rc<ModelGeometry>>>,
    wmos: HashMap<u32, Option<Rc<WmoModel>>>,
    /// Dropped thin doodads by model, counted once per (placement unique id, model): the
    /// doodads of one placed WMO count once.
    dropped: HashMap<u32, DroppedDoodad>,
    dropped_placements: HashSet<(u32, u32)>,
    /// Model files the extraction lacks or that fail to parse, with why.
    pub problems: Vec<String>,
}

impl Baker {
    /// `data` is game-engine's `data/` (the local CASC extraction).
    pub fn new(data: &Path, cutoff: f32) -> Self {
        let listfile = rusqlite::Connection::open_with_flags(
            data.join("community-listfile.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .ok();
        Self {
            data: data.to_path_buf(),
            listfile,
            cutoff,
            collision: false,
            m2_only: false,
            m2s: HashMap::new(),
            wmos: HashMap::new(),
            dropped: HashMap::new(),
            dropped_placements: HashSet::new(),
            problems: Vec::new(),
        }
    }

    /// The same placement/asset collector, selecting ground collision rather than LOS render faces.
    pub fn for_navigation(data: &Path) -> Self {
        Self {
            collision: true,
            ..Self::new(data, 0.0)
        }
    }

    /// Authored M2 collision hulls only, without LOS thickness/material filtering.
    /// WMO doodad sets use the same composed transforms as navigation and rendering.
    pub fn for_m2_floors(data: &Path) -> Self {
        Self {
            collision: true,
            m2_only: true,
            ..Self::new(data, 0.0)
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    fn read(&self, dir: &str, name: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.data.join(dir).join(name))
    }

    /// The map's tiles from its WDT (`terrain/<wdt>.wdt`), indexed `col * 64 + row`.
    pub fn map_tiles(&self, wdt_fdid: u32) -> Result<Vec<TileFiles>> {
        let wdt = self
            .read("terrain", &format!("{wdt_fdid}.wdt"))
            .map_err(|err| format!("terrain/{wdt_fdid}.wdt: {err}"))?;
        adt::wdt_tiles(&wdt)
    }

    /// A global WMO uses the ground system's origin-relative placement, split into the
    /// same tiles as continents. Both LOS and navigation consume this collector.
    pub fn collect_global(&mut self, wdt_fdid: u32) -> Result<Option<GlobalGeometry>> {
        let wdt = self
            .read("terrain", &format!("{wdt_fdid}.wdt"))
            .map_err(|err| err.to_string())?;
        let Some(mut placement) = adt::global_wmo(&wdt)? else {
            return Ok(None);
        };
        let fdid = match &placement.model {
            ModelRef::Fdid(fdid) => *fdid,
            ModelRef::Path(path) => self
                .path_fdid(path)
                .ok_or_else(|| format!("global WMO path {path}: not in local listfile"))?,
        };
        placement.model = ModelRef::Fdid(fdid);
        let model = self
            .wmo(fdid)
            .ok_or_else(|| format!("missing global WMO {fdid}"))?;
        let position = shared::ground::global_wmo_placement_position(placement.position);
        let world = Affine3A::from_scale_rotation_translation(
            Vec3::splat(placement.scale),
            adt::placement_rotation(placement.rotation),
            position,
        );
        let triangles = if self.m2_only {
            self.load_global_m2_triangles(&model.root, &placement, world)
        } else {
            transformed(&model.geometry.triangles, world).collect()
        };
        let mut tiles = std::collections::BTreeSet::new();
        for tri in triangles {
            let min = tri[0].min(tri[1]).min(tri[2]);
            let max = tri[0].max(tri[1]).max(tri[2]);
            let (r0, c1) = tile_of(min.x, min.z);
            let (r1, c0) = tile_of(max.x, max.z);
            for row in r0..=r1 {
                for col in c0..=c1 {
                    tiles.insert((row, col));
                }
            }
        }
        let mut geometry = Vec::new();
        for tile in tiles {
            let mut build = TileBuild {
                tile,
                border: if self.collision { 2.0 } else { 0.0 },
                triangles: Vec::new(),
                obstacles: Vec::new(),
                stats: TileStats::default(),
            };
            self.place_wmo_at(&placement, world, &mut build);
            geometry.push((
                tile,
                CollectedGeometry {
                    triangles: build.triangles,
                    obstacles: build.obstacles,
                },
            ));
        }
        Ok(Some(geometry))
    }

    fn load_global_m2_triangles(
        &mut self,
        root: &wmo::WmoRoot,
        placement: &Placement,
        world: Affine3A,
    ) -> Vec<[Vec3; 3]> {
        let mut triangles = Vec::new();
        for doodad in placed_wmo_doodads(root, placement, world) {
            if let Some(model) = self.m2(doodad.fdid) {
                triangles.extend(transformed(&model.triangles, doodad.world));
            }
        }
        triangles
    }

    /// Authored terrain-water quads, using the same MH2O parser as runtime ground.
    /// Bounds mark submerged walkable spans; water is not substituted for collision.
    pub fn collect_water(&self, files: TileFiles) -> Result<Vec<(Vec3, Vec3)>> {
        if files.root == 0 {
            return Ok(Vec::new());
        }
        let root = self
            .read("terrain", &format!("{}.adt", files.root))
            .map_err(|err| err.to_string())?;
        let Some(payload) = chunks::find_reversed(&root, b"MH2O") else {
            return Ok(Vec::new());
        };
        let liquids = crate::liquid::parse_mh2o(payload)?;
        let mut bounds = Vec::new();
        for (tag, chunk) in chunks::chunks(&root) {
            if &tag != b"KNCM" {
                continue;
            }
            let x_index = chunks::u32_at(chunk, 4)? as usize;
            let y_index = chunks::u32_at(chunk, 8)? as usize;
            let Some(liquid) = liquids.get(y_index * 16 + x_index) else {
                continue;
            };
            let x = chunks::f32_at(chunk, 0x68)?;
            let z = -chunks::f32_at(chunk, 0x6c)?;
            for layer in &liquid.layers {
                bounds.extend(water_quad_bounds(layer, x, z));
            }
        }
        Ok(bounds)
    }

    /// Thin doodads dropped so far, thinnest first.
    pub fn dropped_doodads(&self) -> Vec<DroppedDoodad> {
        let mut dropped: Vec<_> = self.dropped.values().cloned().collect();
        dropped.sort_by(|a, b| {
            a.thickness
                .total_cmp(&b.thickness)
                .then(a.fdid.cmp(&b.fdid))
        });
        dropped
    }

    pub fn model_name(&self, fdid: u32) -> String {
        self.listfile
            .as_ref()
            .and_then(|db| {
                db.query_row(
                    "SELECT path FROM listfile_entries WHERE fdid = ?1",
                    [fdid],
                    |row| row.get(0),
                )
                .ok()
            })
            .unwrap_or_else(|| format!("fdid {fdid}"))
    }

    /// Look up an authored model path in the local listfile database.
    pub fn path_fdid(&self, path: &str) -> Option<u32> {
        let path = path.to_lowercase();
        // MMDX names `.mdx`/`.mdl`; the listfile holds the `.m2`.
        let path = match path.rsplit_once('.') {
            Some((stem, "mdx" | "mdl")) => format!("{stem}.m2"),
            _ => path,
        };
        self.listfile
            .as_ref()?
            .query_row(
                "SELECT fdid FROM listfile_entries WHERE lower_path = ?1",
                [path],
                |row| row.get(0),
            )
            .ok()
    }

    /// Bakes tile `(row, col)`; `None` when the WDT has no root ADT there.
    pub fn bake_tile(
        &mut self,
        tiles: &[TileFiles],
        (row, col): (u32, u32),
    ) -> Result<Option<(LosTile, TileStats)>> {
        let Some((triangles, stats)) = self.collect_tile(tiles, (row, col))? else {
            return Ok(None);
        };
        let (vertices, indexed) = index_triangles(&triangles.triangles);
        Ok(Some((LosTile::build(row, col, vertices, indexed), stats)))
    }

    /// World triangles and counts, before LOS BVH or navigation voxelization.
    pub fn collect_tile(
        &mut self,
        tiles: &[TileFiles],
        (row, col): (u32, u32),
    ) -> Result<Option<(CollectedGeometry, TileStats)>> {
        let files = tiles[(col * MAP_TILES + row) as usize];
        if files.root == 0 {
            return Ok(None);
        }
        let root = self
            .read("terrain", &format!("{}.adt", files.root))
            .map_err(|err| format!("terrain/{}.adt: {err}", files.root))?;
        let terrain = if self.m2_only {
            Vec::new()
        } else {
            adt::terrain_triangles(&root)?
        };
        let mut build = TileBuild {
            tile: (row, col),
            border: if self.collision { 2.0 } else { 0.0 },
            stats: TileStats {
                terrain_triangles: terrain.len(),
                ..TileStats::default()
            },
            triangles: terrain,
            obstacles: Vec::new(),
        };
        if self.collision && !self.m2_only {
            self.collect_neighbour_terrain(tiles, &mut build)?;
        }
        self.collect_placed_geometry(tiles, &mut build)?;
        Ok(Some(build.into_geometry()))
    }

    fn collect_neighbour_terrain(&self, tiles: &[TileFiles], build: &mut TileBuild) -> Result<()> {
        let (row, col) = build.tile;
        for r in row.saturating_sub(1)..=(row + 1).min(MAP_TILES - 1) {
            for c in col.saturating_sub(1)..=(col + 1).min(MAP_TILES - 1) {
                if (r, c) == (row, col) {
                    continue;
                }
                let root = tiles[(c * MAP_TILES + r) as usize].root;
                if root == 0 {
                    continue;
                }
                let data = self
                    .read("terrain", &format!("{root}.adt"))
                    .map_err(|err| format!("terrain/{root}.adt: {err}"))?;
                build.add(&adt::terrain_triangles(&data)?, Affine3A::IDENTITY);
            }
        }
        Ok(())
    }

    fn collect_placed_geometry(
        &mut self,
        tiles: &[TileFiles],
        build: &mut TileBuild,
    ) -> Result<()> {
        let (doodads, wmos) = self.neighbourhood_placements(tiles, build.tile)?;
        for placement in &wmos {
            self.place_wmo(placement, build);
        }
        for placement in &doodads {
            let ModelRef::Fdid(fdid) = self.resolve(&placement.model) else {
                continue;
            };
            let doodad = PlacedDoodad {
                fdid,
                unique_id: placement.unique_id,
                scale: placement.scale,
                world: placement.world_from_local(),
            };
            self.place_doodad(&doodad, build);
        }
        Ok(())
    }

    /// MDDF and MODF placements of the tile's and its neighbours' `_obj0`, each unique id
    /// once.
    fn neighbourhood_placements(
        &mut self,
        tiles: &[TileFiles],
        (row, col): (u32, u32),
    ) -> Result<(Vec<Placement>, Vec<Placement>)> {
        let (mut doodads, mut wmos) = (Vec::new(), Vec::new());
        let (mut seen_doodads, mut seen_wmos) = (HashSet::new(), HashSet::new());
        for r in row.saturating_sub(1)..=(row + 1).min(MAP_TILES - 1) {
            for c in col.saturating_sub(1)..=(col + 1).min(MAP_TILES - 1) {
                let obj0 = tiles[(c * MAP_TILES + r) as usize].obj0;
                if obj0 == 0 {
                    continue;
                }
                let data = match self.read("terrain", &format!("{obj0}.adt")) {
                    Ok(data) => data,
                    Err(err) => {
                        self.problems
                            .push(format!("terrain/{obj0}.adt (tile {r},{c} _obj0): {err}"));
                        continue;
                    }
                };
                let (tile_doodads, tile_wmos) =
                    adt::placements(&data).map_err(|err| format!("terrain/{obj0}.adt: {err}"))?;
                doodads.extend(
                    tile_doodads
                        .into_iter()
                        .filter(|p| seen_doodads.insert(p.unique_id)),
                );
                wmos.extend(
                    tile_wmos
                        .into_iter()
                        .filter(|p| seen_wmos.insert(p.unique_id)),
                );
            }
        }
        Ok((doodads, wmos))
    }

    fn resolve(&mut self, model: &ModelRef) -> ModelRef {
        match model {
            ModelRef::Fdid(fdid) => ModelRef::Fdid(*fdid),
            ModelRef::Path(path) => match self.path_fdid(path) {
                Some(fdid) => ModelRef::Fdid(fdid),
                None => {
                    self.problems
                        .push(format!("MDDF model {path}: not in the listfile"));
                    model.clone()
                }
            },
        }
    }

    fn place_wmo(&mut self, placement: &Placement, build: &mut TileBuild) {
        self.place_wmo_at(placement, placement.world_from_local(), build);
    }

    fn place_wmo_at(&mut self, placement: &Placement, world: Affine3A, build: &mut TileBuild) {
        let ModelRef::Fdid(fdid) = placement.model else {
            return;
        };
        let Some(model) = self.wmo(fdid) else { return };
        if !self.m2_only && overlaps_tile(&model.geometry, world, build.tile, build.border) {
            build.stats.wmo_triangles += build.add(&model.geometry.triangles, world);
            build.stats.alpha_or_blended_triangles += model.geometry.dropped_triangles;
        }
        for doodad in placed_wmo_doodads(&model.root, placement, world) {
            self.place_doodad(&doodad, build);
        }
    }

    fn place_doodad(&mut self, doodad: &PlacedDoodad, build: &mut TileBuild) {
        let Some(model) = self.m2(doodad.fdid) else {
            return;
        };
        if model.triangles.is_empty()
            || !overlaps_tile(&model, doodad.world, build.tile, build.border)
        {
            return;
        }
        let thickness = model.thickness(doodad.scale);
        if thickness >= self.cutoff {
            build.stats.alpha_or_blended_triangles += model.dropped_triangles;
            let added = if self.m2_only {
                build.add(&model.select_m2_faces(doodad.world, true), doodad.world)
            } else {
                build.add(&model.triangles, doodad.world)
            };
            build.stats.doodad_triangles += added;
            if self.collision && !self.m2_only {
                let obstacles = model.select_m2_faces(doodad.world, false);
                build
                    .obstacles
                    .extend(transformed(&obstacles, doodad.world));
            }
            return;
        }
        build.stats.thin_doodads += 1;
        if self
            .dropped_placements
            .insert((doodad.unique_id, doodad.fdid))
        {
            let name = self.model_name(doodad.fdid);
            let entry = self.dropped.entry(doodad.fdid).or_insert(DroppedDoodad {
                fdid: doodad.fdid,
                name,
                thickness,
                placements: 0,
            });
            entry.thickness = entry.thickness.min(thickness);
            entry.placements += 1;
        }
    }

    /// Doodad model `fdid`'s opaque geometry; `None` when it is missing or unreadable.
    pub fn m2(&mut self, fdid: u32) -> Option<Rc<ModelGeometry>> {
        if let Some(cached) = self.m2s.get(&fdid) {
            return cached.clone();
        }
        let loaded = self
            .load_m2(fdid)
            .map_err(|err| self.problems.push(format!("models/{fdid}.m2: {err}")))
            .ok()
            .map(Rc::new);
        self.m2s.insert(fdid, loaded.clone());
        loaded
    }

    fn load_m2(&self, fdid: u32) -> Result<ModelGeometry> {
        let data = self
            .read("models", &format!("{fdid}.m2"))
            .map_err(|err| err.to_string())?;
        let file = m2::M2File::parse(&data)?;
        if self.collision {
            let mut geometry = ModelGeometry::new(file.collision_triangles()?, 0);
            geometry.floor_normals = file.collision_normals()?;
            return Ok(geometry);
        }
        let skin = primary_skin(&self.data.join("models"), fdid, file.skin_fdid)?;
        let (triangles, dropped) = file.opaque_triangles(&skin)?;
        let triangles = triangles
            .into_iter()
            .map(|tri| tri.map(adt::file_to_bevy))
            .collect();
        Ok(ModelGeometry::new(triangles, dropped))
    }

    fn wmo(&mut self, fdid: u32) -> Option<Rc<WmoModel>> {
        if let Some(cached) = self.wmos.get(&fdid) {
            return cached.clone();
        }
        let loaded = self
            .load_wmo(fdid)
            .map_err(|err| self.problems.push(format!("models/{fdid}.wmo: {err}")))
            .ok()
            .map(Rc::new);
        self.wmos.insert(fdid, loaded.clone());
        loaded
    }

    fn load_wmo(&mut self, fdid: u32) -> Result<WmoModel> {
        let data = self
            .read("models", &format!("{fdid}.wmo"))
            .map_err(|err| err.to_string())?;
        let root = wmo::WmoRoot::parse(&data)?;
        let mut triangles = Vec::new();
        let mut dropped = 0;
        for &group in &root.group_fdids {
            let group_data = match self.read("models", &format!("{group}.wmo")) {
                Ok(group_data) => group_data,
                Err(err) => {
                    self.problems
                        .push(format!("models/{group}.wmo (group of {fdid}): {err}"));
                    continue;
                }
            };
            let (group_triangles, group_dropped) = if self.collision {
                let collision = shared::ground::WmoGroupCollision::parse(&group_data)?;
                (collision.collidable_triangles().collect(), 0)
            } else {
                root.group_triangles(&group_data)
                    .map_err(|err| format!("group {group}: {err}"))?
            };
            triangles.extend(group_triangles);
            dropped += group_dropped;
        }
        Ok(WmoModel {
            root,
            geometry: ModelGeometry::new(triangles, dropped),
        })
    }
}

/// A doodad model placed in the world.
struct PlacedDoodad {
    fdid: u32,
    /// The MDDF placement's, or the placed WMO's for a WMO doodad.
    unique_id: u32,
    scale: f32,
    world: Affine3A,
}

/// WMO file space -> local engine axes, composed with its parent placement.
fn placed_wmo_doodads<'a>(
    root: &'a wmo::WmoRoot,
    placement: &'a Placement,
    world: Affine3A,
) -> impl Iterator<Item = PlacedDoodad> + 'a {
    let file_to_bevy =
        Affine3A::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]);
    root.doodads(placement.doodad_set).map(move |doodad| {
        let file_local = Affine3A::from_scale_rotation_translation(
            Vec3::splat(doodad.scale),
            glam::Quat::from_array(doodad.rotation).normalize(),
            Vec3::from(doodad.position),
        );
        let local = file_to_bevy * file_local * file_to_bevy.inverse();
        PlacedDoodad {
            fdid: doodad.fdid,
            // A WMO doodad's report key uses its parent placement's unique id.
            unique_id: placement.unique_id,
            scale: doodad.scale * placement.scale,
            world: world * local,
        }
    })
}

/// The triangles one tile keeps and its counts.
struct TileBuild {
    tile: (u32, u32),
    border: f32,
    triangles: Vec<[Vec3; 3]>,
    obstacles: Vec<[Vec3; 3]>,
    stats: TileStats,
}

impl TileBuild {
    fn into_geometry(self) -> (CollectedGeometry, TileStats) {
        (
            CollectedGeometry {
                triangles: self.triangles,
                obstacles: self.obstacles,
            },
            self.stats,
        )
    }

    /// Adds the placed triangles whose centroid lies in the tile; returns how many.
    fn add(&mut self, triangles: &[[Vec3; 3]], world: Affine3A) -> usize {
        let before = self.triangles.len();
        let tile = self.tile;
        let border = self.border;
        self.triangles
            .extend(transformed(triangles, world).filter(|tri| {
                if border == 0.0 {
                    let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;
                    return tile_of(centroid.x, centroid.z) == tile;
                }
                let center = 32.0 * TILE_SIZE;
                let x0 = center - (tile.1 + 1) as f32 * TILE_SIZE - border;
                let x1 = center - tile.1 as f32 * TILE_SIZE + border;
                let z0 = tile.0 as f32 * TILE_SIZE - center - border;
                let z1 = (tile.0 + 1) as f32 * TILE_SIZE - center + border;
                let min = tri[0].min(tri[1]).min(tri[2]);
                let max = tri[0].max(tri[1]).max(tri[2]);
                footprints_overlap(min, max, Vec3::new(x0, 0.0, z0), Vec3::new(x1, 0.0, z1))
            }));
        self.triangles.len() - before
    }
}

fn transformed(triangles: &[[Vec3; 3]], world: Affine3A) -> impl Iterator<Item = [Vec3; 3]> + '_ {
    triangles
        .iter()
        .map(move |tri| tri.map(|point| world.transform_point3(point)))
}

/// Whether the placed model's bounds reach into the tile's XZ footprint.
fn overlaps_tile(
    model: &ModelGeometry,
    world: Affine3A,
    (row, col): (u32, u32),
    border: f32,
) -> bool {
    if model.triangles.is_empty() {
        return false;
    }
    let (min, max) = (0..8)
        .map(|corner| {
            let pick = |bit: usize, axis: usize| {
                if corner >> bit & 1 == 0 {
                    model.min[axis]
                } else {
                    model.max[axis]
                }
            };
            world.transform_point3(Vec3::new(pick(0, 0), pick(1, 1), pick(2, 2)))
        })
        .fold(
            (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
            |(min, max), point| (min.min(point), max.max(point)),
        );
    let center = 32.0 * TILE_SIZE;
    // Row `r` spans Bevy z in [r*T - center, (r+1)*T - center); column `c` spans Bevy x
    // in (center - (c+1)*T, center - c*T].
    let (z0, z1) = (
        row as f32 * TILE_SIZE - center,
        (row + 1) as f32 * TILE_SIZE - center,
    );
    let (x0, x1) = (
        center - (col + 1) as f32 * TILE_SIZE,
        center - col as f32 * TILE_SIZE,
    );
    footprints_overlap(
        min,
        max,
        Vec3::new(x0 - border, 0.0, z0 - border),
        Vec3::new(x1 + border, 0.0, z1 + border),
    )
}

fn footprints_overlap(min: Vec3, max: Vec3, foot_min: Vec3, foot_max: Vec3) -> bool {
    max.xz().cmpge(foot_min.xz()).all() && min.xz().cmple(foot_max.xz()).all()
}

fn water_quad_bounds(layer: &crate::liquid::WaterLayer, x: f32, z: f32) -> Vec<(Vec3, Vec3)> {
    const QUAD_SIZE: f32 = (100.0 / 3.0) / 8.0;
    const WATER_MIN_Y: f32 = -2000.0;
    let mut bounds = Vec::new();
    for row in 0..8 {
        for col in 0..8 {
            let Some(level) = layer.level_at(row as f32 + 0.5, col as f32 + 0.5) else {
                continue;
            };
            let min = Vec3::new(
                x - (row + 1) as f32 * QUAD_SIZE,
                WATER_MIN_Y,
                z + col as f32 * QUAD_SIZE,
            );
            let max = Vec3::new(
                x - row as f32 * QUAD_SIZE,
                level,
                z + (col + 1) as f32 * QUAD_SIZE,
            );
            bounds.push((min, max));
        }
    }
    bounds
}

/// Shared vertices for identical corners.
pub fn index_triangles(triangles: &[[Vec3; 3]]) -> (Vec<Vec3>, Vec<[u32; 3]>) {
    let mut vertices = Vec::new();
    let mut lookup: HashMap<[u32; 3], u32> = HashMap::new();
    let indexed = triangles
        .iter()
        .map(|tri| {
            tri.map(|point| {
                *lookup
                    .entry(point.to_array().map(f32::to_bits))
                    .or_insert_with(|| {
                        vertices.push(point);
                        (vertices.len() - 1) as u32
                    })
            })
        })
        .collect();
    (vertices, indexed)
}

/// Model `fdid`'s primary skin: `models/<SFID>.skin`, else the path-named
/// `<fdid>00.skin`.
pub(crate) fn primary_skin(models: &Path, fdid: u32, skin_fdid: Option<u32>) -> Result<Vec<u8>> {
    let names = skin_fdid
        .map(|skin| format!("{skin}.skin"))
        .into_iter()
        .chain([format!("{fdid}00.skin")]);
    for name in names {
        if let Ok(data) = std::fs::read(models.join(name)) {
            return Ok(data);
        }
    }
    Err(format!("no skin (SFID {skin_fdid:?}) extracted"))
}
