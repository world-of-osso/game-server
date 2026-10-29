use super::*;

#[test]
fn floor_resolver_missing_geometry_is_not_a_single_floor() {
    let dir = tempfile::tempdir().unwrap();
    let mut resolver = FloorResolver::new(dir.path(), dir.path(), HashMap::new());
    let request = Request {
        file: String::new(),
        run_start: String::new(),
        guid: String::new(),
        map: 2519,
        x: -250.0,
        y: 550.0,
        ui_map_id: None,
        neighbor_hints: Vec::new(),
    };
    assert!(matches!(
        resolver.resolve(&request),
        Resolution::Gap {
            gap: Gap::MissingGeometry { .. }
        }
    ));
}

#[test]
#[ignore = "requires local Neltharus geometry and navigation assets"]
fn floor_resolver_neltharus_real_data() {
    let geometry = Path::new("/home/osso/Projects/world-of-osso/game-engine/data");
    let nav = Path::new("/home/osso/Projects/world-of-osso/game-server/data/ground/navmesh");
    let db = rusqlite::Connection::open_with_flags(
        "/home/osso/Projects/world-of-osso/game-server/data/world.db",
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt: u32 = db
        .query_row("SELECT WdtFileDataID FROM map WHERE ID=2519", [], |row| {
            row.get(0)
        })
        .unwrap();
    let mut resolver = FloorResolver::new(geometry, nav, HashMap::from([(2519, wdt)]));
    let request = Request {
        file: String::new(),
        run_start: String::new(),
        guid: String::new(),
        map: 2519,
        x: -250.0,
        y: 550.0,
        ui_map_id: Some(1),
        neighbor_hints: Vec::new(),
    };
    println!("{:?}", resolver.resolve(&request));
}
