use super::*;
use crate::persistence::housing_store::BUILD_KEY;
use bevy::prelude::Vec3;
use rusqlite::{Connection, params};

fn import_fixture() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE housing_catalog_build (build_key TEXT PRIMARY KEY, build TEXT); CREATE TABLE housing_catalog_record (build_key TEXT, table_name TEXT, record_id INTEGER, fields_json TEXT);").unwrap();
    db.execute(
        "INSERT INTO housing_catalog_build VALUES (?, ?)",
        params![crate::persistence::housing_store::BUILD_KEY, "12.1.0.69933"],
    )
    .unwrap();
    db
}

#[test]
fn housing_catalog_reads_authored_neighborhood_and_cornerstone_without_default_grants() {
    let db = import_fixture();
    let neighborhood = serde_json::json!({"MapID":"2735", "EntryPosition_0":"3807.760009765625", "EntryPosition_1":"-160.427001953125", "EntryPosition_2":"194.11099243164062", "EntryRotation":"163.41299438476562"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodMap', 1, ?)",
        params![BUILD_KEY, neighborhood.to_string()],
    )
    .unwrap();
    let plot = serde_json::json!({"NeighborhoodMapID":"1", "PlotIndex":"0", "Name":"", "Cost":"10000000", "CornerstonePosition_0":"2942.743896484375", "CornerstonePosition_1":"-214.265625", "CornerstonePosition_2":"113.11284637451172", "TeleportPosition_0":"2939.34375", "TeleportPosition_1":"-212.25540161132812", "TeleportPosition_2":"113.32099914550781"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodPlot', 417, ?)",
        params![BUILD_KEY, plot.to_string()],
    )
    .unwrap();
    let catalog = HousingCatalog::read(&db).unwrap();
    let neighborhood = catalog.neighborhood(1).unwrap();
    assert_eq!(neighborhood.entry.map_id, 2735);
    assert_eq!(
        neighborhood.entry.position,
        Vec3::new(3807.76, -160.427, 194.111)
    );
    let plot = catalog.plot(417).unwrap();
    assert_eq!(plot.neighborhood_id, 1);
    assert_eq!(plot.cost, 10_000_000);
    assert_eq!(plot.cornerstone, [2942.744, -214.26563, 113.11285]);
    assert_eq!(plot.house_id, None);
    assert_eq!(plot.owner_character_id, None);
    assert!(catalog.neighborhood(4).is_err());
    db.execute("UPDATE housing_catalog_build SET build='wrong'", [])
        .unwrap();
    assert!(
        HousingCatalog::read(&db)
            .unwrap_err()
            .contains("12.1.0.69933")
    );
}
