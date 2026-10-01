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
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data")
}

/// `world/maps/azeroth/azeroth.wdt`.
const AZEROTH_WDT_FDID: u32 = 775_971;

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
    let game_db = persistence::GameDatabase { db };
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
    for (x, y, floor) in [(103.0, 76.0, -34.86), (104.5, 77.3, -34.86)] {
        let GroundProbe::Supported(height) = stockade_ground(&mut ground, x, y, floor + 0.3) else {
            panic!("no Jail18 floor at ({x}, {y})");
        };
        assert!((height - floor).abs() < 0.05, "({x}, {y}) floor {height}");
    }
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
fn azeroth_ground_opens_at_startup_from_the_client_cache() {
    let mut ground = client_ground();

    assert_eq!(ground.open_now(0), Ok(()));
    // Northshire, below the ridge the Blackrock Spy spawns on: terrain 76.14.
    let (x, _, z) = crate::terrain_height::wow_to_bevy(-8966.63, -192.0, 76.0);
    let GroundProbe::Supported(floor) = ground.ground_at(0, Vec3::new(x, 81.04, z)) else {
        panic!("no Northshire ground");
    };
    assert!((floor - 76.14).abs() < 0.01, "floor {floor}");
}

#[test]
fn missing_client_cache_fails_naming_the_azeroth_wdt() {
    let missing = std::env::temp_dir().join("no-such-game-engine-data");
    let mut ground = GroundMap::new(
        &missing,
        std::collections::HashMap::from([(0, AZEROTH_WDT_FDID)]),
    );

    let err = ground.open_now(0).unwrap_err();

    let wdt = missing.join("terrain/775971.wdt");
    assert!(err.starts_with(&format!("{}: ", wdt.display())), "{err}");
}
