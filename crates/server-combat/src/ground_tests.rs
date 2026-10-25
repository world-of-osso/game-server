use std::path::PathBuf;
use std::time::Duration;

use bevy::ecs::system::{RunSystemOnce, SystemState};
use shared::components::{Health, MovementControl, Player, Position, VerticalVelocity};

use super::*;
use crate::networking::FallTracker;
use crate::persistence;

/// Inside the Stormwind auction house (WoW -8820, 660): its floor is at 98.06
/// over terrain at 94.65 (game-engine docs/wiki/investigations/stormwind-hilly-plaza.md).
const AUCTION_HOUSE_WOW: (f32, f32) = (-8820.0, 660.0);
const AUCTION_HOUSE_FLOOR: f32 = 98.06;

fn client_data_dir() -> PathBuf {
    std::env::var_os("GAME_SERVER_GROUND_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
        })
}

/// `world/maps/azeroth/azeroth.wdt`.
const AZEROTH_WDT_FDID: u32 = 775_971;

#[test]
fn silvershard_track_switch_approach_uses_real_floor_not_npc_pivot() {
    let mut ground = GroundMap::new(&client_data_dir(), HashMap::from([(727, 810604)]));
    // Source4000143's exact pivot has no walkable WMO face at its authored height.
    // The neighboring collision floor is a valid player approach within interaction range.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(715.642, 100.165, 320.284);
    assert_eq!(
        ground.ground_at(727, Vec3::new(x, y, z)),
        GroundProbe::Supported(0.0)
    );
    let (x, y, z) = crate::terrain_height::wow_to_bevy(719.642, 100.165, 321.0);
    let GroundProbe::Supported(height) = ground.ground_at(727, Vec3::new(x, y, z)) else {
        panic!("missing approach floor beside source Track Switch");
    };
    assert!((height - 320.82416).abs() < 0.01, "approach floor {height}");
}

fn probe_cart_approach(ground: &mut GroundMap, map: u32, center: Vec3) -> Option<[f32; 3]> {
    // Player feet alongside a cart, not its model pivot or the center of its rail.
    for radius in [1.0, 2.0, 4.0, 8.0, 12.0] {
        for (dx, dy) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
        ] {
            let x = center.x + radius * dx;
            let y = center.y + radius * dy;
            let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, center.z + 1.0);
            let GroundProbe::Supported(height) = ground.ground_at(map, Vec3::new(bx, by, bz))
            else {
                continue;
            };
            if height < center.z - 3.0 {
                continue;
            }
            return Some([x, y, height + 0.1]);
        }
    }
    None
}

fn probe_object_approach(
    ground: &mut GroundMap,
    map: u32,
    center: Vec3,
    bounds: crate::game_objects::GameObjectBounds,
) -> Option<[f32; 3]> {
    for radius in [0.0, 0.5, 1.0, 2.0, 4.0] {
        for (dx, dy) in [
            (1.0, 0.0),
            (-1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (1.0, 1.0),
            (-1.0, 1.0),
            (1.0, -1.0),
            (-1.0, -1.0),
        ] {
            let x = center.x + radius * dx;
            let y = center.y + radius * dy;
            let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, center.z + 1.0);
            let GroundProbe::Supported(height) = ground.ground_at(map, Vec3::new(bx, by, bz))
            else {
                continue;
            };
            let approach = Vec3::new(x, y, height + 0.1);
            if height >= center.z - 3.0
                && bounds.reaches(
                    center,
                    approach,
                    crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
                )
            {
                return Some(approach.to_array());
            }
        }
    }
    None
}

