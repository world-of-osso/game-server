use super::*;
use crate::persistence::housing_store::BUILD_KEY;
use bevy::prelude::Vec3;
use rusqlite::{Connection, params};

#[test]
fn db2_follow_install_housing_catalog_reads_the_declared_next_build_key() {
    let db = catalog_fixture();
    let key = "334a36cf5f3c32a5f7a167405487fa14";
    db.execute("UPDATE housing_catalog_build SET build_key=?, build='12.1.5.70391'", [key]).unwrap();
    db.execute("UPDATE housing_catalog_record SET build_key=?", [key]).unwrap();
    let catalog = HousingCatalog::read(&db).unwrap();
    assert_eq!(catalog.plot(417).unwrap().cost, 10_000_000);
    assert_eq!(catalog.cornerstone_plot(527268), Some(417));
}

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

pub(crate) fn catalog_fixture() -> Connection {
    let db = import_fixture();
    let neighborhood = serde_json::json!({"MapID":"2735", "EntryPosition_0":"3807.760009765625", "EntryPosition_1":"-160.427001953125", "EntryPosition_2":"194.11099243164062", "EntryRotation":"163.41299438476562"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodMap', 1, ?)",
        params![BUILD_KEY, neighborhood.to_string()],
    )
    .unwrap();
    let interior = serde_json::json!({"MapID":"2783", "EntryPosition_0":"-1000", "EntryPosition_1":"-1000", "EntryPosition_2":"0.1", "EntryRotation":"0"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodMap', 7, ?)",
        params![BUILD_KEY, interior.to_string()],
    )
    .unwrap();
    let plot = serde_json::json!({"NeighborhoodMapID":"1", "PlotIndex":"0", "Name":"", "Cost":"10000000", "CornerstonePosition_0":"2942.743896484375", "CornerstonePosition_1":"-214.265625", "CornerstonePosition_2":"113.11284637451172", "TeleportPosition_0":"2939.34375", "TeleportPosition_1":"-212.25540161132812", "TeleportPosition_2":"113.32099914550781", "CornerstoneGameObjectID":"527268"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'NeighborhoodPlot', 417, ?)",
        params![BUILD_KEY, plot.to_string()],
    )
    .unwrap();
    db.execute_batch("CREATE TABLE game_object_display_info (ID INTEGER, GeoBox_0 REAL, GeoBox_1 REAL, GeoBox_2 REAL, GeoBox_3 REAL, GeoBox_4 REAL, GeoBox_5 REAL); INSERT INTO game_object_display_info VALUES (110660,-2.6780343,-2.6780343,-0.5331883,2.6780343,2.6780343,4.1304092);").unwrap();
    let object = serde_json::json!({"OwnerID":"2735", "DisplayID":"110660", "Name_lang":"Cornerstone - Plot 0", "Pos_0":"2941.23169", "Pos_1":"-214.586807", "Pos_2":"113.156387", "Rot_0":"0", "Rot_1":"0", "Rot_2":"0.902585328", "Rot_3":"0.430510968", "Scale":"1", "TypeID":"48"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'GameObjects', 527268, ?)",
        params![BUILD_KEY, object.to_string()],
    )
    .unwrap();
    db
}

/// GameObjects 576398 "Founders Point Bulletin Board" exactly as authored in 12.1.0.69933.
pub(crate) fn add_bulletin_board(db: &Connection) {
    db.execute_batch("INSERT INTO game_object_display_info VALUES (47198,-0.27140423656,-1.76862227917,-0.1756734848,0.22231735289,1.76862227917,2.63712787628);").unwrap();
    let object = serde_json::json!({"OwnerID":"2735", "DisplayID":"47198", "Name_lang":"Founders Point Bulletin Board", "Pos_0":"3323.43579", "Pos_1":"75.2065964", "Pos_2":"101.290031", "Rot_0":"0", "Rot_1":"0", "Rot_2":"-0.237685874", "Rot_3":"0.971342087", "Scale":"1", "TypeID":"48", "PropValue_0":"4", "PropValue_1":"1", "PropValue_4":"10", "PropValue_7":"72"});
    db.execute(
        "INSERT INTO housing_catalog_record VALUES (?, 'GameObjects', 576398, ?)",
        params![BUILD_KEY, object.to_string()],
    )
    .unwrap();
}

#[test]
fn housing_catalog_cornerstones_keep_actual_object_transform_and_refuse_substitution() {
    let db = catalog_fixture();
    let catalog = HousingCatalog::read(&db).unwrap();
    let objects = catalog.cornerstones();
    assert_eq!(objects.len(), 1);
    let stone = &objects[0];
    assert_eq!(stone.info.entry, 527268);
    assert_eq!(stone.info.display_id, 110660);
    assert_eq!(stone.info.go_type, 48);
    assert_eq!(stone.info.name, "Cornerstone - Plot 0");
    assert_eq!(stone.position, Vec3::new(2941.2317, -214.5868, 113.15639));
    assert_ne!(
        stone.position,
        Vec3::from_array(catalog.plot(417).unwrap().cornerstone)
    );
    assert_eq!(catalog.cornerstone_plot(527268), Some(417));
    assert_eq!(catalog.cornerstone_plot(581971), None);
    db.execute(
        "DELETE FROM housing_catalog_record WHERE table_name='GameObjects'",
        [],
    )
    .unwrap();
    let unavailable = HousingCatalog::read(&db).unwrap();
    assert!(unavailable.cornerstones().is_empty());
    assert_eq!(unavailable.cornerstone_plot(527268), None);
    assert_eq!(unavailable.plots(1).unwrap().len(), 1);
}

#[test]
fn housing_catalog_reads_authored_neighborhood_and_cornerstone_without_default_grants() {
    let db = catalog_fixture();
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
            .contains("Invalid DB2 source build")
    );
}
