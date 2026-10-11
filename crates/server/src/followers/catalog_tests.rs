use super::catalog::load_catalog;

#[test]
fn follower_catalog_requires_populated_205_spawns_and_an_authored_entrance() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE follower_profiles(entry INTEGER,name TEXT,class_id INTEGER,spec_id INTEGER,role INTEGER,attack_spell INTEGER,cooldown_spell INTEGER,heal_spell INTEGER,taunt_spell INTEGER,form_spell INTEGER,melee INTEGER);
        INSERT INTO follower_profiles VALUES(209057,'Captain Garrick',2,66,0,275779,31935,0,62124,0,1);
        CREATE TABLE follower_lfg_dungeons(id INTEGER,name TEXT,map_id INTEGER,difficulty_id INTEGER,tanks INTEGER,healers INTEGER,damage INTEGER);
        INSERT INTO follower_lfg_dungeons VALUES(2436,'Ruby Life Pools',2521,205,1,1,3),(2439,'Brackenhide Hollow',2520,205,1,1,3);
        CREATE TABLE tdb_world_safe_locs(ID INTEGER,MapID INTEGER,LocX REAL,LocY REAL,LocZ REAL,Facing REAL,Comment TEXT);
        INSERT INTO tdb_world_safe_locs VALUES(100030,2521,1386.2,-169.719,131.0,342.674,'10.x Dungeon - Ruby Life Pools - Entrance'),(100031,2520,0,0,0,0,'Entrance');
        CREATE TABLE content_creature(map INTEGER,spawnDifficulties TEXT);
        INSERT INTO content_creature VALUES(2521,'1,2,23,205'),(2520,'1205');").unwrap();
    let catalog = load_catalog(&db).unwrap();
    assert_eq!(catalog.profiles[0].entry, 209057);
    assert_eq!(catalog.profiles[0].spec_id, 66);
    assert_eq!(catalog.dungeons.len(), 1);
    assert!(catalog.follower_ids.contains(&2439));
    let ruby = catalog.dungeons.get(&2436).unwrap();
    assert_eq!(ruby.entrance.map_id, 2521);
    assert!((ruby.entrance.position.x - 1386.2).abs() < 0.001);
    assert!((ruby.entrance.position.z - 131.0).abs() < 0.001);
    assert!((ruby.entrance.orientation - 342.674_f32.to_radians()).abs() < 0.0001);
    db.execute("DELETE FROM content_creature WHERE map=2521", [])
        .unwrap();
    assert!(load_catalog(&db).unwrap().dungeons.is_empty());
}
