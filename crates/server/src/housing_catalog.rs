//! Qualified authored housing coordinates; catalog membership never grants property.
use crate::maps::WorldLocation;
use crate::persistence::housing_store::BUILD_KEY;
use bevy::prelude::*;
use rusqlite::{Connection, params};
use shared::protocol::HousingPlot;
use std::collections::BTreeMap;

const BUILD: &str = "12.1.0.69933";
type Fields = BTreeMap<String, String>;

#[derive(Clone, Debug)]
pub(crate) struct Neighborhood {
    pub id: u32,
    pub entry: WorldLocation,
}

#[derive(Resource, Clone, Debug)]
pub(crate) struct HousingCatalog {
    neighborhoods: BTreeMap<u32, Neighborhood>,
    plots: BTreeMap<u32, HousingPlot>,
}

impl HousingCatalog {
    pub(crate) fn read(db: &Connection) -> Result<Self, String> {
        let build: String = db
            .query_row(
                "SELECT build FROM housing_catalog_build WHERE build_key=?",
                [BUILD_KEY],
                |row| row.get(0),
            )
            .map_err(|error| {
                format!(
                    "Housing catalog build {BUILD}: {error}; apply the housing-catalog lead import."
                )
            })?;
        if build != BUILD {
            return Err(format!("Housing catalog build {build}; expected {BUILD}."));
        }
        let neighborhoods = Self::read_neighborhoods(db)?;
        let plots = Self::read_plots(db, &neighborhoods)?;
        Ok(Self {
            neighborhoods,
            plots,
        })
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
                };
                plots.insert(id, plot);
            }
        }
        Ok(plots)
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

fn read_rows(db: &Connection, table: &str) -> Result<Vec<(u32, Fields)>, String> {
    let mut query = db.prepare("SELECT record_id, fields_json FROM housing_catalog_record WHERE build_key=? AND table_name=? ORDER BY record_id").map_err(|error| error.to_string())?;
    let rows = query
        .query_map(params![BUILD_KEY, table], |row| {
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