#[test]
fn alterac_source_starts_banners_and_leaders_have_real_walkable_approaches() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut ground = GroundMap::new(&client_data_dir(), HashMap::from([(30, 790112)]));
    let mut starts = db.prepare("SELECT ID,LocX,LocY,LocZ,TransportSpawnId FROM tdb_world_safe_locs WHERE ID IN (611,610) ORDER BY ID").unwrap();
    let mut start_approaches = Vec::new();
    for row in starts
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
                row.get::<_, Option<u32>>(4)?,
            ))
        })
        .unwrap()
    {
        let (id, at, transport) = row.unwrap();
        assert_eq!(transport, None);
        let (x, y, z) = crate::terrain_height::wow_to_bevy(at.x, at.y, at.z);
        let GroundProbe::Supported(height) = ground.ground_at(30, Vec3::new(x, y, z)) else {
            panic!("Alterac source start{id} has missing ground");
        };
        assert!(
            (height - at.z).abs() < 1.5,
            "Alterac source start{id}: floor{height}, start{at:?}"
        );
        start_approaches.push(serde_json::json!({"id":id,"source":at.to_array(),"floor":height}));
    }
    assert_eq!(start_approaches.len(), 2);
    let mut banners = db.prepare("SELECT g.id,g.guid,g.position_x,g.position_y,g.position_z,g.orientation,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5 FROM tdb_gameobject g JOIN tdb_gameobject_template t ON t.entry=g.id JOIN game_object_display_info d ON d.ID=t.displayId WHERE g.map=30 AND t.ScriptName IN ('go_av_capturable_object','go_av_contested_object') ORDER BY g.id").unwrap();
    let rows = banners
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u64>(1)?,
                Vec3::new(row.get(2)?, row.get(3)?, row.get(4)?),
                crate::game_objects::GameObjectBounds {
                    min: Vec3::new(row.get(7)?, row.get(8)?, row.get(9)?),
                    max: Vec3::new(row.get(10)?, row.get(11)?, row.get(12)?),
                    scale: row.get(6)?,
                    orientation: row.get(5)?,
                },
            ))
        })
        .unwrap();
    let mut banner_approaches = Vec::new();
    for row in rows {
        let (entry, guid, center, bounds) = row.unwrap();
        let approach = probe_object_approach(&mut ground,30,center,bounds).unwrap_or_else(|| panic!("Alterac source banner{entry}/{guid} has no real floor/reach at{center:?}; missing{:?}",ground.missing_files(30).into_iter().take(12).collect::<Vec<_>>()));
        assert!(bounds.reaches(
            center,
            Vec3::from_array(approach),
            crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE
        ));
        banner_approaches.push(serde_json::json!({"entry":entry,"guid":guid,"source":center.to_array(),"approach":approach}));
    }
    assert_eq!(banner_approaches.len(), 45);
    let mut leaders = db.prepare("SELECT id,guid,position_x,position_y,position_z FROM tdb_creature WHERE map=30 AND id IN (11946,11947,11948,11949) ORDER BY id").unwrap();
    let mut leader_approaches = Vec::new();
    for row in leaders
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, u64>(1)?,
                Vec3::new(row.get(2)?, row.get(3)?, row.get(4)?),
            ))
        })
        .unwrap()
    {
        let (entry, guid, center) = row.unwrap();
        let approach = probe_cart_approach(&mut ground, 30, center).unwrap_or_else(|| {
            panic!("Alterac leader{entry}/{guid} has no real nearby floor at{center:?}")
        });
        // combat.rs currently uses five yards for a normal melee swing.
        assert!(Vec3::from_array(approach).distance(center) <= 5.0);
        leader_approaches.push(serde_json::json!({"entry":entry,"guid":guid,"source":center.to_array(),"approach":approach}));
    }
    assert_eq!(leader_approaches.len(), 4);
    if let Some(path) = std::env::var_os("ALTERAC_APPROACH_RECEIPT") {
        let receipt = serde_json::json!({"starts":start_approaches,"banners":banner_approaches,"leaders":leader_approaches});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

#[test]
fn deephaul_source_starts_flag_and_cart_paths_have_real_walkable_approaches() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut ground = GroundMap::new(&client_data_dir(), HashMap::from([(2656, 5246270)]));
    let mut starts = db.prepare("SELECT ID,LocX,LocY,LocZ,TransportSpawnId FROM tdb_world_safe_locs WHERE ID IN (100070,100071) ORDER BY ID").unwrap();
    let starts: Vec<_> = starts
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
                row.get::<_, Option<u32>>(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(starts.len(), 2);
    for (id, at, transport) in starts {
        assert_eq!(transport, None);
        let (x, y, z) = crate::terrain_height::wow_to_bevy(at.x, at.y, at.z);
        let GroundProbe::Supported(height) = ground.ground_at(2656, Vec3::new(x, y, z)) else {
            panic!(
                "Deephaul start {id} has missing geometry: {:?}",
                ground.missing_files(2656)
            );
        };
        assert!(
            (height - at.z).abs() < 3.0,
            "Deephaul start {id} floor {height}, source {at:?}"
        );
    }
    let flag:Vec3 = db.query_row("SELECT position_x,position_y,position_z FROM tdb_gameobject WHERE map=2656 AND id=422413",[],|row| Ok(Vec3::new(row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
    let flag_approach =
        probe_cart_approach(&mut ground, 2656, flag).expect("flag player approach missing");
    assert!(Vec3::from_array(flag_approach).distance(flag) < 5.0);
    let capture_approaches: Vec<_> = shared::battleground::DEEPHAUL_CAPTURE_TRIGGERS
        .iter()
        .map(|trigger| {
            let center = Vec3::from_array(*trigger);
            let approach = probe_cart_approach(&mut ground, 2656, center)
                .expect("capture trigger player floor missing");
            assert!(Vec3::from_array(approach).distance(center) < 5.0);
            approach
        })
        .collect();
    let mut statement = db.prepare("SELECT PathId,PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId IN (21469000,21734600) ORDER BY PathId,NodeId").unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
            ))
        })
        .unwrap();
    let mut routes = std::collections::BTreeMap::<u32, Vec<Vec3>>::new();
    for row in rows {
        let (path, at) = row.unwrap();
        routes.entry(path).or_default().push(at);
    }
    assert_eq!(routes.len(), 2);
    let mut approaches = Vec::new();
    for definition in &shared::battleground::DEEPHAUL_CARTS {
        let mut previous = Vec3::from_array(definition.spawn);
        for &node in &routes[&definition.path] {
            for center in sample_cart_segment(previous, node) {
                let at = probe_cart_approach(&mut ground, 2656, center).unwrap_or_else(|| {
                    panic!(
                        "Deephaul path {} lacks player floor at {center:?}; missing {:?}",
                        definition.path,
                        ground.missing_files(2656)
                    )
                });
                assert!(Vec3::from_array(at).distance(center) < 30.0);
                approaches.push(serde_json::json!({"path":definition.path,"center":center.to_array(),"approach":at}));
            }
            previous = node;
        }
    }
    assert!(
        approaches.len() > 128,
        "{} samples exceed the original128source waypoints",
        approaches.len()
    );
    if let Some(path) = std::env::var_os("DEEPHAUL_APPROACH_RECEIPT") {
        let receipt = serde_json::json!({"cart_approaches":approaches,"flag_approach":flag_approach,"capture_approaches":capture_approaches});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

fn sample_cart_segment(start: Vec3, end: Vec3) -> impl Iterator<Item = Vec3> {
    let count = start.distance(end).ceil().max(1.0) as u32;
    (0..=count).map(move |index| start.lerp(end, index as f32 / count as f32))
}

#[test]
fn silvershard_authored_cart_routes_have_walkable_control_approaches() {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut statement = db.prepare("SELECT PathId,PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId BETWEEN 6014000 AND 6014011 ORDER BY PathId,NodeId").unwrap();
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                Vec3::new(row.get(1)?, row.get(2)?, row.get(3)?),
            ))
        })
        .unwrap();
    let mut routes = std::collections::BTreeMap::<u32, Vec<Vec3>>::new();
    for row in rows {
        let (id, position) = row.unwrap();
        routes.entry(id).or_default().push(position);
    }
    assert_eq!(routes.len(), 12);
    let mut ground = GroundMap::new(&client_data_dir(), HashMap::from([(727, 810604)]));
    let mut approaches = Vec::new();
    let mut append = |path, center: Vec3| {
        let approach = probe_cart_approach(&mut ground, 727, center).unwrap_or_else(|| {
            panic!("no grounded control approach on source path {path} at {center:?}")
        });
        assert!((Vec3::from_array(approach) - center).length() < 25.0);
        approaches
            .push(serde_json::json!({"path":path,"center":center.to_array(),"approach":approach}));
    };
    for definition in shared::battleground::SILVERSHARD_CARTS {
        let start = Vec3::from_array(definition.spawn);
        let end = routes[&definition.initial_path][0];
        for center in sample_cart_segment(start, end) {
            append(definition.initial_path, center);
        }
    }
    // TC switch branches connect these authored paths, including one-node connectors.
    for (from, to) in [
        (0, 1),
        (2, 3),
        (2, 5),
        (3, 4),
        (5, 6),
        (7, 8),
        (7, 10),
        (8, 9),
        (10, 11),
    ] {
        let from = 6014000 + from;
        let to = 6014000 + to;
        let start = *routes[&from].last().unwrap();
        let end = routes[&to][0];
        for center in sample_cart_segment(start, end) {
            append(to, center);
        }
    }
    for (path, nodes) in routes {
        assert!(!nodes.is_empty());
        // The authored depot connector paths can contain one node.
        for node in &nodes {
            append(path, *node);
        }
        for segment in nodes.windows(2) {
            for center in sample_cart_segment(segment[0], segment[1]) {
                append(path, center);
            }
        }
    }
    if let Some(path) = std::env::var_os("BATTLEGROUND_APPROACH_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&approaches).unwrap()).unwrap();
    }
    assert!(approaches.len() > 1000);
}

