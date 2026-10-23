//! Catalog provenance follows the imported source, never a compiled build number.
use rusqlite::Connection;

pub(crate) fn validate_build(build: &str) -> Result<(), String> {
    let parts: Vec<_> = build.split('.').collect();
    if parts.len() != 4 || parts.iter().any(|part| part.parse::<u32>().is_err()) {
        return Err(format!("Invalid DB2 source build {build:?}"));
    }
    Ok(())
}

pub(crate) fn read_source_build(db: &Connection, table: &str) -> Result<String, String> {
    let mut query = db
        .prepare(&format!("SELECT DISTINCT build FROM {table}"))
        .map_err(|error| format!("{table} provenance: {error}"))?;
    let builds = query
        .query_map([], |row| row.get::<_, String>(0))
        .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
        .map_err(|error| format!("{table} provenance: {error}"))?;
    let [build] = builds.as_slice() else {
        return Err(format!(
            "{table} requires exactly one declared source build"
        ));
    };
    validate_build(build)?;
    Ok(build.clone())
}
