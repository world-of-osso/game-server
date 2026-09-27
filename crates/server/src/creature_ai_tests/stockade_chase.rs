//! Riverpaw Slayer 20375682 (world.db `content_creature`, map 34) chasing a player through
//! the Stockade, on the real Stockade WMO ground (`stormwindjail.wdt`, one global WMO).
use super::*;
use crate::ground::{GroundMap, GroundProbe};
use shared::components::CreatureMotion;

/// `world/maps/stormwindjail/stormwindjail.wdt`.
const STOCKADE_WDT_FDID: u32 = 791_060;
const STOCKADE: u32 = 34;
/// Riverpaw Slayer 20375682, WoW spawn position.
const SLAYER_SPAWN: (f32, f32, f32) = (115.646, 46.6108, -34.8562);
/// On the cell block floor beside the Riverpaws (WMO floor -33.86).
const PLAYER_ON_FLOOR: (f32, f32, f32) = (128.5, 52.0, -33.86);
/// On the upper corridor over the cell block, where Petty Criminal 46382 stands (floor -26.44).
const PLAYER_ON_CORRIDOR: (f32, f32, f32) = (111.0, 24.6, -26.44);

fn stockade_ground() -> GroundMap {
    GroundMap::new(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        std::collections::HashMap::from([(STOCKADE, STOCKADE_WDT_FDID)]),
    )
}

fn bevy((x, y, z): (f32, f32, f32)) -> Position {
    let (x, y, z) = wow_to_bevy(x, y, z);
    Position { x, y, z }
}

fn stockade_app() -> App {
    let mut app = ai_test_app();
    app.insert_resource(stockade_ground());
    app
}

fn player(app: &mut App, at: (f32, f32, f32)) -> Entity {
    app.world_mut()
        .spawn((bevy(at), crate::maps::InMap::continent(STOCKADE)))
        .id()
}

fn slayer(app: &mut App, state: CreatureState) -> Entity {
    let spawn = bevy(SLAYER_SPAWN);
    app.world_mut()
        .spawn((
            spawn,
            SpawnPoint {
                x: spawn.x,
                y: spawn.y,
                z: spawn.z,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.142_86,
            },
            state,
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            CreatureMotion::default(),
            crate::maps::InMap::continent(STOCKADE),
        ))
        .id()
}

/// The floor under `pos`, `None` in the columns of the cell walls and bars, which have no
/// floor face (the straight chase line crosses them).
fn floor_under(ground: &mut GroundMap, pos: Position) -> Option<f32> {
    match ground.ground_at(STOCKADE, Vec3::new(pos.x, pos.y, pos.z)) {
        GroundProbe::Supported(floor) => Some(floor),
        GroundProbe::Unsupported => None,
        GroundProbe::Unloaded => panic!("Stockade ground not loaded"),
    }
}

/// `Rotation.y` of a creature facing along Bevy `(dx, dz)` (WoW orientation + π/2).
fn facing(dx: f32, dz: f32) -> f32 {
    (-dz).atan2(dx) + std::f32::consts::FRAC_PI_2
}

fn same_angle(a: f32, b: f32) -> bool {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d < 0.01 || d > std::f32::consts::TAU - 0.01
}

/// The slayer runs across the floor to the player: it stands on the floor each tick (the
/// player is a yard higher), faces where it runs with `Run` motion, then stops 2 yd short,
/// `Still`, facing the player (`ChaseMovementGenerator` `SetInFront`).
#[test]
fn riverpaw_runs_on_the_floor_facing_its_way_and_stops_facing_the_player() {
    let mut app = stockade_app();
    let target = player(&mut app, PLAYER_ON_FLOOR);
    let chaser = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut ground = stockade_ground();
    let mut last = *app.world().get::<Position>(chaser).unwrap();
    let mut ran = 0;
    for tick in 0..60 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        let rotation = app.world().get::<Rotation>(chaser).unwrap().y;
        let motion = *app.world().get::<CreatureMotion>(chaser).unwrap();
        let (dx, dz) = (pos.x - last.x, pos.z - last.z);
        if dx.hypot(dz) > 0.001 {
            ran += 1;
            assert_eq!(motion, CreatureMotion::Run, "tick {tick}");
            assert!(
                same_angle(rotation, facing(dx, dz)),
                "tick {tick}: {rotation}"
            );
            if let Some(floor) = floor_under(&mut ground, pos) {
                assert!(
                    (pos.y - floor).abs() < 0.05,
                    "tick {tick}: y {} floor {floor}",
                    pos.y
                );
            }
        }
        last = pos;
    }
    assert!(ran > 5, "ran {ran} ticks");
    let target_pos = bevy(PLAYER_ON_FLOOR);
    let gap = (target_pos.x - last.x).hypot(target_pos.z - last.z);
    assert!((gap - 2.0).abs() < 0.01, "stops 2 yd short: {gap}");
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
    let rotation = app.world().get::<Rotation>(chaser).unwrap().y;
    assert!(
        same_angle(
            rotation,
            facing(target_pos.x - last.x, target_pos.z - last.z)
        ),
        "faces the player: {rotation}"
    );
}

/// A player on the corridor 8 yd above: the slayer stays on the cell block floor under it
/// instead of rising through the air toward the player's height.
#[test]
fn riverpaw_chasing_a_player_above_stays_on_its_floor() {
    let mut app = stockade_app();
    let target = player(&mut app, PLAYER_ON_CORRIDOR);
    let chaser = slayer(
        &mut app,
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
    );
    let mut ground = stockade_ground();
    for tick in 0..80 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        if let Some(floor) = floor_under(&mut ground, pos) {
            assert!(
                (pos.y - floor).abs() < 0.05,
                "tick {tick}: WoW z {} over the floor at {floor}",
                pos.y
            );
        }
    }
}

/// Evading from the player back to its spawn a yard lower, the slayer runs on the floor
/// (it kept the player's floor height until arrival), `Run` until home, then `Still`.
#[test]
fn riverpaw_returns_home_on_the_floor() {
    let mut app = stockade_app();
    let chaser = slayer(&mut app, CreatureState::Returning);
    *app.world_mut().get_mut::<Position>(chaser).unwrap() = bevy(PLAYER_ON_FLOOR);
    let mut ground = stockade_ground();
    let mut last = bevy(PLAYER_ON_FLOOR);
    for tick in 0..60 {
        app.update();
        let pos = *app.world().get::<Position>(chaser).unwrap();
        let motion = *app.world().get::<CreatureMotion>(chaser).unwrap();
        let returning =
            *app.world().get::<CreatureState>(chaser).unwrap() == CreatureState::Returning;
        if returning && (pos.x - last.x).hypot(pos.z - last.z) > 0.001 {
            assert_eq!(motion, CreatureMotion::Run, "tick {tick}");
        }
        if let Some(floor) = floor_under(&mut ground, pos) {
            assert!(
                (pos.y - floor).abs() < 0.05,
                "tick {tick}: WoW z {} over the floor at {floor}",
                pos.y
            );
        }
        last = pos;
    }
    assert_eq!(
        *app.world().get::<CreatureState>(chaser).unwrap(),
        CreatureState::Idle
    );
    assert_eq!(
        *app.world().get::<CreatureMotion>(chaser).unwrap(),
        CreatureMotion::Still
    );
}
