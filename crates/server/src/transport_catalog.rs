//! Authored Seething airship templates and constant-speed TaxiPath routes.
use bevy::prelude::Resource;
use rusqlite::Connection;
use shared::{protocol::GameObjectInfo, transport_route::TransportRoute};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct ShipSpec {
    pub spawn_id: u32,
    pub map_id: u32,
    pub interior_map: u32,
    pub path_id: u32,
    pub speed: f32,
    pub knots: Vec<[f32; 3]>,
    pub info: GameObjectInfo,
    pub route: TransportRoute,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct TransportCatalog(pub BTreeMap<u32, ShipSpec>);

impl TransportCatalog {
    pub fn load(conn: &Connection) -> Result<Self, String> {
        let mut statement = conn
            .prepare(
                "SELECT guid,entry FROM tdb_transports WHERE ScriptName='transport_seething_shore'",
            )
            .map_err(|error| format!("read Seething transports: {error}"))?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?)))
            .map_err(|error| format!("read Seething transport identities: {error}"))?;
        let mut ships = BTreeMap::new();
        for row in rows {
            let (spawn_id, entry) = row.map_err(|error| error.to_string())?;
            let ship = load_ship(conn, spawn_id, entry)?;
            ships.insert(spawn_id, ship);
        }
        if ships.is_empty() {
            return Err("no authored Seething transports; import TDB transport metadata".into());
        }
        Ok(Self(ships))
    }
}

fn load_ship(conn: &Connection, spawn_id: u32, entry: u32) -> Result<ShipSpec, String> {
    let (info, path_id, speed, interior_map) = conn.query_row(
        "SELECT type,displayId,name,size,Data0,Data1,Data6 FROM tdb_gameobject_template WHERE entry=?",
        [entry],
        |row| Ok((GameObjectInfo {
            entry, go_type: row.get(0)?, display_id: row.get(1)?, name: row.get(2)?, scale: row.get(3)?,
        }, row.get::<_,u32>(4)?, row.get::<_,f32>(5)?, row.get::<_,u32>(6)?)),
    ).map_err(|error| format!("transport {spawn_id} template {entry}: {error}"))?;
    if info.go_type != 15 {
        return Err(format!(
            "transport {spawn_id} template {entry} is not a map-object transport"
        ));
    }
    let (map_id, knots) = load_route_knots(conn, path_id)?;
    let route = TransportRoute::new(knots.clone(), speed)
        .map_err(|error| format!("transport {spawn_id} path {path_id}: {error:?}"))?;
    Ok(ShipSpec {
        spawn_id,
        map_id,
        interior_map,
        path_id,
        speed,
        knots,
        info,
        route,
    })
}

