use super::catalog::load_catalog;

#[test]
fn follower_population_catalog_lists_every_map_with_authentic_sources() {
    use std::collections::BTreeSet;

    let source = crate::test_data::world_db_path();
    let temporary = tempfile::tempdir().unwrap();
    let copy = temporary.path().join("follower-population.sqlite");
    std::fs::copy(&source, &copy).unwrap();
    let db = rusqlite::Connection::open(&copy).unwrap();
    let expected: BTreeSet<u32> = db
        .prepare(
            "SELECT DISTINCT d.map_id FROM follower_lfg_dungeons d
            JOIN tdb_world_safe_locs l ON l.MapID=d.map_id AND l.Comment LIKE '%Entrance%'
            JOIN tdb_creature c ON c.map=d.map_id
            WHERE d.difficulty_id=205 AND d.tanks=1 AND d.healers=1 AND d.damage=3
              AND (','||replace(c.spawnDifficulties,' ','')||',') LIKE '%,1,%'
              AND c.terrainSwapMap=-1 AND (c.phaseUseFlags=1 OR
                  (c.phaseUseFlags=0 AND c.PhaseId IN (0,169) AND c.PhaseGroup=0))",
        )
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(expected, BTreeSet::from([2451, 2515, 2521, 2526, 2648]));
    db.execute_batch(
        "DELETE FROM content_creature WHERE map IN
            (SELECT map_id FROM follower_lfg_dungeons WHERE difficulty_id=205);
        DELETE FROM tdb_world_safe_locs WHERE MapID IN
            (SELECT map_id FROM follower_lfg_dungeons WHERE difficulty_id=205);",
    )
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source_path = source.canonicalize().unwrap();
    let db2 = source_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("db2/12.1.5.70448");
    let output = std::process::Command::new("python3")
        .arg(root.join("scripts/follower_population.py"))
        .arg("--source-db")
        .arg(&source)
        .arg("--db")
        .arg(&copy)
        .arg("--db2-dir")
        .arg(db2)
        .arg("--report")
        .arg(temporary.path().join("population.json"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "producer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: BTreeSet<_> = load_catalog(&db)
        .unwrap()
        .dungeons
        .values()
        .map(|dungeon| dungeon.entrance.map_id)
        .collect();
    assert_eq!(
        actual, expected,
        "every source-backed map must be queueable"
    );
}

#[test]
fn follower_normal_population_requires_exact_normal_spawns_and_an_authored_entrance() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE follower_profiles(entry INTEGER,name TEXT,class_id INTEGER,spec_id INTEGER,role INTEGER,attack_spell INTEGER,cooldown_spell INTEGER,heal_spell INTEGER,taunt_spell INTEGER,form_spell INTEGER,melee INTEGER);
        INSERT INTO follower_profiles VALUES(209057,'Captain Garrick',2,66,0,275779,31935,0,62124,0,1);
        CREATE TABLE follower_lfg_dungeons(id INTEGER,name TEXT,map_id INTEGER,difficulty_id INTEGER,tanks INTEGER,healers INTEGER,damage INTEGER);
        INSERT INTO follower_lfg_dungeons VALUES(2436,'Ruby Life Pools',2521,205,1,1,3),(2439,'Brackenhide Hollow',2520,205,1,1,3);
        CREATE TABLE tdb_world_safe_locs(ID INTEGER,MapID INTEGER,LocX REAL,LocY REAL,LocZ REAL,Facing REAL,Comment TEXT);
        INSERT INTO tdb_world_safe_locs VALUES(100030,2521,1386.2,-169.719,131.0,342.674,'10.x Dungeon - Ruby Life Pools - Entrance'),(100031,2520,0,0,0,0,'Entrance');
        CREATE TABLE content_creature(map INTEGER,spawnDifficulties TEXT);
        INSERT INTO content_creature VALUES(2521,'1,2,23'),(2520,'11,205');").unwrap();
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
