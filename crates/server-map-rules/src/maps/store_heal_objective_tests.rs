use super::*;

#[test]
fn heal_objective_loads_only_with_matching_db2_encounter() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE encounter_heal_objective (map_id INTEGER, creature_entry INTEGER, start_health_pct REAL); \
        CREATE TABLE raid_encounter_creature (map_id INTEGER, creature_entry INTEGER, encounter_id INTEGER, bit INTEGER); \
        CREATE TABLE dungeon_encounter (ID INTEGER, MapID INTEGER, Bit INTEGER); \
        INSERT INTO encounter_heal_objective VALUES (1877,133392,25); \
        INSERT INTO raid_encounter_creature VALUES (1877,133392,2127,3); \
        INSERT INTO dungeon_encounter VALUES (2127,1877,3);").unwrap();
    assert_eq!(
        load_encounter_heal_objectives(&db)
            .unwrap()
            .get(&(1877, 133392)),
        Some(&25.0)
    );
    db.execute("UPDATE dungeon_encounter SET Bit=2 WHERE ID=2127", [])
        .unwrap();
    assert!(
        load_encounter_heal_objectives(&db).is_err(),
        "a DB2 bit mismatch must not silently discard the objective"
    );
}
