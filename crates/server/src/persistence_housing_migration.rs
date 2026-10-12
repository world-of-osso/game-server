//! One-time precision/access migrations. No legacy runtime reader.
use super::{BUILD_KEY, STATE, STATE_ROW};
use redb::{ReadableTable, WriteTransaction};
use rusqlite::Connection;
use serde_json::Value;

pub(super) const SCHEMA_VERSION: u64 = 2;

pub(super) fn version(bytes: &[u8]) -> Result<u64, String> {
    let state: Value =
        serde_json::from_slice(bytes).map_err(|e| format!("Read housing schema: {e}"))?;
    match state.get("schema_version") {
        None => Ok(0), // The shipped pre-precision schema had no version field.
        Some(value) => value
            .as_u64()
            .ok_or("Invalid housing schema version.".into()),
    }
}

pub(super) fn migrate(txn: &WriteTransaction, source: &Connection) -> Result<(), String> {
    let mut table = txn.open_table(STATE).map_err(|e| e.to_string())?;
    let bytes = table
        .get(STATE_ROW)
        .map_err(|e| e.to_string())?
        .ok_or("Missing housing state.")?
        .value()
        .to_vec();
    let version = version(&bytes)?;
    match version {
        SCHEMA_VERSION => return Ok(()),
        0 | 1 => {}
        version => return Err(format!("Unsupported housing schema {version}.")),
    }
    let mut state: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if state["build_key"].as_str() != Some(BUILD_KEY) {
        return Err("Housing migration requires the authenticated Retail build.".into());
    }
    if version == 0 {
        let build: String = source
            .query_row(
                "SELECT build FROM housing_catalog_build WHERE build_key=?",
                [BUILD_KEY],
                |row| row.get(0),
            )
            .map_err(|e| format!("Housing migration source: {e}"))?;
        if build != "12.1.0.69933" {
            return Err("Housing migration source build differs.".into());
        }
    }
    let houses = state["houses"]
        .as_object_mut()
        .ok_or("Missing housing houses.")?;
    for house in houses.values_mut() {
        if version == 0 {
            let decor = house["decor"]
                .as_array_mut()
                .ok_or("Missing house decor.")?;
            for placed in decor {
                source_missing_axes(source, placed)?;
            }
        }
        let object = house.as_object_mut().ok_or("Invalid house record.")?;
        let previous = object
            .remove("access_anyone")
            .ok_or("Missing previous house permission.")?;
        let access = match previous {
            Value::Null => Value::Null,
            Value::Bool(allowed) => serde_json::to_value(shared::protocol::HouseAccess {
                anyone: allowed,
                neighbors: allowed,
                guild: allowed,
                friends: allowed,
                party: allowed,
            })
            .map_err(|e| e.to_string())?,
            _ => return Err("Invalid previous house permission.".into()),
        };
        object.insert("house_access".into(), access);
    }
    state["schema_version"] = Value::from(SCHEMA_VERSION);
    // Validate the complete new representation before atomically replacing the row.
    let migrated: super::State = serde_json::from_value(state)
        .map_err(|e| format!("Validate migrated housing state: {e}"))?;
    let bytes = serde_json::to_vec(&migrated).map_err(|e| e.to_string())?;
    table
        .insert(STATE_ROW, bytes.as_slice())
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn source_missing_axes(source: &Connection, placed: &mut Value) -> Result<(), String> {
    let decor_id = placed["decor_id"]
        .as_u64()
        .ok_or("Missing placed decor identity.")?;
    let json: String = source.query_row("SELECT fields_json FROM housing_catalog_record WHERE build_key=? AND table_name='HouseDecor' AND record_id=?", rusqlite::params![BUILD_KEY, decor_id], |row| row.get(0)).map_err(|e| format!("Migration decor{decor_id} has no authenticated source: {e}"))?;
    let fields: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let number = |key: &str| -> Result<f32, String> {
        let value = fields
            .get(key)
            .ok_or_else(|| format!("Migration decor{decor_id} missing {key}."))?
            .parse::<f32>()
            .map_err(|e| format!("Migration {key}: {e}"))?;
        if !value.is_finite() {
            return Err(format!("Migration {key} must be finite."));
        }
        Ok(value)
    };
    let scale = number("InitialScale")?;
    if scale <= 0.0 {
        return Err(format!("Migration decor{decor_id} scale must be positive."));
    }
    let transform = placed["transform"]
        .as_object_mut()
        .ok_or("Missing legacy transform.")?;
    // DB2 rotations are degrees in Z-up axes. The engine's basis maps Y to -Z;
    // preserve the saved engine yaw, rather than overwriting it with initial Z.
    transform.insert(
        "pitch".into(),
        Value::from(number("InitialRotation_0")?.to_radians()),
    );
    transform.insert(
        "roll".into(),
        Value::from(-number("InitialRotation_1")?.to_radians()),
    );
    transform.insert("scale".into(), Value::from(scale));
    Ok(())
}