fn load_route_knots(conn: &Connection, path_id: u32) -> Result<(u32, Vec<[f32; 3]>), String> {
    let mut statement = conn.prepare(
        "SELECT ContinentID,Loc_0,Loc_1,Loc_2,Flags,Delay,ArrivalEventID,DepartureEventID FROM taxi_path_node WHERE PathID=? ORDER BY NodeIndex",
    ).map_err(|error| format!("transport path {path_id}: {error}"))?;
    let rows = statement
        .query_map([path_id], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                [row.get::<_, f32>(1)?, row.get(2)?, row.get(3)?],
                [row.get::<_, u32>(4)?, row.get(5)?, row.get(6)?, row.get(7)?],
            ))
        })
        .map_err(|error| format!("transport path {path_id}: {error}"))?;
    let mut map = None;
    let mut knots = Vec::new();
    for row in rows {
        let (map_id, position, behavior) = row.map_err(|error| error.to_string())?;
        if behavior != [0; 4] {
            return Err(format!(
                "transport path {path_id} has unsupported flags/delay/events {behavior:?}"
            ));
        }
        if map.is_some_and(|previous| previous != map_id) {
            return Err(format!("transport path {path_id} spans multiple maps"));
        }
        map = Some(map_id);
        knots.push(position);
    }
    let map = map.ok_or_else(|| format!("transport path {path_id} has no nodes"))?;
    Ok((map, knots))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::transport_route::TransportPose;

    fn source_fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE tdb_transports(guid INTEGER,entry INTEGER,name TEXT,ScriptName TEXT);
            CREATE TABLE tdb_gameobject_template(entry INTEGER,type INTEGER,displayId INTEGER,name TEXT,size REAL,Data0 INTEGER,Data1 INTEGER,Data6 INTEGER);
            CREATE TABLE taxi_path_node(PathID INTEGER,NodeIndex INTEGER,ContinentID INTEGER,Loc_0 REAL,Loc_1 REAL,Loc_2 REAL,Flags INTEGER,Delay INTEGER,ArrivalEventID INTEGER,DepartureEventID INTEGER);
            INSERT INTO tdb_transports VALUES (35,278407,'Sword of Dawn','transport_seething_shore'),(36,279254,'The warbringer','transport_seething_shore');
            INSERT INTO tdb_gameobject_template VALUES (278407,15,9002,'Sword of Dawn',1,6519,4,1856),(279254,15,9001,'The Warbringer',1,6520,4,1857);")
            .unwrap();
        let source = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut statement = source.prepare("SELECT PathID,NodeIndex,ContinentID,Loc_0,Loc_1,Loc_2,Flags,Delay,ArrivalEventID,DepartureEventID FROM taxi_path_node WHERE PathID IN (6519,6520) ORDER BY PathID,NodeIndex").unwrap();
        let rows = statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, u32>(0)?,
                    r.get::<_, u32>(1)?,
                    r.get::<_, u32>(2)?,
                    r.get::<_, f32>(3)?,
                    r.get::<_, f32>(4)?,
                    r.get::<_, f32>(5)?,
                    r.get::<_, u32>(6)?,
                    r.get::<_, u32>(7)?,
                    r.get::<_, u32>(8)?,
                    r.get::<_, u32>(9)?,
                ))
            })
            .unwrap();
        for row in rows {
            let (path, index, map, x, y, z, flags, delay, arrival, departure) = row.unwrap();
            conn.execute(
                "INSERT INTO taxi_path_node VALUES (?,?,?,?,?,?,?,?,?,?)",
                rusqlite::params![path, index, map, x, y, z, flags, delay, arrival, departure],
            )
            .unwrap();
        }
        conn
    }

    #[test]
    fn source_ships_use_real_speed_routes_and_transport_interior_maps() {
        let conn = source_fixture();
        let catalog = TransportCatalog::load(&conn).unwrap();
        assert_eq!(catalog.0.len(), 2);
        for (id, entry, display, path, interior) in [
            (35, 278407, 9002, 6519, 1856),
            (36, 279254, 9001, 6520, 1857),
        ] {
            let ship = &catalog.0[&id];
            assert_eq!(
                (ship.spawn_id, ship.info.entry, ship.info.display_id),
                (id, entry, display)
            );
            assert_eq!(
                (ship.map_id, ship.interior_map, ship.path_id),
                (1803, interior, path)
            );
            assert_eq!(ship.speed, 4.0);
            assert_eq!(ship.knots.len(), 15);
            let route = TransportRoute::new(ship.knots.clone(), 4.0).unwrap();
            assert_eq!(ship.route.period_ms(), route.period_ms());
            assert_eq!(ship.route.pose_at(0).position, ship.knots[1]);
            assert_eq!(
                ship.route.pose_at(u64::from(ship.route.period_ms())),
                ship.route.pose_at(0)
            );
        }
    }

    #[test]
    fn authored_start_offsets_follow_the_ship_while_preserving_local_pose() {
        let catalog = TransportCatalog::load(&source_fixture()).unwrap();
        let local = TransportPose {
            position: [-29.8623, 0.520232, 20.5534],
            orientation: 0.0,
        };
        let ship = &catalog.0[&35];
        let first = ship.route.pose_at(0);
        let later = ship.route.pose_at(30_000);
        let at_first = first.local_to_world(local);
        let at_later = later.local_to_world(local);
        assert!((at_first.position[2] - 317.64004).abs() < 0.001);
        assert!((at_first.position[0] - at_later.position[0]).abs() > 1.0);
        let returned = later.world_to_local(at_later);
        for axis in 0..3 {
            assert!((returned.position[axis] - local.position[axis]).abs() < 0.001);
        }
    }

    #[test]
    fn unsupported_path_flags_fail_instead_of_substituting_a_ground_entry() {
        let conn = source_fixture();
        conn.execute(
            "UPDATE taxi_path_node SET Flags=1 WHERE PathID=6519 AND NodeIndex=1",
            [],
        )
        .unwrap();
        let error = TransportCatalog::load(&conn).unwrap_err();
        assert!(error.contains("6519") && error.contains("flags"), "{error}");
    }
}
