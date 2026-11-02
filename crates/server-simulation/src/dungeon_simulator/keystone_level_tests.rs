//! Keystone actors use the Mythic map tuning, not follower-dungeon tuning.
use super::read_mythic_level;
use rusqlite::{Connection, OpenFlags};

#[test]
fn mythic_copy_level_uses_its_own_content_tuning() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE map_difficulty(MapID INTEGER, DifficultyID INTEGER, ContentTuningID INTEGER);
         CREATE TABLE content_tuning(ID INTEGER, MaxLevelSquish INTEGER);
         INSERT INTO content_tuning VALUES (2845, 70), (3025, 90);
         INSERT INTO map_difficulty VALUES (2521, 205, 2845), (2521, 23, 3025);",
    )
    .unwrap();
    assert_eq!(read_mythic_level(&db, 2521).unwrap(), 90);
    assert!(read_mythic_level(&db, 9999).is_err());
}

#[test]
fn ruby_mythic_copy_uses_level_90_from_native_catalog() {
    let path = crate::test_data::world_db_path();
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(read_mythic_level(&db, 2521).unwrap(), 90);
}
