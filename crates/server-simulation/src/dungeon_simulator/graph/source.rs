//! Catalog inputs for a data-derived follower dungeon graph.
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug)]
pub(super) struct Source {
    pub difficulty: u32,
    pub entrance: [f32; 3],
    pub bosses: Vec<(u32, u32)>,
}

pub(super) fn read_source(db: &Connection, map: u32) -> Result<Source, String> {
    let difficulty = db
        .query_row(
            "SELECT difficulty_id FROM follower_lfg_dungeons WHERE map_id=?1 ORDER BY id LIMIT 1",
            [map],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("map{map} catalog: {error}"))?
        .ok_or_else(|| format!("map{map} missing follower catalog entry"))?;
    let mut entrances = db.prepare(
        "SELECT LocX, LocY, LocZ FROM tdb_world_safe_locs WHERE MapID=?1 AND lower(Comment) LIKE '%entrance%' ORDER BY ID",
    ).map_err(|error| format!("map{map} entrances: {error}"))?;
    let points = entrances
        .query_map([map], |row| Ok([row.get(0)?, row.get(1)?, row.get(2)?]))
        .map_err(|error| format!("map{map} entrances: {error}"))?
        .collect::<Result<Vec<[f32; 3]>, _>>()
        .map_err(|error| format!("map{map} entrances: {error}"))?;
    let entrance = match points.as_slice() {
        [] => return Err(format!("map{map} missing entrance")),
        [point] if point.iter().all(|v| v.is_finite()) => *point,
        _ => return Err(format!("map{map} ambiguous or invalid entrance")),
    };
    let count: u32 = db
        .query_row(
            "SELECT count(*) FROM content_creature WHERE map=?1",
            [map],
            |row| row.get(0),
        )
        .map_err(|error| format!("map{map} population: {error}"))?;
    if count == 0 {
        return Err(format!("map{map} missing population"));
    }
    let mut query = db.prepare("SELECT bit, creature_entry FROM raid_encounter_creature WHERE map_id=?1 ORDER BY bit, creature_entry")
        .map_err(|error| format!("map{map} bindings: {error}"))?;
    let bosses = query
        .query_map([map], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|error| format!("map{map} bindings: {error}"))?
        .collect::<Result<Vec<(u32, u32)>, _>>()
        .map_err(|error| format!("map{map} bindings: {error}"))?;
    if bosses.is_empty() {
        return Err(format!("map{map} missing encounter bindings"));
    }
    let bits: std::collections::BTreeSet<_> = bosses.iter().map(|(bit, _)| *bit).collect();
    if bits.iter().copied().ne(0..bits.len() as u32) {
        return Err(format!("map{map} missing encounter binding bits: {bits:?}"));
    }
    Ok(Source {
        difficulty,
        entrance,
        bosses,
    })
}

#[test]
fn source_requires_real_entrance_population_and_bindings() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE follower_lfg_dungeons(id INTEGER, map_id INTEGER, difficulty_id INTEGER);
        CREATE TABLE tdb_world_safe_locs(ID INTEGER, MapID INTEGER, LocX REAL, LocY REAL, LocZ REAL, Comment TEXT);
        CREATE TABLE content_creature(map INTEGER);
        CREATE TABLE raid_encounter_creature(map_id INTEGER, bit INTEGER, creature_entry INTEGER);
        INSERT INTO follower_lfg_dungeons VALUES(1,2451,205);").unwrap();
    assert!(
        read_source(&db, 2451)
            .unwrap_err()
            .contains("missing entrance")
    );
    db.execute_batch("INSERT INTO tdb_world_safe_locs VALUES(1,2451,10,20,30,'Entrance');")
        .unwrap();
    assert!(
        read_source(&db, 2451)
            .unwrap_err()
            .contains("missing population")
    );
    db.execute_batch("INSERT INTO content_creature VALUES(2451);")
        .unwrap();
    assert!(
        read_source(&db, 2451)
            .unwrap_err()
            .contains("missing encounter bindings")
    );
    db.execute_batch("INSERT INTO raid_encounter_creature VALUES(2451,0,184580), (2451,2,184124);")
        .unwrap();
    assert!(
        read_source(&db, 2451)
            .unwrap_err()
            .contains("missing encounter binding bits")
    );
    db.execute_batch("INSERT INTO raid_encounter_creature VALUES(2451,1,184018);")
        .unwrap();
    let source = read_source(&db, 2451).unwrap();
    assert_eq!(source.difficulty, 205);
    assert_eq!(source.entrance, [10.0, 20.0, 30.0]);
    assert_eq!(source.bosses, [(0, 184580), (1, 184018), (2, 184124)]);
}
