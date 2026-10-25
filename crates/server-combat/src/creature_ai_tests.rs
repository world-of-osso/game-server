use super::*;

/// Find the nearest waypoint index to resume patrol after combat.
fn nearest_waypoint(pos: &Position, path: &crate::world_data::WaypointPath) -> usize {
    let mut best_idx = 0;
    let mut best_dist = f32::MAX;
    for (i, wp) in path.points.iter().enumerate() {
        let (bx, _by, bz) =
            crate::terrain_height::wow_to_bevy(wp.position_x, wp.position_y, wp.position_z);
        let dx = pos.x - bx;
        let dz = pos.z - bz;
        let dist_sq = dx * dx + dz * dz;
        if dist_sq < best_dist {
            best_dist = dist_sq;
            best_idx = i;
        }
    }
    best_idx
}

/// Build an app with creature AI systems registered and time advancing.
/// Uses FixedUpdate at 20Hz (0.05s timestep) like the real server.
fn ai_test_app() -> App {
    use bevy::time::TimeUpdateStrategy;
    use std::collections::HashMap;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    app.init_resource::<MapStore>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(WorldData {
        templates: HashMap::new().into(),
        spawns: vec![].into(),
        spawns_by_map: HashMap::new(),
        waypoints: HashMap::new(),
        ..Default::default()
    });
    app.add_systems(
        FixedUpdate,
        (
            wander_system,
            waypoint_system,
            (
                chase_system,
                no_path_evade_system,
                leash_check_system,
                return_to_spawn_system,
            )
                .chain(),
        ),
    );
    app.update();
    app
}

#[path = "creature_ai_tests/chase.rs"]
mod chase;
#[path = "creature_ai_tests/integration.rs"]
mod integration;
#[path = "creature_ai_tests/leash.rs"]
mod leash;
#[path = "creature_ai_tests/movement.rs"]
mod movement;
#[path = "creature_ai_tests/return_to_spawn.rs"]
mod return_to_spawn;
#[path = "creature_ai_tests/state.rs"]
mod state;
#[path = "creature_ai_tests/stockade_chase.rs"]
mod stockade_chase;
#[path = "creature_ai_tests/wander.rs"]
mod wander;
#[path = "creature_ai_tests/waypoint.rs"]
mod waypoint;
