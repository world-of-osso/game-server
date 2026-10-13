//! Authenticated GameObjects references, not substitutions at plot-marker positions.
//! Cornerstones and bulletin boards are both GAMEOBJECT_TYPE_UI_LINK (48) objects.
use super::{HousingCatalog, number, read_rows, vector};
use crate::game_objects::{GameObjectBounds, GameObjectSpawn};
use bevy::prelude::*;
use rusqlite::Connection;
use std::collections::BTreeMap;

use shared::protocol::GAMEOBJECT_TYPE_HOUSING_PLOT;

pub(super) type Objects = BTreeMap<u32, super::Fields>;

pub(super) fn read_objects(db: &Connection) -> Result<Objects, String> {
    Ok(read_rows(db, "GameObjects")?.into_iter().collect())
}

pub(super) fn read_cornerstones(
    db: &Connection,
    catalog: &HousingCatalog,
    objects: &Objects,
) -> Result<BTreeMap<u32, (u32, GameObjectSpawn)>, String> {
    let mut stones = BTreeMap::new();
    for (plot_id, plot) in read_rows(db, "NeighborhoodPlot")? {
        let Some(property) = catalog.plots.get(&plot_id) else {
            continue;
        };
        let object_id = number(&plot, "CornerstoneGameObjectID")?;
        let Some(fields) = objects.get(&object_id) else {
            warn!(
                "Housing plot {plot_id}: no authenticated GameObjects{object_id} cornerstone; unavailable."
            );
            continue;
        };
        let map_id = catalog.neighborhood(property.neighborhood_id)?.entry.map_id;
        let spawn = read_ui_link(db, object_id, map_id, fields)?;
        stones.insert(object_id, (plot_id, spawn));
    }
    Ok(stones)
}

/// PropValue_7 of a UI link is its PlayerInteractionType; 72 is HousingBulletinBoard.
const INTERACTION_HOUSING_BULLETIN_BOARD: u32 = 72;

/// Each neighborhood map's authored board, keyed by GameObjects ID.
pub(super) fn read_bulletin_boards(
    db: &Connection,
    catalog: &HousingCatalog,
    objects: &Objects,
) -> Result<BTreeMap<u32, (u32, GameObjectSpawn)>, String> {
    let mut boards = BTreeMap::new();
    for (&object_id, fields) in objects {
        let is_board = fields
            .get("PropValue_7")
            .and_then(|value| value.parse().ok())
            == Some(INTERACTION_HOUSING_BULLETIN_BOARD)
            && number::<u8>(fields, "TypeID")? == GAMEOBJECT_TYPE_HOUSING_PLOT;
        if !is_board {
            continue;
        }
        let owner: u32 = number(fields, "OwnerID")?;
        let Some(neighborhood) = catalog
            .neighborhoods
            .values()
            .find(|neighborhood| neighborhood.entry.map_id == owner)
        else {
            continue;
        };
        let spawn = read_ui_link(db, object_id, owner, fields)?;
        boards.insert(object_id, (neighborhood.id, spawn));
    }
    Ok(boards)
}

fn read_ui_link(
    db: &Connection,
    object_id: u32,
    map_id: u32,
    fields: &super::Fields,
) -> Result<GameObjectSpawn, String> {
    let owner: u32 = number(fields, "OwnerID")?;
    let go_type: u8 = number(fields, "TypeID")?;
    if owner != map_id || go_type != GAMEOBJECT_TYPE_HOUSING_PLOT {
        return Err(format!(
            "GameObjects{object_id} is not a housing UI link on map{map_id}."
        ));
    }
    let scale: f32 = number(fields, "Scale")?;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(format!("GameObjects{object_id} has invalid Scale."));
    }
    let orientation = read_heading(fields)?;
    let display_id = number(fields, "DisplayID")?;
    let bounds = read_bounds(db, display_id, scale, orientation)?;
    Ok(GameObjectSpawn {
        guid: object_id,
        map_id,
        info: shared::protocol::GameObjectInfo {
            entry: object_id,
            go_type,
            display_id,
            name: super::field(fields, "Name_lang")?.to_owned(),
            scale,
        },
        position: Vec3::from_array(vector(fields, "Pos")?),
        orientation,
        // TC GameObject::Create defaults script-created objects to GO_STATE_READY.
        initial_state: 1,
        bounds,
        // Housing has no MapDifficulty: 0 selects non-dungeon content only.
        spawn_difficulties: vec![0],
        chair: None,
        quest_object: None,
        script_name: String::new(),
    })
}

fn read_heading(fields: &super::Fields) -> Result<f32, String> {
    let [x, y, z] = vector(fields, "Rot")?;
    let w: f32 = number(fields, "Rot_3")?;
    let length = Vec4::new(x, y, z, w).length();
    if x.abs() > 0.0001 || y.abs() > 0.0001 || !w.is_finite() || (length - 1.0).abs() > 0.001 {
        return Err(
            "Cornerstone quaternion cannot be represented by the gameobject yaw transform.".into(),
        );
    }
    Ok(2.0 * z.atan2(w))
}

fn read_bounds(
    db: &Connection,
    display: u32,
    scale: f32,
    orientation: f32,
) -> Result<GameObjectBounds, String> {
    let values: [f32; 6] = db.query_row(
        "SELECT GeoBox_0,GeoBox_1,GeoBox_2,GeoBox_3,GeoBox_4,GeoBox_5 FROM game_object_display_info WHERE ID=?",
        [display],
        |row| Ok([row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?]),
    ).map_err(|error| format!("Cornerstone display{display} bounds: {error}"))?;
    let min = Vec3::new(values[0], values[1], values[2]);
    let max = Vec3::new(values[3], values[4], values[5]);
    if !min.is_finite() || !max.is_finite() || !min.cmple(max).all() {
        return Err(format!("Cornerstone display{display} has invalid bounds."));
    }
    Ok(GameObjectBounds {
        min,
        max,
        scale,
        orientation,
    })
}
