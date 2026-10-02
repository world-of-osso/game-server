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

use glam::{Affine3A, Vec3};

use crate::{LosTile, MAP_TILES, TILE_SIZE, tile_of};
use adt::{ModelRef, Placement, TileFiles};
use chunks::Result;

/// Default doodad thickness cutoff in yards (spec: "start near 1 yd").
pub const DEFAULT_THICKNESS_CUTOFF: f32 = 1.0;

/// A model's opaque triangles in model-local Bevy space and their bounds.
pub struct ModelGeometry {
    pub triangles: Vec<[Vec3; 3]>,
    pub min: Vec3,
    pub max: Vec3,
    /// Triangles of alpha-tested or blended batches.
    pub dropped_triangles: usize,
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
        }
    }

    /// Smallest side of the bounding box at `scale`.
    pub fn thickness(&self, scale: f32) -> f32 {
        ((self.max - self.min) * scale).min_element()
    }
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
            m2s: HashMap::new(),
            wmos: HashMap::new(),
            dropped: HashMap::new(),
            dropped_placements: HashSet::new(),
            problems: Vec::new(),
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

    fn path_fdid(&self, path: &str) -> Option<u32> {
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
        let files = tiles[(col * MAP_TILES + row) as usize];
        if files.root == 0 {
            return Ok(None);
        }
        let root = self
            .read("terrain", &format!("{}.adt", files.root))
            .map_err(|err| format!("terrain/{}.adt: {err}", files.root))?;
        let terrain = adt::terrain_triangles(&root)?;
        let mut build = TileBuild {
            tile: (row, col),
            stats: TileStats {
                terrain_triangles: terrain.len(),
                ..TileStats::default()
            },
            triangles: terrain,
        };
        let (doodads, wmos) = self.neighbourhood_placements(tiles, (row, col))?;
        for placement in &wmos {
            self.place_wmo(placement, &mut build);
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
            self.place_doodad(&doodad, &mut build);
        }
        let (vertices, indexed) = index_triangles(&build.triangles);
        Ok(Some((
            LosTile::build(row, col, vertices, indexed),
            build.stats,
        )))
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
        let ModelRef::Fdid(fdid) = placement.model else {
            return;
        };
        let Some(model) = self.wmo(fdid) else {
            return;
        };
        let world = placement.world_from_local();
        if overlaps_tile(&model.geometry, world, build.tile) {
            build.stats.wmo_triangles += build.add(&model.geometry.triangles, world);
            build.stats.alpha_or_blended_triangles += model.geometry.dropped_triangles;
        }
        // WMO file space -> WMO-local Bevy space: (x, y, z) -> (x, z, -y).
        let file_to_bevy = Affine3A::from_cols_array(&[
            1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0,
        ]);
        for doodad in model.root.doodads(placement.doodad_set) {
            let local = file_to_bevy
                * Affine3A::from_scale_rotation_translation(
                    Vec3::splat(doodad.scale),
                    glam::Quat::from_array(doodad.rotation).normalize(),
                    Vec3::from(doodad.position),
                )
                * file_to_bevy.inverse();
            // WMO doodads have no unique id of their own: the report keys them by the WMO's.
            let placed = PlacedDoodad {
                fdid: doodad.fdid,
                unique_id: placement.unique_id,
                scale: doodad.scale * placement.scale,
                world: world * local,
            };
            self.place_doodad(&placed, build);
        }
    }

    fn place_doodad(&mut self, doodad: &PlacedDoodad, build: &mut TileBuild) {
        let Some(model) = self.m2(doodad.fdid) else {
            return;
        };
        if model.triangles.is_empty() || !overlaps_tile(&model, doodad.world, build.tile) {
            return;
        }
        let thickness = model.thickness(doodad.scale);
        if thickness >= self.cutoff {
            build.stats.alpha_or_blended_triangles += model.dropped_triangles;
            build.stats.doodad_triangles += build.add(&model.triangles, doodad.world);
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
        let skin = self.read_skin(fdid, file.skin_fdid)?;
        let (triangles, dropped) = file.opaque_triangles(&skin)?;
        let triangles = triangles
            .into_iter()
            .map(|tri| tri.map(adt::file_to_bevy))
            .collect();
        Ok(ModelGeometry::new(triangles, dropped))
    }

    /// The primary skin: `models/<SFID>.skin`, else the path-named `<fdid>00.skin`.
    fn read_skin(&self, fdid: u32, skin_fdid: Option<u32>) -> Result<Vec<u8>> {
        let names = skin_fdid
            .map(|skin| format!("{skin}.skin"))
            .into_iter()
            .chain([format!("{fdid}00.skin")]);
        for name in names {
            if let Ok(data) = self.read("models", &name) {
                return Ok(data);
            }
        }
        Err(format!("no skin (SFID {skin_fdid:?}) extracted"))
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
            let (group_triangles, group_dropped) = root
                .group_triangles(&group_data)
                .map_err(|err| format!("group {group}: {err}"))?;
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

/// The triangles one tile keeps and its counts.
struct TileBuild {
    tile: (u32, u32),
    triangles: Vec<[Vec3; 3]>,
    stats: TileStats,
}

impl TileBuild {
    /// Adds the placed triangles whose centroid lies in the tile; returns how many.
    fn add(&mut self, triangles: &[[Vec3; 3]], world: Affine3A) -> usize {
        let before = self.triangles.len();
        let tile = self.tile;
        self.triangles
            .extend(transformed(triangles, world).filter(|tri| {
                let centroid = (tri[0] + tri[1] + tri[2]) / 3.0;
                tile_of(centroid.x, centroid.z) == tile
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
fn overlaps_tile(model: &ModelGeometry, world: Affine3A, (row, col): (u32, u32)) -> bool {
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
    max.x >= x0 && min.x <= x1 && max.z >= z0 && min.z <= z1
}

/// Shared vertices for identical corners.
fn index_triangles(triangles: &[[Vec3; 3]]) -> (Vec<Vec3>, Vec<[u32; 3]>) {
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
