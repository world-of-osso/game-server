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
