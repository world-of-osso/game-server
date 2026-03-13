use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{Health, ModelDisplay, Npc, Position, Zone};

use crate::creature_ai::{SpawnPoint, WanderPhase, WanderState, WaypointState};
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
    let count = spawn_elwynn_core(&mut commands, &world_data, true);
    info!("Spawned {count} creatures in Elwynn Forest area");
}

/// Spawn creatures in Elwynn bounds. When `replicate` is true, adds `Replicate`
/// component (requires lightyear plugins). Tests pass `false` to avoid that dependency.
fn spawn_elwynn_core(commands: &mut Commands, world_data: &WorldData, replicate: bool) -> u32 {
    let Some(indices) = world_data.spawns_by_map.get(&0) else {
        warn!("No spawns found for map 0 (Eastern Kingdoms)");
        return 0;
    };

    let mut count = 0u32;
    for &idx in indices {
        let spawn = &world_data.spawns[idx];
        if !in_elwynn_bounds(spawn.position_x, spawn.position_y) {
            continue;
        }
        let display_id = resolve_display_id(world_data, spawn.template_entry);
        let health = resolve_health(world_data, spawn.template_entry);
        let speed_walk = resolve_speed_walk(world_data, spawn.template_entry);
        let bundle = creature_bundle(spawn, health, display_id);

        let mut entity = commands.spawn(bundle);
        if replicate {
            entity.insert((
                Replicate::new(ReplicationMode::SingleServer(NetworkTarget::All)),
                NetworkVisibility,
            ));
        }
        insert_ai_components(&mut entity, spawn, speed_walk, world_data);
        count += 1;
    }
    count
}

/// Core creature components without replication marker.
fn creature_bundle(
    spawn: &crate::world_data::CreatureSpawn,
    health: f32,
    display_id: u32,
) -> (Position, Npc, Health, ModelDisplay, Zone) {
    // Spawn data is WoW coords; convert to Bevy space for ECS.
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(
        spawn.position_x,
        spawn.position_y,
        spawn.position_z,
    );
    (
        Position {
            x: bx,
            y: by,
            z: bz,
        },
        Npc {
            template_id: spawn.template_entry,
        },
        Health {
            current: health,
            max: health,
        },
        ModelDisplay { display_id },
        Zone {
            id: spawn.zone_id as u32,
        },
    )
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

fn resolve_speed_walk(world_data: &WorldData, template_entry: u32) -> f32 {
    world_data
        .templates
        .get(&template_entry)
        .map(|t| t.speed_walk)
        .unwrap_or(1.0)
}

fn insert_ai_components(
    entity: &mut EntityCommands,
    spawn: &crate::world_data::CreatureSpawn,
    speed_walk: f32,
    world_data: &WorldData,
) {
    let movement_type = spawn.movement_type;
    if movement_type == 1 {
        insert_wander(entity, spawn, speed_walk);
    } else if movement_type == 2 {
        insert_waypoint(entity, spawn, speed_walk, world_data);
    }
}

fn insert_wander(
    entity: &mut EntityCommands,
    spawn: &crate::world_data::CreatureSpawn,
    speed_walk: f32,
) {
    let (bx, by, bz) = crate::terrain_height::wow_to_bevy(
        spawn.position_x,
        spawn.position_y,
        spawn.position_z,
    );
    entity.insert(SpawnPoint {
        x: bx,
        y: by,
        z: bz,
        wander_distance: spawn.wander_distance,
        speed_walk,
    });
    entity.insert(WanderState {
        phase: WanderPhase::Idle,
        target_x: bx,
        target_z: bz,
        idle_remaining: 1.0,
        seed: spawn.guid,
    });
}

fn insert_waypoint(
    entity: &mut EntityCommands,
    spawn: &crate::world_data::CreatureSpawn,
    speed_walk: f32,
    world_data: &WorldData,
) {
    if !world_data.waypoints.contains_key(&spawn.guid) {
        return;
    }
    entity.insert(WaypointState {
        creature_guid: spawn.guid,
        current_index: 0,
        delay_remaining: 0.0,
        speed_walk,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate};
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

    /// Test-only system: spawns creatures without `Replicate` (which requires lightyear plugins).
    fn spawn_elwynn_test(mut commands: Commands, world_data: Res<WorldData>) {
        spawn_elwynn_core(&mut commands, &world_data, false);
    }

    fn run_spawn_system(wd: WorldData) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(wd);
        app.add_systems(PostStartup, spawn_elwynn_test);
        app.update();
        app
    }

    fn get_spawned(app: &mut App) -> Vec<(Position, Npc, Health, ModelDisplay, Zone)> {
        let mut query = app
            .world_mut()
            .query::<(&Position, &Npc, &Health, &ModelDisplay, &Zone)>();
        let mut results: Vec<_> = query
            .iter(app.world())
            .map(|(p, n, h, m, z)| (*p, *n, *h, *m, *z))
            .collect();
        results.sort_by_key(|(_, npc, _, _, _)| npc.template_id);
        results
    }

    fn assert_creature(
        c: &(Position, Npc, Health, ModelDisplay, Zone),
        template_id: u32,
        wow_x: f32,
        health: f32,
        display: u32,
    ) {
        assert_eq!(c.1.template_id, template_id);
        // Position is now Bevy space; wow_x maps to bevy_x unchanged.
        assert_eq!(c.0.x, wow_x);
        assert_eq!(c.2.current, health);
        assert_eq!(c.2.max, health);
        assert_eq!(c.3.display_id, display);
    }

    #[test]
    fn spawn_system_creates_entities_in_bounds() {
        let templates = vec![
            make_template(1, 1.5, 1001),
            make_template(2, 2.0, 1002),
            make_template(3, 1.0, 1003),
        ];
        let spawns = vec![
            make_spawn(1, -9000.0, -100.0, 50.0),
            make_spawn(2, -8800.0, 0.0, 60.0),
            make_spawn(3, -10000.0, 0.0, 70.0), // outside bounds
        ];
        let mut app = run_spawn_system(make_world_data(templates, spawns));
        let results = get_spawned(&mut app);

        assert_eq!(results.len(), 2, "only 2 of 3 spawns are in Elwynn bounds");
        assert_creature(&results[0], 1, -9000.0, 150.0, 1001);
        assert_creature(&results[1], 2, -8800.0, 200.0, 1002);
    }

    #[test]
    fn spawn_system_no_map0_spawns() {
        let templates = vec![make_template(1, 1.0, 100)];
        let mut spawn = make_spawn(1, -9000.0, -100.0, 50.0);
        spawn.map = 1; // not Eastern Kingdoms
        let mut app = run_spawn_system(make_world_data(templates, vec![spawn]));
        assert_eq!(get_spawned(&mut app).len(), 0);
    }

    #[test]
    fn spawn_system_missing_template_uses_defaults() {
        let mut app = run_spawn_system(make_world_data(
            vec![],
            vec![make_spawn(99, -9000.0, 0.0, 0.0)],
        ));
        let results = get_spawned(&mut app);
        assert_eq!(results.len(), 1);
        assert_creature(&results[0], 99, -9000.0, 100.0, 0);
    }
}