fn client_ground() -> GroundMap {
    GroundMap::new(
        &client_data_dir(),
        std::collections::HashMap::from([(0, AZEROTH_WDT_FDID)]),
    )
}

fn auction_house_feet(wow_z: f32) -> Vec3 {
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(AUCTION_HOUSE_WOW.0, AUCTION_HOUSE_WOW.1, wow_z);
    Vec3::new(x, y, z)
}

#[test]
fn auction_house_ground_is_its_floor_not_the_terrain_under_it() {
    let mut ground = client_ground();

    let on_floor = ground.ground_at(0, auction_house_feet(98.1));
    let under_floor = ground.ground_at(0, auction_house_feet(94.7));

    let GroundProbe::Supported(floor) = on_floor else {
        panic!("no ground on the auction house floor: {on_floor:?}");
    };
    assert!((floor - AUCTION_HOUSE_FLOOR).abs() < 0.05, "floor {floor}");
    let GroundProbe::Supported(terrain) = under_floor else {
        panic!("no ground under the auction house floor: {under_floor:?}");
    };
    assert!((terrain - 94.65).abs() < 0.05, "terrain {terrain}");
}

/// Player "Mover" standing at height `start_y` is moved by `game-server-admin
/// set-position` to WoW `(AUCTION_HOUSE_WOW, wow_z)`; returns their height and
/// health after two seconds of server gravity.
fn settle_after_set_position(start_y: f32, wow_z: f32) -> (f32, f32) {
    let db = persistence::temp_database();
    let (character_id, _) =
        persistence::create_character(&db, 1, "Mover", 1, 1).expect("character");
    let game_db = persistence::GameDatabase::new(db);
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins)
        .add_message::<crate::death::PlayerDeathTransition>()
        .insert_resource(client_ground());
    app.update();
    let player = app
        .world_mut()
        .spawn((
            Position {
                x: 0.0,
                y: start_y,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            VerticalVelocity::default(),
            FallTracker::grounded(start_y),
            crate::maps::InMap::continent(0),
            Player {
                name: "Mover".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            persistence::ClientId(character_id),
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
        "Mover",
        AUCTION_HOUSE_WOW.0,
        AUCTION_HOUSE_WOW.1,
        wow_z,
    );
    admin.apply(app.world_mut());

    for _ in 0..40 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        app.world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
    }

    let world = app.world();
    (
        world.get::<Position>(player).unwrap().y,
        world.get::<Health>(player).unwrap().current,
    )
}

/// `game-server-admin set-position` a little above the auction house floor: the
/// server's gravity lands the player on the floor.
#[test]
fn set_position_over_the_auction_house_floor_lands_on_it() {
    let (y, _) = settle_after_set_position(0.0, 99.0);

    assert!((y - AUCTION_HOUSE_FLOOR).abs() < 0.05, "player y {y}");
}

/// A set-position down from 40 yd higher, then a 2.4 yd drop to the floor, is a
/// reposition and a short fall: no fall damage.
#[test]
fn set_position_down_onto_the_floor_deals_no_fall_damage() {
    let (y, health) = settle_after_set_position(140.0, 100.5);

    assert!((y - AUCTION_HOUSE_FLOOR).abs() < 0.05, "player y {y}");
    assert_eq!(health, 100.0);
}

/// The old Tauren start (-2917, -257, 53) was below the extracted Kalimdor
/// terrain (59.429), so a new character fell forever despite the tile being loaded.
#[test]
fn new_tauren_starts_on_real_mulgore_ground_and_stays_there() {
    let db = persistence::temp_database();
    let (id, _) = persistence::create_character(&db, 1, "Mulgore", 6, 1).unwrap();
    let character = persistence::load_character(&db, id).unwrap();
    assert_eq!(crate::maps::persistence::load_character_map(&db, id), 1);
    assert_eq!(
        (character.x, character.y, character.z),
        (-2915.55, -257.347, 59.2693)
    );
    let mut ground = GroundMap::new(&client_data_dir(), HashMap::from([(1, 782_779)]));
    let (x, y, z) = crate::terrain_height::wow_to_bevy(character.x, character.y, character.z);
    assert_eq!(
        ground.ground_at(1, Vec3::new(-2917.0, 53.0, 257.0)),
        GroundProbe::Unsupported,
        "underground terrain must not become a reachable floor",
    );
    let GroundProbe::Supported(floor) = ground.ground_at(1, Vec3::new(x, y, z)) else {
        panic!("Tauren start at {y} has no reachable Mulgore ground");
    };
    assert!((floor - 59.45241).abs() < 0.001, "authored ground {floor}");
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins)
        .add_message::<crate::death::PlayerDeathTransition>()
        .insert_resource(ground);
    app.update();
    let player = app
        .world_mut()
        .spawn((
            Position { x, y, z },
            Health {
                current: 100.0,
                max: 100.0,
            },
            VerticalVelocity::default(),
            FallTracker::grounded(y),
            crate::maps::InMap::continent(1),
            Player {
                name: character.name,
                race: 6,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id();
    for _ in 0..200 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        app.world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
        let position = app.world().get::<Position>(player).unwrap();
        assert!(
            (position.y - floor).abs() < 0.1,
            "player fell to {}",
            position.y
        );
        assert_eq!(app.world().get::<Health>(player).unwrap().current, 100.0);
    }
}

/// `world/maps/stormwindjail/stormwindjail.wdt`: the Stockade, one global WMO (108631).
const STOCKADE_WDT_FDID: u32 = 791_060;

fn stockade_ground(ground: &mut GroundMap, wow_x: f32, wow_y: f32, wow_z: f32) -> GroundProbe {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow_x, wow_y, wow_z);
    ground.ground_at(34, Vec3::new(x, y, z))
}

/// The teleport targets players fell from in the Stockade lie outside its WMO: no face of any
/// of its 27 groups crosses those columns (walls and rock between the cells). A yard or two
/// east, on the Jail18 (-34.86) and Jail02 (-26.52) cell floors, the ground holds.
#[test]
fn stockade_fall_points_are_outside_the_wmo_and_its_floors_hold_beside_them() {
    let mut ground = GroundMap::new(
        &client_data_dir(),
        std::collections::HashMap::from([(34, STOCKADE_WDT_FDID)]),
    );
    for (x, y) in [(97.0, 76.0), (99.0, 77.0), (101.0, 78.0), (74.0, -16.0)] {
        for z in [-40.0, -35.0, -30.0, -26.0, -20.0, -10.0, 0.0, 10.0] {
            assert_eq!(
                stockade_ground(&mut ground, x, y, z),
                GroundProbe::Unsupported,
                "({x}, {y}, {z})"
            );
        }
    }
    // (103, 76) is inside Jail18, under barrel01.m2 (FDID 199563), MODD 523 of
    // WMO 108631's selected/default set 0. Its authored hull top is -33.924946;
    // use bare WMO columns here, not the barrel column. Fall columns above have
    // no terrain, WMO or M2 face, including with the full M2 bake present.
    for (x, y, floor) in [(106.0, 76.0, -34.86), (104.5, 77.3, -34.86)] {
        let GroundProbe::Supported(height) = stockade_ground(&mut ground, x, y, floor + 0.3) else {
            panic!("no Jail18 floor at ({x}, {y})");
        };
        assert!((height - floor).abs() < 0.05, "({x}, {y}) floor {height}");
    }
    let GroundProbe::Supported(barrel) = stockade_ground(&mut ground, 103.0, 76.0, -34.5) else {
        panic!("no authored MODD 523 barrel floor at (103, 76)");
    };
    assert!((barrel + 33.924946).abs() < 0.01, "barrel floor {barrel}");
    let GroundProbe::Supported(corridor) = stockade_ground(&mut ground, 78.0, -16.0, -26.0) else {
        panic!("no Jail02 floor at (78, -16)");
    };
    assert!(corridor < -26.0 && corridor > -28.0, "floor {corridor}");
}

/// A living player at WoW `(x, y, z)` on `map_id` (WDT `wdt`) under up to `seconds` of server
/// gravity, stopping when they die: their WoW z then, health, `PlayerDeathTransition`s and
/// `OutOfBounds`.
fn fall_for(
    map_id: u32,
    wdt: u32,
    (x, y, z): (f32, f32, f32),
    seconds: u32,
) -> (f32, f32, usize, bool) {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins)
        .add_message::<crate::death::PlayerDeathTransition>()
        .insert_resource(GroundMap::new(
            &client_data_dir(),
            std::collections::HashMap::from([(map_id, wdt)]),
        ));
    app.update();
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, z);
    let player = app
        .world_mut()
        .spawn((
            Position {
                x: bx,
                y: by,
                z: bz,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            VerticalVelocity::default(),
            FallTracker::grounded(by),
            MovementControl::default(),
            crate::maps::InMap {
                map_id,
                instance_id: 1,
            },
            Player {
                name: "Faller".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
        ))
        .id();
    for _ in 0..seconds * 20 {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(50));
        app.world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
        if app.world().get::<Health>(player).unwrap().current <= 0.0 {
            break;
        }
    }
    app.update();
    let deaths = app
        .world()
        .resource::<bevy::ecs::message::Messages<crate::death::PlayerDeathTransition>>()
        .len();
    let world = app.world();
    (
        world.get::<Position>(player).unwrap().y,
        world.get::<Health>(player).unwrap().current,
        deaths,
        world
            .get::<crate::networking::OutOfBounds>(player)
            .is_some(),
    )
}

