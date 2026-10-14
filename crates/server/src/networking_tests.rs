use super::*;
use crate::auras::AuraStatMods;
use bevy::ecs::system::RunSystemOnce;
use shared::components::{MovementSpeed, Rotation};
use shared::protocol::{ChatType, PlayerInput};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Component)]
struct ReplicationTag;

#[derive(Component)]
struct VisibilityTag;

fn spawn_player(world: &mut World, x: f32, y: f32, z: f32) -> Entity {
    world
        .spawn((
            Position { x, y, z },
            Health {
                current: 100.0,
                max: 100.0,
            },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            MovementSpeed(DEFAULT_RUN_SPEED),
            shared::components::PlayerMotion::default(),
            VerticalVelocity::default(),
            FallTracker::grounded(y),
            crate::maps::InMap::continent(0),
            Player {
                name: "test".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            AuraStatMods::default(),
        ))
        .id()
}

/// Movement reads the aura run speed multiplier from `AuraStatMods`.
fn set_aura_speed(world: &mut World, player: Entity, movement_speed: f32) {
    world.entity_mut(player).insert(AuraStatMods {
        movement_speed,
        ..Default::default()
    });
}

/// One 20 fps client frame.
const TEST_STEP_SECS: f32 = 0.05;
/// A 7 yd/s run for one 20 fps frame.
const RUN_STEP: f32 = DEFAULT_RUN_SPEED * TEST_STEP_SECS;

/// The input of a client that ran one frame along `(dx, dz)` from the origin, facing it.
fn make_input(dx: f32, dy: f32, dz: f32) -> PlayerInput {
    let facing_yaw = if dx.abs() > f32::EPSILON || dz.abs() > f32::EPSILON {
        dx.atan2(dz)
    } else {
        0.0
    };
    let ran = Vec2::new(dx, dz).normalize_or_zero() * RUN_STEP;
    PlayerInput {
        transport: None,
        direction: [dx, dy, dz],
        facing_yaw,
        jumping: false,
        running: true,
        swimming: false,
        flying: false,
        position: [ran.x, 0.0, ran.y],
        epoch: 0,
    }
}

/// `input` reporting that the client put the player at `position`.
fn reporting(position: [f32; 3], input: PlayerInput) -> PlayerInput {
    PlayerInput { position, ..input }
}

/// A report 10 yd along +x, farther than any speed covers in one input.
const FAR_X: [f32; 3] = [10.0, 0.0, 0.0];

fn setup_app_with_time() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.add_message::<crate::death::PlayerDeathTransition>();
    app.update();
    app.update();
    app
}

#[path = "networking_tests/active_spec.rs"]
mod active_spec;
#[path = "networking_tests/admin_position.rs"]
mod admin_position;
#[path = "networking_tests/bag_slots.rs"]
mod bag_slots;
#[path = "networking_tests/client_height.rs"]
mod client_height;
#[path = "networking_tests/death.rs"]
mod death;
#[path = "networking_tests/flight.rs"]
mod flight;
#[path = "networking_tests/forced_disconnect.rs"]
mod forced_disconnect;
#[path = "networking_tests/ghoststate.rs"]
mod ghoststate;
#[path = "networking_tests/misc.rs"]
mod misc;
#[path = "networking_tests/movement.rs"]
mod movement;
#[path = "networking_tests/physics.rs"]
mod physics;
#[path = "networking_tests/player_motion.rs"]
mod player_motion;
#[path = "networking_tests/receiver_rate.rs"]
mod receiver_rate;
#[path = "networking_tests/regen.rs"]
mod regen;
#[path = "networking_tests/sheet_stats.rs"]
mod sheet_stats;
#[path = "networking_tests/skyriding.rs"]
mod skyriding;
#[path = "networking_tests/stand_state.rs"]
mod stand_state;
#[path = "networking_tests/stockade_run.rs"]
mod stockade_run;
#[path = "networking_tests/terrain.rs"]
mod terrain;
#[path = "networking_tests/vignette.rs"]
mod vignette;
#[path = "networking_tests/world_entry.rs"]
mod world_entry;
