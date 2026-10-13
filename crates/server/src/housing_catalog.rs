//! Qualified authored housing coordinates; catalog membership never grants property.
use crate::maps::WorldLocation;
use bevy::prelude::*;
use rusqlite::{Connection, params};
use shared::protocol::HousingPlot;
use std::collections::BTreeMap;

type Fields = BTreeMap<String, String>;

#[path = "housing_cornerstones.rs"]
mod cornerstones;
#[path = "housing_dye_catalog.rs"]
mod dyes;
pub(crate) use dyes::DyeCatalog;

#[derive(Clone, Debug)]
pub(crate) struct Neighborhood {
    pub id: u32,
    pub entry: WorldLocation,
}

#[derive(Resource, Clone, Debug)]
pub(crate) struct HousingCatalog {
    neighborhoods: BTreeMap<u32, Neighborhood>,
    plots: BTreeMap<u32, HousingPlot>,
    cornerstones: BTreeMap<u32, (u32, crate::game_objects::GameObjectSpawn)>,
    bulletin_boards: BTreeMap<u32, (u32, crate::game_objects::GameObjectSpawn)>,
    home_entry: Option<WorldLocation>,
    decor_scales: BTreeMap<u32, (u32, f32)>,
    pub(crate) dyes: DyeCatalog,
}

impl HousingCatalog {
    pub(crate) fn read(db: &Connection) -> Result<Self, String> {
        read_build_key(db)?;
        let neighborhoods = Self::read_neighborhoods(db)?;
        let plots = Self::read_plots(db, &neighborhoods)?;
        let mut catalog = Self {
            neighborhoods,
            plots,
            cornerstones: BTreeMap::new(),
            bulletin_boards: BTreeMap::new(),
            dyes: DyeCatalog::read(db)?,
            decor_scales: read_rows(db, "HouseDecor")?
                .into_iter()
                .map(|(id, fields)| {
                    Ok((
                        id,
                        (
                            number(&fields, "ModelType")?,
                            number(&fields, "InitialScale")?,
                        ),
                    ))
                })
                .collect::<Result<_, String>>()?,
            home_entry: read_rows(db, "NeighborhoodMap")?
                .into_iter()
                .find(|(id, _)| *id == 7)
                .map(|(_, fields)| {
                    Ok::<_, String>(WorldLocation {
                        map_id: number(&fields, "MapID")?,
                        position: Vec3::from_array(vector(&fields, "EntryPosition")?),
                        orientation: number(&fields, "EntryRotation")?,
                    })
                })
                .transpose()?,
        };
        let objects = cornerstones::read_objects(db)?;
        catalog.cornerstones = cornerstones::read_cornerstones(db, &catalog, &objects)?;
        catalog.bulletin_boards = cornerstones::read_bulletin_boards(db, &catalog, &objects)?;
        Ok(catalog)
    }

    fn read_neighborhoods(db: &Connection) -> Result<BTreeMap<u32, Neighborhood>, String> {
        let mut neighborhoods = BTreeMap::new();
        for (id, fields) in read_rows(db, "NeighborhoodMap")? {
            let map_id = number(&fields, "MapID")?;
            // Shipping neighborhood maps from this pinned Map.db2. Record4/map2640
            // has no Map row; record7 is an interior, not a purchasable neighborhood.
            if matches!(map_id, 2735 | 2736) {
                let entry = WorldLocation {
                    map_id,
                    position: Vec3::from_array(vector(&fields, "EntryPosition")?),
                    orientation: number(&fields, "EntryRotation")?,
                };
                neighborhoods.insert(id, Neighborhood { id, entry });
            }
        }
        Ok(neighborhoods)
    }

    fn read_plots(
        db: &Connection,
        neighborhoods: &BTreeMap<u32, Neighborhood>,
    ) -> Result<BTreeMap<u32, HousingPlot>, String> {
        let mut plots = BTreeMap::new();
        for (id, fields) in read_rows(db, "NeighborhoodPlot")? {
            let neighborhood_id = number(&fields, "NeighborhoodMapID")?;
            if neighborhoods.contains_key(&neighborhood_id) {
                let plot = HousingPlot {
                    plot_id: id,
                    neighborhood_id,
                    plot_index: number(&fields, "PlotIndex")?,
                    name: field(&fields, "Name")?.to_owned(),
                    cornerstone: vector(&fields, "CornerstonePosition")?,
                    teleport: vector(&fields, "TeleportPosition")?,
                    cost: number(&fields, "Cost")?,
                    house_id: None,
                    owner_character_id: None,
                    owner_type: shared::protocol::HousingPlotOwnerType::None,
                };
                plots.insert(id, plot);
            }
        }
        Ok(plots)
    }

