//! Admission-only disk parsing. The tick thread installs owned prepared caches.
use super::*;

pub(crate) struct GroundPreparation {
    data_dir: PathBuf,
    map_id: u32,
    wdt: Option<u32>,
    cached: Option<Option<MapGround>>,
}

pub(crate) struct PreparedGround {
    map_id: u32,
    map: Option<MapGround>,
    m2: los::LosMap,
}

impl GroundMap {
    pub(crate) fn preparation(&self, map_id: u32) -> GroundPreparation {
        GroundPreparation {
            data_dir: self.data_dir.clone(),
            map_id,
            wdt: self.wdts.get(&map_id).copied(),
            cached: self.maps.get(&map_id).map(|map| {
                map.as_ref().map(|map| MapGround {
                    source: map.source.clone(),
                    global_wmo: map.global_wmo.clone(),
                    tiles: HashMap::new(),
                    wmo_models: map.wmo_models.clone(),
                })
            }),
        }
    }

    pub(crate) fn install_prepared(&mut self, prepared: PreparedGround) {
        match self.maps.entry(prepared.map_id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(prepared.map);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                match (entry.get_mut(), prepared.map) {
                    (Some(live), Some(region)) => {
                        live.tiles.extend(region.tiles);
                        live.wmo_models.extend(region.wmo_models);
                    }
                    (live @ None, Some(region)) => *live = Some(region),
                    _ => {}
                }
            }
        }
        match self.m2_floors.entry(prepared.map_id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(prepared.m2);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().merge_preloaded(prepared.m2);
            }
        }
    }
}

impl GroundPreparation {
    pub(crate) fn load(self, tiles: &[(u32, u32)]) -> Result<PreparedGround, String> {
        let mut map = match self.cached {
            Some(cached) => cached,
            None => open_optional_map(&self.data_dir, self.wdt)?,
        };
        if let Some(map) = &mut map {
            for &tile in tiles {
                let loaded = load_optional_tile(map, tile)?;
                map.tiles.insert(tile, loaded);
            }
        }
        let mut m2 = los::LosMap::new(&self.data_dir.join(format!("los/{}/m2", self.map_id)));
        m2.preload_tiles(tiles.iter().copied())?;
        Ok(PreparedGround {
            map_id: self.map_id,
            map,
            m2,
        })
    }
}

fn read_optional(
    data_dir: &Path,
    directory: &str,
    fdid: u32,
    extension: &str,
) -> Result<Option<Vec<u8>>, String> {
    if fdid == 0 {
        return Ok(None);
    }
    let path = data_dir.join(format!("{directory}/{fdid}.{extension}"));
    match std::fs::read(&path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn open_optional_map(data_dir: &Path, wdt: Option<u32>) -> Result<Option<MapGround>, String> {
    let Some(wdt) = wdt else { return Ok(None) };
    let Some(bytes) = read_optional(data_dir, "terrain", wdt, "wdt")? else {
        return Ok(None);
    };
    let mphd = find_chunk(&bytes, b"MPHD")?.ok_or("WDT has no MPHD")?;
    if read_u32(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO != 0 {
        return open_optional_global_wmo(data_dir, &bytes);
    }
    Ok(Some(MapGround {
        source: Some(TileSource {
            data_dir: data_dir.to_path_buf(),
            tiles: parse_maid(&bytes)?,
        }),
        ..default()
    }))
}

fn open_optional_global_wmo(data_dir: &Path, wdt: &[u8]) -> Result<Option<MapGround>, String> {
    let modf = find_chunk(wdt, b"MODF")?.ok_or("global-WMO WDT has no MODF")?;
    let entry = modf
        .get(..MODF_ENTRY_SIZE)
        .ok_or("global-WMO WDT MODF is empty")?;
    let placement = parse_modf_entry(entry)?;
    let Some(groups) = load_optional_groups(data_dir, placement.fdid)? else {
        return Ok(None);
    };
    let transform = Affine3A::from_scale_rotation_translation(
        Vec3::splat(placement.scale),
        placement_rotation(placement.rotation),
        global_wmo_placement_position(placement.position),
    );
    Ok(Some(MapGround {
        global_wmo: Some(WmoCollision::new(transform, groups)),
        ..default()
    }))
}

fn load_optional_tile(
    map: &mut MapGround,
    (row, col): (u32, u32),
) -> Result<Option<TileGround>, String> {
    let Some(source) = &map.source else {
        return Ok(None);
    };
    let files = source.tiles[col as usize * MAP_TILES + row as usize];
    let data_dir = source.data_dir.clone();
    let Some(bytes) = read_optional(&data_dir, "terrain", files.root, "adt")? else {
        return Ok(None);
    };
    let (terrain, flight_min) = parse_terrain(&bytes)?;
    let placements = optional_placements(&data_dir, files.obj0)?;
    let mut wmos = Vec::new();
    for placement in placements {
        if !map.wmo_models.contains_key(&placement.fdid) {
            let Some(groups) = load_optional_groups(&data_dir, placement.fdid)? else {
                continue;
            };
            map.wmo_models.insert(placement.fdid, groups);
        }
        wmos.push(WmoCollision::new(
            placement.world_from_local(),
            map.wmo_models[&placement.fdid].clone(),
        ));
    }
    Ok(Some(TileGround {
        terrain,
        wmos,
        flight_min,
    }))
}

fn optional_placements(data_dir: &Path, fdid: u32) -> Result<Vec<WmoPlacement>, String> {
    let Some(bytes) = read_optional(data_dir, "terrain", fdid, "adt")? else {
        return Ok(Vec::new());
    };
    let Some(modf) = find_chunk(&bytes, b"MODF")? else {
        return Ok(Vec::new());
    };
    if modf.len() % MODF_ENTRY_SIZE != 0 {
        return Err(format!("MODF {fdid} has a truncated placement"));
    }
    modf.chunks_exact(MODF_ENTRY_SIZE)
        .map(parse_modf_entry)
        .collect()
}

fn load_optional_groups(
    data_dir: &Path,
    root: u32,
) -> Result<Option<Vec<Arc<WmoGroupCollision>>>, String> {
    let Some(bytes) = read_optional(data_dir, "models", root, "wmo")? else {
        return Ok(None);
    };
    let mut groups = Vec::new();
    for fdid in group_file_data_ids(&bytes)? {
        let Some(bytes) = read_optional(data_dir, "models", fdid, "wmo")? else {
            return Ok(None);
        };
        let parsed = WmoGroupCollision::parse(&bytes)
            .map_err(|error| format!("WMO group {fdid}: {error}"))?;
        groups.push(Arc::new(parsed));
    }
    Ok(Some(groups))
}
