//! Real Retail terrain versus legacy AzerothCore STV audit destinations.
use std::path::PathBuf;
use std::time::Duration;

use bevy::ecs::system::{RunSystemOnce, SystemState};
use shared::components::{Health, MovementControl, Player, Position, VerticalVelocity};

use super::*;
use crate::networking::FallTracker;
use crate::persistence;

fn client_data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
}

fn stv_ground() -> GroundMap {
    GroundMap::new(&client_data(), HashMap::from([(0, 775_971)]))
}

/// The achievements audit used legacy creature guid 532's Z (11.0223),
/// offset three yards east. Retail ADT 778052 has ground at 16.23857 there.
/// Do not make underground terrain reachable to repair a wrong destination.
#[test]
fn stv_audit_teleport_requires_retail_ground_height() {
    let mut ground = stv_ground();
    let xy = (-11625.6, -54.5569);
    let feet = |z| Vec3::new(xy.0, z, -xy.1);
    assert_eq!(bevy_to_tile(xy.0, -xy.1), (32, 53));
    assert_eq!(ground.ground_at(0, feet(11.0223)), GroundProbe::Unsupported);
    let GroundProbe::Supported(floor) = ground.ground_at(0, feet(16.23857)) else {
        panic!("Retail STV tile has no reachable ground");
    };
    assert!((floor - 16.23857).abs() < 0.001, "floor {floor}");

    for (destination, supported) in [(11.0223, false), (16.23857, true)] {
        let db = persistence::temp_database();
        let (id, _) = persistence::create_character(&db, 1, "Stvprobe", 1, 1).unwrap();
        let game_db = persistence::GameDatabase::new(db);
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_message::<crate::death::PlayerDeathTransition>()
            .insert_resource(stv_ground());
        app.update();
        let player = app
            .world_mut()
            .spawn((
                Position {
                    x: 0.0,
                    y: 98.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                VerticalVelocity::default(),
                FallTracker::grounded(98.0),
                crate::maps::InMap::continent(0),
                Player {
                    name: "Stvprobe".into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                persistence::ClientId(id),
            ))
            .id();
        let mut admin = SystemState::<
            Query<(
                Entity,
                &mut Position,
                &persistence::ClientId,
                &mut MovementControl,
            )>,
        >::new(app.world_mut());
        let mut query = admin.get_mut(app.world_mut()).unwrap();
        crate::admin_handlers::handle_set_position(
            &game_db,
            &mut query,
            None,
            None,
            "Stvprobe",
            xy.0,
            xy.1,
            destination,
        );
        admin.apply(app.world_mut());
        for _ in 0..200 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            app.world_mut()
                .run_system_once(crate::networking::apply_terrain_gravity)
                .unwrap();
            if supported {
                assert!((app.world().get::<Position>(player).unwrap().y - floor).abs() < 0.001);
                assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
            }
        }
        if !supported {
            assert!(app.world().get::<Position>(player).unwrap().y < -50.0);
        }
    }
}

#[test]
fn stv_retail_hubs_have_reachable_ground() {
    let mut ground = stv_ground();
    // Runtime content_creature Hemet 715, then Retail taxi_nodes destinations.
    for (name, x, y, z) in [
        ("Hemet Jr", -11653.4, -61.026, 16.313),
        ("Booty Bay east", -14271.8, 299.87, 31.09),
        ("Booty Bay Alliance", -14444.3, 509.62, 26.2),
        ("Booty Bay Horde", -14473.0, 464.15, 36.43),
        ("Grom'gol", -12414.2, 146.29, 3.28),
        ("Rebel Camp", -11344.0, -216.83, 75.22),
        ("Fort Livingston", -12828.8, -413.71, 12.9666),
        ("Explorers' League", -13600.9, -85.5486, 37.8078),
        ("Hardwrench", -13288.3, 722.028, 3.75389),
        ("Bambala", -12092.2, -814.099, 42.3411),
    ] {
        let probe = ground.ground_at(0, Vec3::new(x, z, -y));
        println!(
            "{name}: tile {:?}, destination {z}, probe {probe:?}",
            bevy_to_tile(x, -y)
        );
        assert!(
            matches!(probe, GroundProbe::Supported(_)),
            "{name}: {probe:?}"
        );
    }
}

/// Explicit diagnostic, not normal-suite work: every Map.db2 row the server loads,
/// including unvisited continents. Uses the production root ADT/liquid parser.
#[test]
#[ignore = "all-map asset census; set STV_GROUND_CENSUS to the output JSON path"]
fn census_all_map_wdt_root_tiles() {
    let output = std::env::var("STV_GROUND_CENSUS").expect("STV_GROUND_CENSUS output path");
    let db = rusqlite::Connection::open_with_flags(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut statement = db
        .prepare("SELECT ID, Directory, WdtFileDataID FROM map ORDER BY ID")
        .unwrap();
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })
        .unwrap();
    let data = client_data();
    let reports: Vec<_> = rows
        .map(|row| {
            let (map, directory, wdt) = row.unwrap();
            scan_map_root_tiles(&data, map, &directory, wdt)
        })
        .collect();
    std::fs::write(output, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    println!("{} Map.db2 rows scanned", reports.len());
}

fn read_wdt_root_tiles(data: &Path, wdt: u32) -> Result<Option<Vec<TileFiles>>, String> {
    let bytes = read_client_file(data, "terrain", wdt, "wdt")?;
    let mphd = find_chunk(&bytes, b"MPHD")?.ok_or("WDT has no MPHD")?;
    if read_u32(mphd, 0)? & MPHD_FLAG_GLOBAL_WMO != 0 {
        return Ok(None);
    }
    parse_maid(&bytes).map(Some)
}

fn scan_map_root_tiles(data: &Path, map: u32, directory: &str, wdt: u32) -> serde_json::Value {
    let mut report = serde_json::json!({"map":map,"directory":directory,"wdt":wdt,"tiles":0,"missing":[],"unparseable":[]});
    match read_wdt_root_tiles(data, wdt) {
        Err(error) => report["wdt_error"] = error.into(),
        Ok(None) => report["global_wmo"] = true.into(),
        Ok(Some(tiles)) => scan_root_tile_files(data, &tiles, &mut report),
    }
    report
}

fn scan_root_tile_files(data: &Path, tiles: &[TileFiles], report: &mut serde_json::Value) {
    for (index, tile) in tiles.iter().enumerate().filter(|(_, tile)| tile.root != 0) {
        report["tiles"] = (report["tiles"].as_u64().unwrap() + 1).into();
        let path = data.join(format!("terrain/{}.adt", tile.root));
        let category = if path.is_file() {
            "unparseable"
        } else {
            "missing"
        };
        if let Err(error) = load_terrain(data, tile.root) {
            report[category]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({
                    "row":index % MAP_TILES,"col":index / MAP_TILES,"fdid":tile.root,"error":error
                }));
        }
    }
}