#[test]
fn achievements_criteria_native_fall_death_distinguishes_ground_from_void() {
    use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
    use crate::auth::{Authenticated, InWorld};
    use crate::networking::PlayerLink;
    use crate::persistence::{self, ClientId, GameDatabase};
    for (coordinates, expected) in [
        ((-8950.0, -132.0, 183.0), Some(1)),
        ((-4500.0, -3210.0, -1100.0), None),
    ] {
        let mut app = App::new();
        app.add_plugins(bevy::MinimalPlugins)
            .add_message::<crate::death::PlayerDeathTransition>();
        app.insert_resource(GroundMap::new(
            &client_data_dir(),
            std::collections::HashMap::from([(0, AZEROTH_WDT_FDID)]),
        ));
        app.init_resource::<crate::maps::ServerClock>();
        let conn = rusqlite::Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let (character, _) = persistence::create_character(&db, 1, "Fallproof", 1, 1).unwrap();
        app.insert_resource(GameDatabase::new(db));
        app.update();
        let (x, y, z) =
            crate::terrain_height::wow_to_bevy(coordinates.0, coordinates.1, coordinates.2);
        let player = app
            .world_mut()
            .spawn((
                Position { x, y, z },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                VerticalVelocity::default(),
                FallTracker::grounded(y),
                MovementControl::default(),
                crate::maps::InMap {
                    map_id: 0,
                    instance_id: 1,
                },
                Player {
                    name: "Fallproof".into(),
                    race: 1,
                    class: 1,
                    appearance: Default::default(),
                },
                ClientId(character),
                PlayerAchievementState::default(),
            ))
            .id();
        app.world_mut().spawn((
            Authenticated { account_id: 1 },
            InWorld {
                character_id: character,
            },
            PlayerLink(player),
        ));
        for _ in 0..20 * 10 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_millis(50));
            app.world_mut()
                .run_system_once(crate::networking::apply_terrain_gravity)
                .unwrap();
            if app.world().get::<Health>(player).unwrap().current <= 0.0 {
                break;
            }
        }
        assert_eq!(app.world().get::<Health>(player).unwrap().current, 0.0);
        // TC's void6 is logged as Falling2, but must not credit the ordinary fall criterion149.
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&149)
                .copied(),
            expected
        );
        app.world_mut()
            .run_system_once(crate::networking::apply_terrain_gravity)
            .unwrap();
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&149)
                .copied(),
            expected
        );
        drop(app);
        let db = redb::Database::open(file.path()).unwrap();
        assert_eq!(
            persistence::load_achievements(&db, character)
                .criteria_progress
                .get(&149)
                .copied(),
            expected
        );
    }
}

