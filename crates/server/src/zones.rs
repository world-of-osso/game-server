use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{
    Health, ModelDisplay, Mounted, Npc, Position, Rotation, UnitFactionTemplate, UnitLevel,
    UnitPowers, Zone,
};
use shared::protocol::NpcFlags;
use std::collections::HashMap;

use crate::combat::RespawnDelay;
use crate::creature_ai::{FormationMember, SpawnPoint, WanderPhase, WanderState, WaypointState};
use crate::interest::NetworkVisibility;
use crate::world_data::{CreatureAddon, CreatureSpawn, GROUP_AI_FLAG_FOLLOW_LEADER, WorldData};

/// Elwynn/Northshire spatial bounds for initial creature spawning.
const SPAWN_MIN_X: f32 = -9600.0;
const SPAWN_MAX_X: f32 = -8600.0;
const SPAWN_MIN_Y: f32 = -800.0;
const SPAWN_MAX_Y: f32 = 200.0;

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_elwynn_creatures)
            .add_systems(FixedUpdate, apply_addon_auras);
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
    let spawns = indices
        .iter()
        .map(|&idx| &world_data.spawns[idx])
        .filter(|spawn| in_elwynn_bounds(spawn.position_x, spawn.position_y));
    spawn_creatures(commands, world_data, replicate, spawns).len() as u32
}

/// Spawn `spawns` with their AI, addon and formation components; returns
/// the entity of each spawn guid.
fn spawn_creatures<'a>(
    commands: &mut Commands,
    world_data: &WorldData,
    replicate: bool,
    spawns: impl Iterator<Item = &'a CreatureSpawn>,
) -> HashMap<u32, Entity> {
    let mut entities = HashMap::new();
    for spawn in spawns {
        let display_id = resolve_display_id(world_data, spawn.template_entry);
        let health = resolve_health(world_data, spawn.template_entry);
        let template = world_data
            .templates
            .get(&spawn.template_entry)
            .unwrap_or_else(|| {
                panic!(
                    "creature spawn {} references missing template {}",
                    spawn.guid, spawn.template_entry
                )
            });
        let bundle = creature_bundle(spawn, &template.name, health, display_id);

        let mut entity = commands.spawn(bundle);
        // NPC power pools need creature class/level stats, which are not imported.
        entity.insert((
            UnitLevel(template.min_level),
            UnitPowers::default(),
            UnitFactionTemplate(u32::from(template.faction)),
            NpcFlags::from_azerothcore(template.npc_flags),
        ));
        if replicate {
            entity.insert((
                Replicate::to_clients(NetworkTarget::All),
                NetworkVisibility::default(),
            ));
        }
        let addon = world_data.addon_for(spawn);
        insert_ai_components(&mut entity, spawn, template, addon, world_data);
        if let Some(addon) = addon {
            insert_addon_components(&mut entity, addon);
        }
        entities.insert(spawn.guid, entity.id());
    }
    insert_formation_members(commands, world_data, &entities);
    entities
}

/// `creature_addon` mount and auras (Creature::LoadCreaturesAddon).
fn insert_addon_components(entity: &mut EntityCommands, addon: &CreatureAddon) {
    if addon.mount != 0 {
        entity.insert(Mounted {
            mount_display_id: addon.mount,
        });
    }
    if !addon.auras.is_empty() {
        entity.insert(AddonAuras(addon.auras.clone()));
    }
}

/// Link spawned followers to their spawned leader (FormationMgr::AddCreatureToGroup).
fn insert_formation_members(
    commands: &mut Commands,
    world_data: &WorldData,
    entities: &HashMap<u32, Entity>,
) {
    for (guid, &member) in entities {
        let Some(info) = world_data.formations.get(guid) else {
            continue;
        };
        if info.group_ai & GROUP_AI_FLAG_FOLLOW_LEADER == 0 {
            continue;
        }
        let Some(&leader) = entities.get(&info.leader_guid) else {
            continue;
        };
        commands.entity(member).insert(FormationMember {
            leader,
            follow_dist: info.follow_dist,
            follow_angle: info.follow_angle,
        });
    }
}

/// `creature_addon.auras` not yet applied to the spawned creature.
#[derive(Component, Debug)]
pub(crate) struct AddonAuras(pub Vec<u32>);

/// Self-cast addon auras once the creature exists (Creature::LoadCreaturesAddon `AddAura`).
fn apply_addon_auras(world: &mut World) {
    let pending: Vec<(Entity, Vec<u32>)> = world
        .query::<(Entity, &AddonAuras)>()
        .iter(world)
        .map(|(entity, auras)| (entity, auras.0.clone()))
        .collect();
    for (creature, spell_ids) in pending {
        world.entity_mut(creature).remove::<AddonAuras>();
        for spell_id in spell_ids {
            apply_addon_aura(world, creature, spell_id);
        }
    }
}

