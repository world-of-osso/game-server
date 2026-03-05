use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, ModelDisplay, Npc, Position};

use crate::world_data::WorldData;

/// Elwynn/Northshire spatial bounds for initial creature spawning.
const SPAWN_MIN_X: f32 = -9600.0;
const SPAWN_MAX_X: f32 = -8600.0;
const SPAWN_MIN_Y: f32 = -800.0;
const SPAWN_MAX_Y: f32 = 200.0;

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_elwynn_creatures);
    }
}

fn spawn_elwynn_creatures(mut commands: Commands, world_data: Res<WorldData>) {
    let Some(indices) = world_data.spawns_by_map.get(&0) else {
        warn!("No spawns found for map 0 (Eastern Kingdoms)");
        return;
    };

    let mut count = 0u32;
    for &idx in indices {
        let spawn = &world_data.spawns[idx];
        if !in_elwynn_bounds(spawn.position_x, spawn.position_y) {
            continue;
        }
        let display_id = resolve_display_id(&world_data, spawn.template_entry);
        let health = resolve_health(&world_data, spawn.template_entry);

        commands.spawn((
            Position {
                x: spawn.position_x,
                y: spawn.position_y,
                z: spawn.position_z,
            },
            Npc {
                template_id: spawn.template_entry,
            },
            Health {
                current: health,
                max: health,
            },
            ModelDisplay { display_id },
            Replicate::default(),
        ));
        count += 1;
    }
    info!("Spawned {count} creatures in Elwynn Forest area");
}

fn in_elwynn_bounds(x: f32, y: f32) -> bool {
    x >= SPAWN_MIN_X && x <= SPAWN_MAX_X && y >= SPAWN_MIN_Y && y <= SPAWN_MAX_Y
}

fn resolve_display_id(world_data: &WorldData, template_entry: u32) -> u32 {
    world_data
        .templates
        .get(&template_entry)
        .and_then(|t| t.models.first())
        .map(|m| m.display_id)
        .unwrap_or(0)
}

fn resolve_health(world_data: &WorldData, template_entry: u32) -> f32 {
    world_data
        .templates
        .get(&template_entry)
        .map(|t| t.health_modifier * 100.0)
        .unwrap_or(100.0)
}
