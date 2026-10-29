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

#[derive(Clone, Debug, Serialize)]
pub struct AnchorRow {
    pub table: &'static str,
    pub guid: i64,
}

#[derive(Clone, Debug)]
pub struct AuthoredAnchor {
    pub row: AnchorRow,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Resolution {
    Resolved {
        z: f32,
        method: &'static str,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        anchor_rows: Vec<AnchorRow>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        hint_ids: Vec<String>,
    },
    Gap {
        gap: Gap,
    },
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
        self.resolve_with_hints(request, &[], &[])
    }

    pub fn resolve_with_hints(
        &mut self,
        request: &Request,
        known_hints: &[(f32, f32, f32, String)],
        authored: &[AuthoredAnchor],
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
                anchor_rows: Vec::new(),
                hint_ids: Vec::new(),
            };
        }
        if !authored.is_empty() {
            return self.resolve_authored(request, candidates, authored);
        }
        let mut matching = vec![false; candidates.len()];
        let mut anchors = known_hints.to_vec();
        for hint in &request.neighbor_hints {
            if let Ok(hint_floors) = self.candidates(request.map, hint.x, hint.y) {
                if hint_floors.len() == 1 {
                    anchors.push((hint.x, hint.y, hint_floors[0].z, hint.guid.clone()));
                }
            }
        }
        let mut matching_hints: Vec<Vec<String>> = vec![Vec::new(); candidates.len()];
        let mean_hint_z =
            anchors.iter().map(|anchor| anchor.2).sum::<f32>() / anchors.len().max(1) as f32;
        for (x, y, z, hint_id) in anchors {
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
                            if !hint_id.is_empty() {
                                matching_hints[index].push(hint_id.clone());
                            }
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
        if let (Some(first), Some(last)) = (selected.first(), selected.last()) {
            if candidates[last.0].z - candidates[first.0].z <= navigation::AGENT_CLIMB {
                let index = selected
                    .iter()
                    .map(|entry| entry.0)
                    .min_by(|a, b| {
                        (candidates[*a].z - mean_hint_z)
                            .abs()
                            .total_cmp(&(candidates[*b].z - mean_hint_z).abs())
                    })
                    .unwrap();
                return Resolution::Resolved {
                    z: candidates[index].z,
                    method: "neighbor_path",
                    anchor_rows: Vec::new(),
                    hint_ids: matching_hints[index].clone(),
                };
            }
        }
        Resolution::Gap {
            gap: Gap::NamedGap { candidates },
        }
    }

    fn resolve_authored(
        &mut self,
        request: &Request,
        candidates: Vec<Floor>,
        anchors: &[AuthoredAnchor],
    ) -> Resolution {
        let min = anchors
            .iter()
            .map(|anchor| anchor.z)
            .fold(f32::INFINITY, f32::min);
        let max = anchors
            .iter()
            .map(|anchor| anchor.z)
            .fold(f32::NEG_INFINITY, f32::max);
        if max - min > navigation::AGENT_CLIMB {
            return Resolution::Gap {
                gap: Gap::NamedGap { candidates },
            };
        }
        let mean = anchors.iter().map(|anchor| anchor.z).sum::<f32>() / anchors.len() as f32;
        let nav = self.maps.get_mut(&request.map).unwrap();
        let mut matches = Vec::new();
        for candidate in &candidates {
            if (candidate.z - mean).abs() > navigation::AGENT_CLIMB {
                continue;
            }
            let end = point(request.x, request.y, candidate.z);
            let mut connected = true;
            for anchor in anchors {
                let start = point(anchor.x, anchor.y, anchor.z);
                match nav.floor_polygon(start) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        connected = false;
                        break;
                    }
                    Err(error) => {
                        return Resolution::Gap {
                            gap: Gap::MissingGeometry { error },
                        };
                    }
                }
                let max_distance =
                    (start - point(request.x, request.y, anchor.z)).length() * 3.0 + 15.0;
                match nav.complete_path(start, end) {
                    Ok(path)
                        if path
                            .windows(2)
                            .map(|pair| pair[0].distance(pair[1]))
                            .sum::<f32>()
                            <= max_distance => {}
                    Ok(_) => {
                        connected = false;
                        break;
                    }
                    Err(error) if error.starts_with("unreachable target") => {
                        connected = false;
                        break;
                    }
                    Err(error) => {
                        return Resolution::Gap {
                            gap: Gap::MissingGeometry { error },
                        };
                    }
                }
            }
            if connected {
                matches.push(candidate);
            }
        }
        if let Some(candidate) = matches
            .into_iter()
            .min_by(|a, b| (a.z - mean).abs().total_cmp(&(b.z - mean).abs()))
        {
            return Resolution::Resolved {
                z: candidate.z,
                method: "authored_neighbor_anchor",
                anchor_rows: anchors.iter().map(|anchor| anchor.row.clone()).collect(),
                hint_ids: Vec::new(),
            };
        }
        Resolution::Gap {
            gap: Gap::NamedGap { candidates },
        }
    }
}