fn apply_addon_aura(world: &mut World, creature: Entity, spell_id: u32) {
    let Some(effect_mask) = world
        .get_resource::<crate::spell_info::SpellStore>()
        .and_then(|store| store.get(spell_id))
        .map(crate::auras::aura_effect_mask)
    else {
        warn!("creature_addon aura spell {spell_id} not in SpellStore");
        return;
    };
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster: creature,
            target: creature,
            spell_id,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// Core creature components without replication marker.
fn creature_bundle(
    spawn: &crate::world_data::CreatureSpawn,
    name: &str,
    health: f32,
    display_id: u32,
) -> (Position, Npc, Health, ModelDisplay, Zone, Rotation) {
    // Spawn data is WoW coords; convert to Bevy space for ECS.
    let (bx, by, bz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    (
        Position {
            x: bx,
            y: by,
            z: bz,
        },
        Npc {
            template_id: spawn.template_entry,
            name: name.to_owned(),
        },
        Health {
            current: health,
            max: health,
        },
        ModelDisplay { display_id },
        Zone {
            id: spawn.zone_id as u32,
        },
        Rotation {
            x: 0.0,
            y: spawn.orientation + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    )
}

fn in_elwynn_bounds(x: f32, y: f32) -> bool {
    (SPAWN_MIN_X..=SPAWN_MAX_X).contains(&x) && (SPAWN_MIN_Y..=SPAWN_MAX_Y).contains(&y)
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

/// Every spawn gets a `SpawnPoint` and `RespawnDelay` so it respawns where it
/// was placed; MovementType 1 wanders, 2 patrols.
fn insert_ai_components(
    entity: &mut EntityCommands,
    spawn: &CreatureSpawn,
    template: &crate::world_data::CreatureTemplate,
    addon: Option<&CreatureAddon>,
    world_data: &WorldData,
) {
    let (bx, by, bz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    entity.insert((
        SpawnPoint {
            x: bx,
            y: by,
            z: bz,
            wander_distance: spawn.wander_distance,
            speed_walk: template.speed_walk,
        },
        RespawnDelay::for_spawn(spawn.spawntimesecs, template.rank),
    ));
    if spawn.movement_type == 1 {
        entity.insert(WanderState {
            phase: WanderPhase::Idle,
            target_x: bx,
            target_z: bz,
            idle_remaining: 1.0,
            seed: spawn.guid,
        });
    } else if spawn.movement_type == 2
        && let Some(path_id) = patrol_path(addon, world_data)
    {
        entity.insert(WaypointState::new(
            path_id,
            template.speed_walk,
            template.speed_run,
            (bx, bz),
        ));
    }
}

/// MovementType 2 walks the addon's `path_id` (Creature::LoadCreaturesAddon
/// sets `m_path_id`; WaypointMovementGenerator::LoadPath reads it). Without
/// an addon path AzerothCore idles the spawn (ObjectMgr::LoadCreatureAddons).
fn patrol_path(addon: Option<&CreatureAddon>, world_data: &WorldData) -> Option<u32> {
    addon
        .map(|addon| addon.path_id)
        .filter(|&id| id != 0 && world_data.waypoints.contains_key(&id))
}

#[cfg(test)]
#[path = "zones_world_db_tests.rs"]
mod world_db_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_data::{CreatureModelInfo, CreatureSpawn, CreatureTemplate};
    use std::collections::HashMap;

    fn make_template(entry: u32, health_mod: f32, display_id: u32) -> CreatureTemplate {
        CreatureTemplate {
            entry,
            name: "Fixture wolf".into(),
            min_level: 1,
            max_level: 1,
            faction: 0,
            npc_flags: 0,
            speed_walk: 1.0,
            speed_run: 1.0,
            health_modifier: health_mod,
            rank: 0,
            experience_modifier: 1.0,
            gives_experience: true,
            models: vec![CreatureModelInfo { display_id }],
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
            ..Default::default()
        }
    }

    fn make_spawn(entry: u32, x: f32, y: f32, z: f32) -> CreatureSpawn {
        CreatureSpawn {
            guid: 1,
            template_entry: entry,
            map: 0,
            zone_id: 0,
            position_x: x,
            position_y: y,
            position_z: z,
            orientation: 0.0,
            wander_distance: 0.0,
            movement_type: 0,
            spawntimesecs: 120,
        }
    }

    #[test]
    fn authored_sqlite_orientation_reaches_spawned_npc_model_facing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("world.db");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE creature_template (entry INTEGER, minlevel INTEGER, maxlevel INTEGER,
                faction INTEGER, npcflag INTEGER, speed_walk REAL, HealthModifier REAL, name TEXT, rank INTEGER, exp INTEGER,
                ExperienceModifier REAL, type INTEGER, flags_extra INTEGER, speed_run REAL);
             CREATE TABLE creature_template_model (CreatureID INTEGER, Idx INTEGER,
                CreatureDisplayID INTEGER, DisplayScale REAL, Probability REAL);
             CREATE TABLE creature (guid INTEGER, id1 INTEGER, map INTEGER, zoneId INTEGER,
                position_x REAL, position_y REAL, position_z REAL, wander_distance REAL,
                MovementType INTEGER, orientation REAL, spawntimesecs INTEGER);
             CREATE TABLE game_event_creature (guid INTEGER, eventEntry INTEGER);
             CREATE TABLE waypoint_data (id INTEGER, point INTEGER, position_x REAL,
                position_y REAL, position_z REAL, orientation REAL, delay INTEGER,
                move_type INTEGER);
             CREATE TABLE creature_addon (guid INTEGER, path_id INTEGER, mount INTEGER,
                auras TEXT);
             CREATE TABLE creature_template_addon (entry INTEGER, path_id INTEGER,
                mount INTEGER, auras TEXT);
             CREATE TABLE creature_formations (leaderGUID INTEGER, memberGUID INTEGER,
                dist REAL, angle REAL, groupAI INTEGER);",
        )
        .unwrap();
        let angles = [
            0.0f32,
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::PI,
            3.0 * std::f32::consts::FRAC_PI_2,
            1.606,
        ];
        for (index, angle) in angles.iter().enumerate() {
            let entry = 100 + index as u32;
            conn.execute(
                "INSERT INTO creature_template VALUES (?1,1,1,1,0,1.0,1.0,'Loup — Écorché',0,0,1.0,1,0,1.14)",
                [entry],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO creature_template_model VALUES (?1,0,3167,1.0,1.0)",
                [entry],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO creature VALUES (?1,?1,0,0,-9000,-100,80,0,0,?2,120)",
                rusqlite::params![entry, angle],
            )
            .unwrap();
        }
        drop(conn);
        let data = crate::world_data::load_world_data(&path);
        let mut app = run_spawn_system(data);
        let mut npcs = app
            .world_mut()
            .query::<(&Npc, Option<&shared::components::Rotation>)>();
        let mut count = 0;
        for (npc, rotation) in npcs.iter(app.world()) {
            assert_eq!(serde_json::to_value(npc).unwrap()["name"], "Loup — Écorché");
            let rotation = rotation.expect("authored NPC orientation must be replicated");
            let angle = angles[(npc.template_id - 100) as usize];
            let expected = Vec3::new(angle.cos(), 0.0, -angle.sin());
            let model_forward =
                Quat::from_rotation_y(rotation.y - std::f32::consts::FRAC_PI_2) * Vec3::X;
            assert!((model_forward - expected).length() < 0.00001);
            let logical_forward = Vec3::new(rotation.y.sin(), 0.0, rotation.y.cos());
            assert!((logical_forward - expected).length() < 0.00001);
            count += 1;
        }
        assert_eq!(count, angles.len());
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
            .map(|(p, n, h, m, z)| (*p, n.clone(), *h, *m, *z))
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
    fn spawned_npc_replicates_template_min_level_and_no_powers() {
        let mut template = make_template(1, 1.0, 100);
        template.min_level = 7;
        let spawns = vec![make_spawn(1, -9000.0, -100.0, 50.0)];
        let mut app = run_spawn_system(make_world_data(vec![template], spawns));
        let mut query = app.world_mut().query::<(&UnitLevel, &UnitPowers)>();
        let (level, powers) = query.single(app.world()).unwrap();
        assert_eq!(*level, UnitLevel(7));
        assert!(powers.entries.is_empty());
    }

    #[test]
    fn spawned_npc_carries_template_faction() {
        let mut template = make_template(1, 1.0, 100);
        template.faction = 11;
        let spawns = vec![make_spawn(1, -9000.0, -100.0, 50.0)];
        let mut app = run_spawn_system(make_world_data(vec![template], spawns));
        let mut query = app.world_mut().query::<&UnitFactionTemplate>();
        assert_eq!(*query.single(app.world()).unwrap(), UnitFactionTemplate(11));
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
    #[should_panic(expected = "creature spawn 1 references missing template 99")]
    fn spawn_system_rejects_missing_authoritative_template() {
        drop(run_spawn_system(make_world_data(
            vec![],
            vec![make_spawn(99, -9000.0, 0.0, 0.0)],
        )));
    }
}