/// Player::HandleFall under the map (MovementHandler.cpp:594-612): the Stockade has no grid,
/// so `Map::GetMinHeight` is -500 (TerrainMgr.cpp:671). Teleported into the rock at (99, 77),
/// the player falls, dies of DAMAGE_FALL_TO_VOID below -500 and is out of bounds.
#[test]
fn falling_out_of_the_stockade_kills_below_minus_500() {
    let (z, health, deaths, out_of_bounds) =
        fall_for(34, STOCKADE_WDT_FDID, (99.0, 77.0, -34.5), 30);

    assert_eq!((health, deaths, out_of_bounds), (0.0, 1, true));
    assert!((-501.0..-500.0).contains(&z), "died at {z}");

    let (z, health, deaths, out_of_bounds) =
        fall_for(34, STOCKADE_WDT_FDID, (99.0, 77.0, -34.5), 20);
    assert_eq!(
        (health, deaths, out_of_bounds),
        (100.0, 0, false),
        "still falling at {z}"
    );
}

/// GridMap::getMinHeight (GridMap.cpp:547-575) over the MFBO minimum planes of Eastern
/// Kingdoms tile (38, 40) (root ADT 779167: -1000 on its northern corners, -1237 on its
/// southern edge). At (-4500, -3700) the plane is -1207.38: a player falling from -1200 dies
/// there, not at -1000 or -500. At (-4500, -3210) the plane is -1000: -1100 is already under.
#[test]
fn mfbo_planes_bound_the_fall_on_a_continent_tile() {
    let (z, health, deaths, out_of_bounds) =
        fall_for(0, AZEROTH_WDT_FDID, (-4500.0, -3700.0, -1200.0), 1);
    assert_eq!((health, deaths, out_of_bounds), (0.0, 1, true));
    assert!((-1208.4..-1207.37).contains(&z), "died at {z}");

    let (_, health, deaths, _) = fall_for(0, AZEROTH_WDT_FDID, (-4500.0, -3210.0, -1100.0), 1);
    assert_eq!((health, deaths), (0.0, 1));
}

