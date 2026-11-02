//! Catalog inputs for a data-derived follower dungeon graph.
use rusqlite::{Connection, OptionalExtension};

#[derive(Debug)]
pub(super) struct Source {
    pub difficulty: u32,
    pub entrance: [f32; 3],
    pub bosses: Vec<(u32, u32)>,
}

pub(super) fn read_source(
    db: &Connection,
    map: u32,
    requested_difficulty: Option<u32>,
) -> Result<Source, String> {
    let difficulty = match requested_difficulty {
        Some(difficulty) => difficulty,
        None => db
            .query_row(
                "SELECT difficulty_id FROM follower_lfg_dungeons WHERE map_id=?1 ORDER BY id LIMIT 1",
                [map],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("map{map} catalog: {error}"))?
            .ok_or_else(|| format!("map{map} missing follower catalog entry"))?,
    };
    if requested_difficulty.is_some() {
        let supported: bool = db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM map_difficulty WHERE MapID=?1 AND DifficultyID=?2)",
                (map, difficulty),
                |row| row.get(0),
            )
            .map_err(|error| format!("map{map} difficulty catalog: {error}"))?;
        if !supported {
            return Err(format!("map{map} does not support difficulty{difficulty}"));
        }
    }
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
    let mut query = db
        .prepare(
            "SELECT d.ID, d.Bit, d.Name_lang, b.creature_entry
        FROM dungeon_encounter d LEFT JOIN raid_encounter_creature b
          ON b.encounter_id=d.ID AND b.map_id=d.MapID AND b.bit=d.Bit
        WHERE d.MapID=?1 AND (d.DifficultyID<=0 OR d.DifficultyID=?2)
        ORDER BY d.OrderIndex, d.ID",
        )
        .map_err(|error| format!("map{map} encounter definitions: {error}"))?;
    let rows = query
        .query_map((map, difficulty), |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<u32>>(3)?,
            ))
        })
        .map_err(|error| format!("map{map} encounter definitions: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("map{map} encounter definitions: {error}"))?;
    if rows.is_empty() {
        return Err(format!("map{map} missing encounter definitions"));
    }
    let mut bosses = rows
        .into_iter()
        .map(|(id, bit, name, entry)| {
            if bit >= 32 {
                return Err(format!("map{map} encounter{id} has invalid bit{bit}"));
            }
            entry.map(|entry| (id, bit, entry)).ok_or_else(|| {
                format!("map{map} missing encounter binding for {name} (encounter{id}, bit{bit})")
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    order_encounter_members(map, difficulty, &mut bosses)?;
    Ok(Source {
        difficulty,
        entrance,
        bosses: bosses
            .into_iter()
            .map(|(_, bit, entry)| (bit, entry))
            .collect(),
    })
}

fn order_encounter_members(
    map: u32,
    difficulty: u32,
    bosses: &mut [(u32, u32, u32)],
) -> Result<(), String> {
    let mut start = 0;
    while start < bosses.len() {
        let encounter = bosses[start].0;
        let end = start
            + bosses[start..]
                .iter()
                .take_while(|row| row.0 == encounter)
                .count();
        let members = &mut bosses[start..end];
        if members.len() > 1 {
            let sequence = activation_sequence(map, encounter, difficulty)?;
            if sequence.len() != members.len()
                || members
                    .iter()
                    .any(|member| sequence.iter().filter(|entry| **entry == member.2).count() != 1)
            {
                return Err(format!(
                    "map{map} encounter{encounter} difficulty{difficulty} activation sequence does not match bindings"
                ));
            }
            members.sort_by_key(|member| {
                sequence
                    .iter()
                    .position(|entry| *entry == member.2)
                    .unwrap()
            });
        }
        start = end;
    }
    Ok(())
}

fn activation_sequence(map: u32, encounter: u32, difficulty: u32) -> Result<Vec<u32>, String> {
    let mut found = None;
    for line in include_str!("activation_order.csv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        let values = line
            .split(',')
            .map(str::parse::<u32>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("invalid encounter activation sequence: {error}"))?;
        let [
            source_map,
            source_encounter,
            source_difficulty,
            entries @ ..,
        ] = values.as_slice()
        else {
            return Err("invalid encounter activation sequence row".into());
        };
        if (*source_map, *source_encounter, *source_difficulty) == (map, encounter, difficulty) {
            if found.replace(entries.to_vec()).is_some() {
                return Err(format!(
                    "duplicate activation sequence for map{map} encounter{encounter} difficulty{difficulty}"
                ));
            }
        }
    }
    found.ok_or_else(|| format!("map{map} encounter{encounter} difficulty{difficulty} missing sourced activation sequence"))
}

#[test]
fn source_uses_sourced_council_activation_order() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE map_difficulty(MapID INTEGER, DifficultyID INTEGER);
         CREATE TABLE tdb_world_safe_locs(ID INTEGER, MapID INTEGER, LocX REAL, LocY REAL, LocZ REAL, Comment TEXT);
         CREATE TABLE content_creature(map INTEGER);
         CREATE TABLE raid_encounter_creature(map_id INTEGER, bit INTEGER, creature_entry INTEGER, encounter_id INTEGER);
         CREATE TABLE dungeon_encounter(ID INTEGER, MapID INTEGER, DifficultyID INTEGER, Bit INTEGER, OrderIndex INTEGER, Name_lang TEXT);
         INSERT INTO map_difficulty VALUES(1762,8);
         INSERT INTO tdb_world_safe_locs VALUES(1,1762,10,20,30,'Entrance');
         INSERT INTO content_creature VALUES(1762);
         INSERT INTO dungeon_encounter VALUES(2139,1762,0,0,0,'Serpent'),(2140,1762,0,1,1,'Council');
         INSERT INTO raid_encounter_creature VALUES(1762,0,135322,2139),(1762,1,135470,2140),(1762,1,135472,2140),(1762,1,135475,2140);",
    ).unwrap();
    assert_eq!(
        read_source(&db, 1762, Some(8)).unwrap().bosses,
        [(0, 135322), (1, 135475), (1, 135470), (1, 135472)]
    );
    db.execute(
        "DELETE FROM raid_encounter_creature WHERE creature_entry=135472",
        [],
    )
    .unwrap();
    assert!(
        read_source(&db, 1762, Some(8))
            .unwrap_err()
            .contains("activation sequence does not match bindings")
    );
    db.execute("UPDATE dungeon_encounter SET ID=9999 WHERE ID=2140", [])
        .unwrap();
    db.execute(
        "UPDATE raid_encounter_creature SET encounter_id=9999 WHERE encounter_id=2140",
        [],
    )
    .unwrap();
    assert!(
        read_source(&db, 1762, Some(8))
            .unwrap_err()
            .contains("missing sourced activation sequence")
    );
}

#[test]
fn source_requires_real_entrance_population_and_bindings() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE follower_lfg_dungeons(id INTEGER, map_id INTEGER, difficulty_id INTEGER);
        CREATE TABLE tdb_world_safe_locs(ID INTEGER, MapID INTEGER, LocX REAL, LocY REAL, LocZ REAL, Comment TEXT);
        CREATE TABLE content_creature(map INTEGER);
        CREATE TABLE raid_encounter_creature(map_id INTEGER, bit INTEGER, creature_entry INTEGER, encounter_id INTEGER);
        CREATE TABLE dungeon_encounter(ID INTEGER, MapID INTEGER, DifficultyID INTEGER, Bit INTEGER, OrderIndex INTEGER, Name_lang TEXT);
        INSERT INTO follower_lfg_dungeons VALUES(1,2451,205);
        INSERT INTO dungeon_encounter VALUES(10,2451,0,0,0,'First'),(11,2451,0,1,-1000,'Earlier'),(12,2451,0,2,1000,'Last');").unwrap();
    assert!(
        read_source(&db, 2451, None)
            .unwrap_err()
            .contains("missing entrance")
    );
    db.execute_batch("INSERT INTO tdb_world_safe_locs VALUES(1,2451,10,20,30,'Entrance');")
        .unwrap();
    assert!(
        read_source(&db, 2451, None)
            .unwrap_err()
            .contains("missing population")
    );
    db.execute_batch("INSERT INTO content_creature VALUES(2451);")
        .unwrap();
    assert!(
        read_source(&db, 2451, None)
            .unwrap_err()
            .contains("missing encounter binding for Earlier")
    );
    db.execute_batch(
        "INSERT INTO raid_encounter_creature VALUES(2451,0,184580,10), (2451,1,184018,11);",
    )
    .unwrap();
    assert!(
        read_source(&db, 2451, None)
            .unwrap_err()
            .contains("missing encounter binding for Last (encounter12, bit2)")
    );
    db.execute_batch("INSERT INTO raid_encounter_creature VALUES(2451,2,184124,12);")
        .unwrap();
    let source = read_source(&db, 2451, None).unwrap();
    assert_eq!(source.difficulty, 205);
    assert_eq!(source.entrance, [10.0, 20.0, 30.0]);
    assert_eq!(source.bosses, [(1, 184018), (0, 184580), (2, 184124)]);

    db.execute_batch(
        "CREATE TABLE map_difficulty(MapID INTEGER, DifficultyID INTEGER);
        INSERT INTO map_difficulty VALUES(2451,23);",
    )
    .unwrap();
    assert_eq!(read_source(&db, 2451, Some(23)).unwrap().difficulty, 23);
    db.execute("DELETE FROM follower_lfg_dungeons WHERE map_id=2451", [])
        .unwrap();
    assert_eq!(read_source(&db, 2451, Some(23)).unwrap().difficulty, 23);
    assert!(
        read_source(&db, 2451, None)
            .unwrap_err()
            .contains("missing follower catalog entry")
    );
    assert!(
        read_source(&db, 2451, Some(99))
            .unwrap_err()
            .contains("does not support difficulty99")
    );
}
