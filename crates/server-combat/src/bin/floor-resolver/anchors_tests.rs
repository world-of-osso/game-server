use super::*;

#[test]
fn authored_rows_exclude_log_and_journal_and_enforce_radius() {
    let db = Connection::open_in_memory().unwrap();
    for table in ["tdb_creature", "content_creature", "tdb_gameobject"] {
        db.execute(&format!("CREATE TABLE {table} (guid INTEGER, map INTEGER, position_x REAL, position_y REAL, position_z REAL, Comment TEXT)"), []).unwrap();
    }
    db.execute_batch("INSERT INTO tdb_creature VALUES (1,2519,0,0,133,NULL); INSERT INTO content_creature VALUES (2,2519,1,0,133,'LOG_DERIVED'); INSERT INTO content_creature VALUES (3,2519,2,0,133,'journal marker'); INSERT INTO content_creature VALUES (4,2519,3,0,133,NULL); INSERT INTO tdb_gameobject VALUES (5,2519,16,0,133,NULL);").unwrap();
    let request: Request =
        serde_json::from_str(r#"{"map":2519,"x":0,"y":0,"ui_map_id":1}"#).unwrap();
    let rows = AnchorIndex::read(&db, std::slice::from_ref(&request))
        .unwrap()
        .near(&request);
    let mut ids: Vec<_> = rows.iter().map(|row| row.row.guid).collect();
    ids.sort();
    assert_eq!(ids, vec![1, 4]);
}