#[test]
fn map_min_height_follows_the_mfbo_plane_under_the_feet() {
    let mut ground = client_ground();
    let feet = |x: f32, y: f32| {
        let (bx, by, bz) = crate::terrain_height::wow_to_bevy(x, y, 0.0);
        Vec3::new(bx, by, bz)
    };
    let min = |ground: &mut GroundMap, x, y| ground.min_height(0, feet(x, y)).unwrap();
    assert!((min(&mut ground, -4500.0, -3700.0) - -1207.375).abs() < 0.01);
    assert!((min(&mut ground, -4300.0, -3600.0) - -1118.5).abs() < 0.01);
    assert!((min(&mut ground, -4700.0, -3650.0) - -1162.94).abs() < 0.01);
    assert_eq!(min(&mut ground, -4500.0, -3210.0), -1000.0);
    assert_eq!(min(&mut ground, -8820.0, 660.0), -1000.0);
}

#[test]
fn northshire_ground_under_the_blackrock_spy_is_the_terrain() {
    let mut ground = client_ground();

    // Below the ridge the Blackrock Spy spawns on (81.04): terrain 76.14.
    let (x, _, z) = crate::terrain_height::wow_to_bevy(-8966.63, -192.0, 76.0);
    let GroundProbe::Supported(floor) = ground.ground_at(0, Vec3::new(x, 81.04, z)) else {
        panic!("no Northshire ground");
    };
    assert!((floor - 76.14).abs() < 0.01, "floor {floor}");
}

