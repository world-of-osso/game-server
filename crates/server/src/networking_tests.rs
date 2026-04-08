use super::*;
use bevy::ecs::system::RunSystemOnce;
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
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            MovementSpeed(DEFAULT_RUN_SPEED),
            VerticalVelocity::default(),
            Player {
                name: "test".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Auras::default(),
        ))
        .id()
}

fn make_input(dx: f32, dy: f32, dz: f32) -> PlayerInput {
    PlayerInput {
        direction: [dx, dy, dz],
        facing_yaw: 0.0,
        jumping: false,
        running: true,
        swimming: false,
    }
}

fn setup_app_with_time() -> App {
    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.update();
    app.update();
    app
}

#[path = "networking_tests/misc.rs"]
mod misc;
#[path = "networking_tests/movement.rs"]
mod movement;
#[path = "networking_tests/physics.rs"]
mod physics;
