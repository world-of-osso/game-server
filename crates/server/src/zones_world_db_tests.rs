//! Patrols, formations and creature addons spawned from the imported world.db
//! (`cargo test -p server zones_world_db -- --ignored`).

use super::*;
use crate::creature_ai::CreatureAiPlugin;
use crate::world_data::{WaypointPath, load_world_data};
use bevy::time::TimeUpdateStrategy;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Northshire Abbey guard walking path 799270 (creature_addon.path_id), 14 points,
/// 2000 ms pauses at points 1 and 8.
const NORTHSHIRE_GUARD: u32 = 79927;
const NORTHSHIRE_GUARD_PATH: u32 = 799270;
/// Ironforge Mountaineer (path 44810) leading 4479 (2 yd at 240°) and 4480
/// (2 yd at 120°), groupAI 514 (FOLLOW_LEADER).
const MOUNTAINEER_LEADER: u32 = 4481;
const MOUNTAINEER_MEMBERS: [(u32, f32, f32); 2] = [(4479, 2.0, 240.0), (4480, 2.0, 120.0)];

fn world_db_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db")
}

fn world_data() -> &'static WorldData {
    static DATA: OnceLock<WorldData> = OnceLock::new();
    DATA.get_or_init(|| load_world_data(&world_db_path()))
}

/// Spawn guid -> entity for the spawns a `patrol_app` created.
#[derive(Resource)]
struct Spawned(HashMap<u32, Entity>);

/// App running the creature AI with only the spawns in `guids`.
fn patrol_app(guids: &'static [u32]) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    // One 20 Hz server tick per update, so every arrival is observed.
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(Time::<Fixed>::from_duration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(load_world_data(&world_db_path()));
    app.add_plugins(CreatureAiPlugin);
    app.add_systems(
        PostStartup,
        |mut commands: Commands, world_data: Res<WorldData>| {
            let spawns = world_data
                .spawns
                .iter()
                .filter(|spawn| guids.contains(&spawn.guid));
            let spawned = spawn_creatures(&mut commands, &world_data, false, spawns);
            commands.insert_resource(Spawned(spawned));
        },
    );
    app.update();
    app
}

fn entity_of(app: &App, guid: u32) -> Entity {
    app.world().resource::<Spawned>().0[&guid]
}

fn spawn_of(guid: u32) -> &'static CreatureSpawn {
    world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.guid == guid)
        .expect("spawn in world.db")
}

fn fixed_elapsed(app: &App) -> f32 {
    app.world().resource::<Time<Fixed>>().elapsed_secs()
}

fn point_bevy(path: &WaypointPath, index: usize) -> (f32, f32) {
    let point = &path.points[index];
    let (x, _, z) = wow_to_bevy_xz(point.position_x, point.position_y);
    (x, z)
}

fn wow_to_bevy_xz(x: f32, y: f32) -> (f32, f32, f32) {
    crate::terrain_height::wow_to_bevy(x, y, 0.0)
}

#[test]
#[ignore]
fn northshire_guard_walks_addon_path_in_order_pausing_at_delays() {
    let spawn = spawn_of(NORTHSHIRE_GUARD);
    assert_eq!(spawn.movement_type, 2);
    assert_eq!(
        world_data().addon_for(spawn).map(|addon| addon.path_id),
        Some(NORTHSHIRE_GUARD_PATH)
    );
    assert!(
        !world_data().waypoints.contains_key(&NORTHSHIRE_GUARD),
        "the path is keyed by path id, not guid"
    );
    let path = &world_data().waypoints[&NORTHSHIRE_GUARD_PATH];
    assert_eq!(path.points.len(), 14);

    let mut app = patrol_app(&[NORTHSHIRE_GUARD]);
    let guard = entity_of(&app, spawn.guid);
    assert_eq!(
        app.world().get::<WaypointState>(guard).unwrap().path_id,
        NORTHSHIRE_GUARD_PATH
    );

    let mut reached = Vec::new();
    let mut pauses = Vec::new();
    let mut arrived_at: Option<f32> = None;
    for _ in 0..20_000 {
        app.update();
        let state = app.world().get::<WaypointState>(guard).unwrap();
        let pos = *app.world().get::<Position>(guard).unwrap();
        match (arrived_at, state.arrived) {
            (None, true) => {
                let (px, pz) = point_bevy(path, state.current_index);
                assert!((pos.x - px).abs() < 0.001 && (pos.z - pz).abs() < 0.001);
                reached.push(state.current_index);
                arrived_at = Some(fixed_elapsed(&app));
            }
            (Some(at), false) => {
                pauses.push(fixed_elapsed(&app) - at);
                arrived_at = None;
            }
            _ => {}
        }
        if reached.len() == 15 && arrived_at.is_none() {
            break;
        }
    }
    let expected: Vec<usize> = (0..14).chain([0]).collect();
    assert_eq!(reached, expected, "points walked in order, then wrap");
    for (index, pause) in pauses.iter().enumerate() {
        let delay_secs = path.points[reached[index]].delay as f32 / 1000.0;
        assert!(
            (pause - delay_secs).abs() <= 0.051,
            "point {} paused {pause}s, expected {delay_secs}s",
            reached[index] + 1
        );
    }
    assert_eq!(pauses.iter().filter(|pause| **pause > 1.5).count(), 3);
}