#[test]
fn cached_stockade_has_every_ground_file_and_a_missing_cache_lacks_the_wdt() {
    let wdts = std::collections::HashMap::from([(34, STOCKADE_WDT_FDID)]);
    let cached = GroundMap::new(&client_data_dir(), wdts.clone());
    let missing = GroundMap::new(&std::env::temp_dir().join("no-such-game-engine-data"), wdts);

    assert_eq!(cached.missing_files(34), Vec::<String>::new());
    assert_eq!(missing.missing_files(34), ["terrain/791060.wdt"]);
    assert_eq!(missing.missing_files(1), [NO_WDT]);
}

#[test]
fn spawned_ground_check_fails_on_missing_files_except_the_known_gaps() {
    let ground = GroundMap::new(
        &std::env::temp_dir().join("no-such-game-engine-data"),
        std::collections::HashMap::from([(0, AZEROTH_WDT_FDID), (34, STOCKADE_WDT_FDID)]),
    );
    let gaps = HashSet::from([(34, "terrain/791060.wdt".to_string())]);

    let check = check_spawned_ground(&ground, [34, 0, 1], &gaps);

    assert_eq!(
        check,
        SpawnedGroundCheck {
            missing: vec![
                "map 0: terrain/775971.wdt".into(),
                format!("map 1: {NO_WDT}"),
            ],
            known_gaps: vec![34],
            known_gap_files: 1,
        }
    );
}

