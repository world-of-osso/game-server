//! Offline floor resolution from strict geometry and baked navigation.
use std::{collections::HashMap, path::Path};

use bevy::math::Vec3;
use navigation::NavigationMap;
use serde::{Deserialize, Serialize};

pub use crate::ground::{Floor, FloorGeometry, MissingGeometry};
#[cfg(test)]
#[path = "floor_resolver_tests.rs"]
mod tests;

#[derive(Clone, Debug, Deserialize)]
pub struct Hint {
    #[serde(default)]
    pub guid: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub run_start: String,
    #[serde(default)]
    pub guid: String,
    pub map: u32,
    pub x: f32,
    pub y: f32,
    pub ui_map_id: Option<u32>,
    #[serde(default)]
    pub neighbor_hints: Vec<Hint>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Resolution {
    Resolved { z: f32, method: &'static str },
    Gap { gap: Gap },
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Gap {
    MissingGeometry { error: String },
    NamedGap { candidates: Vec<Floor> },
}

pub struct FloorResolver {
    geometry: FloorGeometry,
    nav_dir: std::path::PathBuf,
    maps: HashMap<u32, NavigationMap>,
}

fn point(x: f32, y: f32, z: f32) -> Vec3 {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(x, y, z);
    Vec3::new(x, y, z)
}

impl FloorResolver {
    pub fn new(geometry: &Path, nav_dir: &Path, wdts: HashMap<u32, u32>) -> Self {
        Self {
            geometry: FloorGeometry::new(geometry, wdts),
            nav_dir: nav_dir.to_owned(),
            maps: HashMap::new(),
        }
    }

    fn candidates(&mut self, map: u32, x: f32, y: f32) -> Result<Vec<Floor>, String> {
        let candidates = self
            .geometry
            .candidate_floors(map, x, y)
            .map_err(|error| error.0)?;
        if !self.maps.contains_key(&map) {
            self.maps.insert(
                map,
                NavigationMap::open(&self.nav_dir.join(map.to_string()))?,
            );
        }
        let nav = self.maps.get_mut(&map).unwrap();
        let mut floors = Vec::new();
        for candidate in candidates {
            if nav.floor_polygon(point(x, y, candidate.z))?.is_some() {
                floors.push(candidate);
            }
        }
        floors.sort_by(|a, b| a.z.total_cmp(&b.z));
        floors.dedup_by(|a, b| (a.z - b.z).abs() < 0.1);
        Ok(floors)
    }

    pub fn resolve(&mut self, request: &Request) -> Resolution {
        self.resolve_with_known_hints(request, &[])
    }

    pub fn resolve_with_known_hints(
        &mut self,
        request: &Request,
        known_hints: &[(f32, f32, f32)],
    ) -> Resolution {
        let candidates = match self.candidates(request.map, request.x, request.y) {
            Ok(candidates) => candidates,
            Err(error) => {
                return Resolution::Gap {
                    gap: Gap::MissingGeometry { error },
                };
            }
        };
        if candidates.len() == 1 {
            return Resolution::Resolved {
                z: candidates[0].z,
                method: "single_candidate",
            };
        }
        let mut matching = vec![false; candidates.len()];
        let mut anchors = known_hints.to_vec();
        for hint in &request.neighbor_hints {
            if let Ok(hint_floors) = self.candidates(request.map, hint.x, hint.y) {
                if hint_floors.len() == 1 {
                    anchors.push((hint.x, hint.y, hint_floors[0].z));
                }
            }
        }
        for (x, y, z) in anchors {
            let start = point(x, y, z);
            let max_distance = (start - point(request.x, request.y, z)).length() * 3.0 + 15.0;
            let nav = self.maps.get_mut(&request.map).unwrap();
            for (index, candidate) in candidates.iter().enumerate() {
                let end = point(request.x, request.y, candidate.z);
                match nav.complete_path(start, end) {
                    Ok(path) => {
                        let length: f32 = path
                            .windows(2)
                            .map(|points| points[0].distance(points[1]))
                            .sum();
                        if length <= max_distance {
                            matching[index] = true;
                        }
                    }
                    Err(error) if error.starts_with("unreachable target") => {}
                    Err(error) => {
                        return Resolution::Gap {
                            gap: Gap::MissingGeometry { error },
                        };
                    }
                }
            }
        }
        let selected: Vec<_> = matching
            .iter()
            .enumerate()
            .filter(|(_, matched)| **matched)
            .collect();
        if selected.len() == 1 {
            return Resolution::Resolved {
                z: candidates[selected[0].0].z,
                method: "neighbor_path",
            };
        }
        Resolution::Gap {
            gap: Gap::NamedGap { candidates },
        }
    }
}
