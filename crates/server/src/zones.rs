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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate, WaypointPath};
    use std::collections::HashMap;

    fn make_template(entry: u32, health_mod: f32, display_id: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: format!("Creature-{entry}"),
            subname: None,
            min_level: 1,
            max_level: 1,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.14,
            scale: 1.0,
            rank: 0,
            unit_class: 1,
            unit_flags: 0,
            creature_type: 1,
            health_modifier: health_mod,
            mana_modifier: 1.0,
            ai_name: String::new(),
            movement_type: 0,
            flags_extra: 0,
            models: vec![CreatureModelInfo {
                display_id,
                display_scale: 1.0,
                probability: 1.0,
            }],
        }
    }

    fn make_world_data(templates: Vec<CreatureTemplate>, spawns: Vec<CreatureSpawn>) -> WorldData {
        let mut template_map = HashMap::new();
        for t in templates {
            template_map.insert(t.entry, t);
        }
        let mut spawns_by_map: HashMap<u16, Vec<usize>> = HashMap::new();
        for (i, s) in spawns.iter().enumerate() {
            spawns_by_map.entry(s.map).or_default().push(i);
        }
        WorldData {
            templates: template_map,
            spawns,
            spawns_by_map,
            waypoints: HashMap::new(),
        }
    }

    fn make_spawn(entry: u32, x: f32, y: f32, z: f32) -> CreatureSpawn {
        CreatureSpawn {
            guid: 1,
            template_entry: entry,
            map: 0,
            zone_id: 0,
            area_id: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: 0.0,
            respawn_time_secs: 300,
            wander_distance: 0.0,
            movement_type: 0,
        }
    }

    #[test]
    fn bounds_inside() {
        assert!(in_elwynn_bounds(-9000.0, -100.0));
    }

    #[test]
    fn bounds_edges() {
        assert!(in_elwynn_bounds(SPAWN_MIN_X, SPAWN_MIN_Y));
        assert!(in_elwynn_bounds(SPAWN_MAX_X, SPAWN_MAX_Y));
    }

    #[test]
    fn bounds_outside() {
        assert!(!in_elwynn_bounds(-10000.0, 0.0)); // too far west
        assert!(!in_elwynn_bounds(-9000.0, 500.0)); // too far north
    }

    #[test]
    fn display_id_from_template() {
        let wd = make_world_data(vec![make_template(42, 1.0, 5555)], vec![]);
        assert_eq!(resolve_display_id(&wd, 42), 5555);
    }

    #[test]
    fn display_id_missing_template() {
        let wd = make_world_data(vec![], vec![]);
        assert_eq!(resolve_display_id(&wd, 999), 0);
    }

    #[test]
    fn health_from_modifier() {
        let wd = make_world_data(vec![make_template(10, 2.5, 0)], vec![]);
        assert_eq!(resolve_health(&wd, 10), 250.0);
    }

    #[test]
    fn health_missing_template() {
        let wd = make_world_data(vec![], vec![]);
        assert_eq!(resolve_health(&wd, 999), 100.0);
    }
}