/// The Jasperlode Mine's south mouth (azeroth_33_49 MCNK 1,3 high-res holes 0xc0c0) is a
/// terrain hole under the mine's WMO floor: TrinityCore `GridMap::getHeight` returns
/// `INVALID_HEIGHT` there. Beside the hole the terrain is 61.5; in it the ground is the
/// mine floor (Kobold Miner 281565 spawns on it at 62.95), not the hole's interpolated
/// surface rising into the hill.
#[test]
fn jasperlode_mouth_hole_has_no_terrain_and_its_ground_is_the_mine_floor() {
    let mut ground = client_ground();
    let probe = |ground: &mut GroundMap, wow_x: f32, wow_y: f32, wow_z: f32| {
        let (x, y, z) = crate::terrain_height::wow_to_bevy(wow_x, wow_y, wow_z);
        ground.ground_at(0, Vec3::new(x, y, z))
    };

    let GroundProbe::Supported(beside) = probe(&mut ground, -9176.0, -595.8, 61.5) else {
        panic!("no terrain beside the hole");
    };
    assert!((beside - 61.5).abs() < 0.1, "terrain {beside}");
    let GroundProbe::Supported(floor) = probe(&mut ground, -9172.45, -599.435, 62.95) else {
        panic!("no ground under the Kobold Miner");
    };
    assert!((floor - 62.95).abs() < 0.2, "mine floor {floor}");
    // Under the mouth's floor, in the hole, there is nothing to stand on.
    assert_eq!(
        probe(&mut ground, -9172.0, -595.8, 58.0),
        GroundProbe::Unsupported
    );
}
