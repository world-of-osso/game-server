//! A map's baked tiles, loaded on first use, and segment queries across tile borders.
//!
//! The bake stores each triangle in the tile holding its centroid, so a triangle can
//! overhang into a neighbour by part of its size. A query tests the tiles under the
//! segment and their neighbours.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use glam::Vec3;

use crate::{LosTile, MAP_TILES, tile_file_name, tile_of};

pub struct LosMap {
    dir: PathBuf,
    /// `None`: the bake has no file for the tile (no ADT there).
    tiles: HashMap<(u32, u32), Option<LosTile>>,
}

impl LosMap {
    /// `dir` holds the map's `<row>_<col>.los` files.
    pub fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            tiles: HashMap::new(),
        }
    }

    /// Whether the open segment `a..b` crosses no baked triangle; loads the tiles it needs.
    pub fn clear(&mut self, a: Vec3, b: Vec3) -> Result<bool, String> {
        self.load_around(a, b)?;
        Ok(!self
            .tiles_around(a, b)
            .filter_map(|tile| self.tiles.get(&tile)?.as_ref())
            .any(|tile| tile.blocks(a, b)))
    }

    /// Segment parameter of the first baked triangle `a..b` crosses.
    pub fn first_hit(&mut self, a: Vec3, b: Vec3) -> Result<Option<f32>, String> {
        self.load_around(a, b)?;
        Ok(self
            .tiles_around(a, b)
            .filter_map(|tile| self.tiles.get(&tile)?.as_ref()?.first_hit(a, b))
            .min_by(f32::total_cmp))
    }

    /// The loaded tile `(row, col)`, if its file exists.
    pub fn tile(&mut self, tile: (u32, u32)) -> Result<Option<&LosTile>, String> {
        self.load(tile)?;
        Ok(self.tiles[&tile].as_ref())
    }

    fn load_around(&mut self, a: Vec3, b: Vec3) -> Result<(), String> {
        for tile in self.tiles_around(a, b).collect::<Vec<_>>() {
            self.load(tile)?;
        }
        Ok(())
    }

    fn load(&mut self, tile: (u32, u32)) -> Result<(), String> {
        if self.tiles.contains_key(&tile) {
            return Ok(());
        }
        let path = self.dir.join(tile_file_name(tile));
        let loaded = match std::fs::read(&path) {
            Ok(data) => Some(
                LosTile::from_bytes(&data).map_err(|err| format!("{}: {err}", path.display()))?,
            ),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(format!("{}: {err}", path.display())),
        };
        self.tiles.insert(tile, loaded);
        Ok(())
    }

    /// Tiles under the segment's XZ bounds and one ring around them.
    fn tiles_around(&self, a: Vec3, b: Vec3) -> impl Iterator<Item = (u32, u32)> + use<> {
        let (row_a, col_a) = tile_of(a.x, a.z);
        let (row_b, col_b) = tile_of(b.x, b.z);
        let span = |x: u32, y: u32| x.min(y).saturating_sub(1)..=(x.max(y) + 1).min(MAP_TILES - 1);
        let cols = span(col_a, col_b);
        span(row_a, row_b).flat_map(move |row| cols.clone().map(move |col| (row, col)))
    }
}