    pub(crate) fn validate_decor_scale(&self, decor_id: u32, scale: f32) -> Result<(), String> {
        let (model_type, initial_scale) = self
            .decor_scales
            .get(&decor_id)
            .ok_or_else(|| format!("Decor {decor_id} has no authenticated scale source."))?;
        if *model_type != 1 && scale != *initial_scale {
            return Err("Retail expert mode cannot scale WMO decor.".into());
        }
        Ok(())
    }

    pub(crate) fn home_entry(&self) -> Result<WorldLocation, String> {
        self.home_entry
            .filter(|entry| entry.map_id == 2783)
            .ok_or_else(|| "The authenticated Home Interior map entry is unavailable.".into())
    }

    pub(crate) fn cornerstones(&self) -> Vec<crate::game_objects::GameObjectSpawn> {
        self.cornerstones
            .values()
            .map(|(_, spawn)| spawn.clone())
            .collect()
    }

    pub(crate) fn cornerstone_plot(&self, entry: u32) -> Option<u32> {
        self.cornerstones.get(&entry).map(|(plot, _)| *plot)
    }

    /// Cornerstones and bulletin boards spawned in every neighborhood copy.
    pub(crate) fn neighborhood_objects(&self) -> Vec<crate::game_objects::GameObjectSpawn> {
        self.cornerstones
            .values()
            .chain(self.bulletin_boards.values())
            .map(|(_, spawn)| spawn.clone())
            .collect()
    }

    pub(crate) fn bulletin_board_neighborhood(&self, entry: u32) -> Option<u32> {
        self.bulletin_boards
            .get(&entry)
            .map(|(neighborhood, _)| *neighborhood)
    }

    pub(crate) fn neighborhood(&self, id: u32) -> Result<&Neighborhood, String> {
        self.neighborhoods.get(&id).ok_or_else(|| {
            format!("Neighborhood {id} is not supported by the authenticated catalog.")
        })
    }

    pub(crate) fn plot(&self, id: u32) -> Result<&HousingPlot, String> {
        self.plots
            .get(&id)
            .ok_or_else(|| format!("Plot {id} is not in a supported neighborhood."))
    }

    pub(crate) fn plots(&self, id: u32) -> Result<Vec<HousingPlot>, String> {
        self.neighborhood(id)?;
        Ok(self
            .plots
            .values()
            .filter(|plot| plot.neighborhood_id == id)
            .cloned()
            .collect())
    }
}

pub(crate) fn read_build_key(db: &Connection) -> Result<String, String> {
    crate::db2_source_build::read_source_build(db, "housing_catalog_build")?;
    let count: u32 = db
        .query_row("SELECT COUNT(*) FROM housing_catalog_build", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())?;
    if count != 1 {
        return Err("Housing catalog requires exactly one declared build key".into());
    }
    db.query_row("SELECT build_key FROM housing_catalog_build", [], |row| {
        row.get(0)
    })
    .map_err(|error| error.to_string())
}

fn read_rows(db: &Connection, table: &str) -> Result<Vec<(u32, Fields)>, String> {
    let mut query = db.prepare("SELECT record_id, fields_json FROM housing_catalog_record WHERE build_key=? AND table_name=? ORDER BY record_id").map_err(|error| error.to_string())?;
    let rows = query
        .query_map(params![read_build_key(db)?, table], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| {
        let (id, json) = row.map_err(|error| error.to_string())?;
        let fields = serde_json::from_str(&json)
            .map_err(|error| format!("Housing {table} {id}: {error}"))?;
        Ok((id, fields))
    })
    .collect()
}

fn field<'a>(fields: &'a Fields, name: &str) -> Result<&'a str, String> {
    fields
        .get(name)
        .map(String::as_str)
        .ok_or_else(|| format!("Housing catalog lacks {name}."))
}

fn number<T: std::str::FromStr>(fields: &Fields, name: &str) -> Result<T, String> {
    field(fields, name)?
        .parse()
        .map_err(|_| format!("Invalid housing field {name}."))
}

fn vector(fields: &Fields, name: &str) -> Result<[f32; 3], String> {
    let vector = [
        number(fields, &format!("{name}_0"))?,
        number(fields, &format!("{name}_1"))?,
        number(fields, &format!("{name}_2"))?,
    ];
    if vector.iter().any(|value: &f32| !value.is_finite()) {
        return Err(format!("Nonfinite housing coordinate {name}."));
    }
    Ok(vector)
}

#[cfg(test)]
#[path = "housing_catalog_tests.rs"]
pub(crate) mod tests;