#[test]
#[ignore]
fn ironforge_mountaineer_escorts_hold_their_offsets_along_the_patrol() {
    let leader_spawn = spawn_of(MOUNTAINEER_LEADER);
    let path_id = world_data().addon_for(leader_spawn).unwrap().path_id;
    let path = &world_data().waypoints[&path_id];
    for (guid, dist, angle) in MOUNTAINEER_MEMBERS {
        let info = &world_data().formations[&guid];
        assert_eq!(info.leader_guid, MOUNTAINEER_LEADER);
        assert_eq!(info.follow_dist, dist);
        assert!((info.follow_angle - angle.to_radians()).abs() < 1e-6);
    }

    let mut app = patrol_app(&[MOUNTAINEER_LEADER, 4479, 4480]);
    let leader = entity_of(&app, MOUNTAINEER_LEADER);
    let members: Vec<(Entity, f32, f32)> = MOUNTAINEER_MEMBERS
        .iter()
        .map(|&(guid, dist, angle)| {
            let entity = entity_of(&app, guid);
            assert_eq!(
                app.world().get::<FormationMember>(entity).unwrap().leader,
                leader
            );
            (entity, dist, angle.to_radians())
        })
        .collect();

    // WoW-space leg start: the spawn, then each reached point.
    let mut origin = (leader_spawn.position_x, leader_spawn.position_y);
    let mut arrivals = 0;
    let mut was_arrived = false;
    let mut first_member_positions = HashSet::new();
    for _ in 0..20_000 {
        app.update();
        let state = app.world().get::<WaypointState>(leader).unwrap();
        let (arrived, index) = (state.arrived, state.current_index);
        let first = *app.world().get::<Position>(members[0].0).unwrap();
        first_member_positions.insert((first.x.to_bits(), first.z.to_bits()));
        if arrived && !was_arrived {
            let point = &path.points[index];
            let heading = (point.position_y - origin.1).atan2(point.position_x - origin.0);
            for &(member, dist, angle) in &members {
                let pos = *app.world().get::<Position>(member).unwrap();
                let (ex, _, ez) = wow_to_bevy_xz(
                    point.position_x + (heading + angle).cos() * dist,
                    point.position_y + (heading + angle).sin() * dist,
                );
                assert!(
                    (pos.x - ex).abs() < 0.01 && (pos.z - ez).abs() < 0.01,
                    "member off its slot at point {}: ({}, {}) vs ({ex}, {ez})",
                    index + 1,
                    pos.x,
                    pos.z
                );
                let leader_pos = *app.world().get::<Position>(leader).unwrap();
                let gap = (pos.x - leader_pos.x).hypot(pos.z - leader_pos.z);
                assert!((gap - dist).abs() < 0.01, "member {gap} yd from leader");
            }
            origin = (point.position_x, point.position_y);
            arrivals += 1;
            if arrivals == 10 {
                break;
            }
        }
        was_arrived = arrived;
    }
    assert_eq!(arrivals, 10, "leader patrolled 10 points");
    assert!(
        first_member_positions.len() > 100,
        "member moved continuously with the leader"
    );
}

#[test]
#[ignore]
fn addon_mounts_spawn_mounted_from_spawn_and_template_rows() {
    // 120618 Stormwind Harbor Guard: creature_addon.mount 2410.
    // 79677 General Marcus Jonathan: no spawn row; creature_template_addon.mount 2410.
    let harbor_guard = spawn_of(120618);
    let marcus = spawn_of(79677);
    assert!(!world_data().addons.contains_key(&79677));
    let app = patrol_app(&[120618, 79677, NORTHSHIRE_GUARD]);
    for (spawn, mount) in [(harbor_guard, Some(2410)), (marcus, Some(2410))] {
        let entity = entity_of(&app, spawn.guid);
        assert_eq!(
            app.world()
                .get::<Mounted>(entity)
                .map(|mounted| mounted.mount_display_id),
            mount
        );
    }
    let guard = entity_of(&app, NORTHSHIRE_GUARD);
    assert!(app.world().get::<Mounted>(guard).is_none());
}

#[test]
#[ignore]
fn northshire_guard_carries_its_addon_aura() {
    let spawn = spawn_of(NORTHSHIRE_GUARD);
    assert_eq!(world_data().addon_for(spawn).unwrap().auras, vec![18950]);
    let conn = rusqlite::Connection::open_with_flags(
        world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (store, _) = crate::spell_info::load_spell_data(&conn).unwrap();
    let factions = crate::faction::load_faction_templates(&conn).unwrap();
    let mut app = patrol_app(&[NORTHSHIRE_GUARD]);
    app.insert_resource(store);
    app.insert_resource(factions);
    app.init_resource::<crate::auras::UnhandledAuras>();
    app.add_systems(FixedUpdate, apply_addon_auras);
    app.update();
    let guard = entity_of(&app, spawn.guid);
    let spells: Vec<u32> = app
        .world()
        .get::<crate::auras::ActiveAuras>(guard)
        .expect("aura components")
        .iter()
        .map(|aura| aura.spell_id)
        .collect();
    assert_eq!(spells, vec![18950], "Invisibility and Stealth Detection");
    assert!(app.world().get::<AddonAuras>(guard).is_none());
}
