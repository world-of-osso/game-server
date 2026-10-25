//! Patrols, formations and creature addons spawned from the imported world.db
//! (`cargo test -p server world_db_tests -- --ignored`).

use super::*;
use crate::creature_ai::CreatureAiPlugin;
use crate::world_data::{WaypointPath, load_world_data};
use bevy::time::TimeUpdateStrategy;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Retail spawns are TrinityCore guid + `RETAIL_GUID_OFFSET` (content_policy.py).
const RETAIL: u32 = 20_000_000;
/// Stormwind City Patroller (1976, TrinityCore guid 313806) walking path 25104480
/// (creature_addon.PathId), 14 points, 12 s and 20 s pauses at points 7 and 14,
/// spawntimesecs 300.
const PATROLLER: u32 = RETAIL + 313_806;
const PATROLLER_PATH: u32 = 25_104_480;
/// Stormwind Royal Guard (42218, TrinityCore guid 280010, path 2240080) leading
/// 280023 (5 yd at 90°) and 280024 (5 yd at 360°), groupAI 515.
const ROYAL_GUARD_LEADER: u32 = RETAIL + 280_010;
const ROYAL_GUARD_MEMBERS: [(u32, f32, f32); 2] = [
    (RETAIL + 280_023, 5.0, 90.0),
    (RETAIL + 280_024, 5.0, 360.0),
];
const ROYAL_GUARD_ESCORTS: [u32; 3] = [
    ROYAL_GUARD_LEADER,
    ROYAL_GUARD_MEMBERS[0].0,
    ROYAL_GUARD_MEMBERS[1].0,
];

fn world_db_path() -> PathBuf {
    crate::test_data::world_db_path()
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
    patrol_app_with(guids, |_| {})
}

/// `patrol_app` plus the NPC respawn timer.
fn respawning_patrol_app(guids: &'static [u32]) -> App {
    patrol_app_with(guids, |app| {
        app.init_resource::<crate::combat::CombatLog>();
        app.add_systems(FixedUpdate, crate::combat::respawn_system);
    })
}

fn patrol_app_with(guids: &'static [u32], extra: impl FnOnce(&mut App)) -> App {
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
    extra(&mut app);
    app.add_systems(
        PostStartup,
        |mut commands: Commands, world_data: Res<WorldData>| {
            let spawns = world_data
                .spawns
                .iter()
                .filter(|spawn| guids.contains(&spawn.guid));
            let spawned = spawn_creatures(
                &mut commands,
                &world_data,
                crate::maps::InMap::continent(0),
                0,
                false,
                spawns,
                Some(&mut crate::test_data::ground_map()),
            );
            commands.insert_resource(Spawned(spawned));
        },
    );
    app.update();
    app
}

#[test]
fn zephras_startup_spawns_imported_givers_on_the_hosted_continent() {
    let data = load_world_data(&world_db_path());
    let expected = data.spawns_by_map[&2991].len();
    assert_eq!(expected, 1662, "actual imported Zephras spawn fixture");
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).insert_resource(data);
    app.add_systems(
        PostStartup,
        |mut commands: Commands, data: Res<WorldData>| {
            let count = spawn_map_creatures(
                &mut commands,
                &data,
                MapContent::continent(2991),
                false,
                Some(&mut crate::test_data::ground_map()),
            );
            assert_eq!(count, 1662, "hosted Zephras must not open empty");
        },
    );
    app.update();
    let world = app.world_mut();
    let mut query = world.query::<(&Npc, &ModelDisplay, &Position, &crate::maps::InMap, &Health)>();
    for (entry, display, wow_position, estimated_health) in [
        (251362, 136968, (4088.500790, 1850.5, 976.173490), 42.0),
        (251487, 139694, (4051.417491, 1861.625, 977.620399), 102.0),
    ] {
        let giver = query
            .iter(world)
            .find(|(npc, _, _, _, _)| npc.template_id == entry)
            .expect("imported nearby quest giver spawned");
        assert_eq!(giver.1.display_id, display);
        assert_eq!(*giver.3, crate::maps::InMap::continent(2991));
        assert_eq!(giver.4.max, estimated_health);
        assert_eq!(giver.4.current, estimated_health);
        let (x, y, z) =
            crate::terrain_height::wow_to_bevy(wow_position.0, wow_position.1, wow_position.2);
        assert!((Vec3::new(giver.2.x, giver.2.y, giver.2.z) - Vec3::new(x, y, z)).length() < 0.01);
    }
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
fn stormwind_patroller_walks_addon_path_in_order_pausing_at_delays() {
    let spawn = spawn_of(PATROLLER);
    assert_eq!(spawn.movement_type, 2);
    assert_eq!(
        world_data().addon_for(spawn).map(|addon| addon.path_id),
        Some(PATROLLER_PATH)
    );
    assert!(
        !world_data().waypoints.contains_key(&PATROLLER),
        "the path is keyed by path id, not guid"
    );
    let path = &world_data().waypoints[&PATROLLER_PATH];
    assert_eq!(path.points.len(), 14);

    let mut app = patrol_app(&[PATROLLER]);
    let guard = entity_of(&app, spawn.guid);
    assert_eq!(
        app.world().get::<WaypointState>(guard).unwrap().path_id,
        PATROLLER_PATH
    );

    let mut reached = Vec::new();
    let mut pauses = Vec::new();
    // Point being waited at and when the patroller arrived there.
    let mut waiting: Option<(usize, f32)> = None;
    for _ in 0..40_000 {
        app.update();
        let now = arrival(&app, guard);
        if waiting.map(|(index, _)| index) != now {
            if let Some((_, at)) = waiting.take() {
                pauses.push(fixed_elapsed(&app) - at);
            }
            if let Some(index) = now {
                let pos = *app.world().get::<Position>(guard).unwrap();
                let (px, pz) = point_bevy(path, index);
                assert!((pos.x - px).abs() < 0.001 && (pos.z - pz).abs() < 0.001);
                reached.push(index);
                waiting = Some((index, fixed_elapsed(&app)));
            }
        }
        if pauses.len() == 15 {
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
    assert_eq!(pauses.iter().filter(|pause| **pause > 1.5).count(), 2);
}

#[test]
#[ignore]
fn royal_guard_escorts_hold_their_offsets_along_the_patrol() {
    let leader_spawn = spawn_of(ROYAL_GUARD_LEADER);
    let path_id = world_data().addon_for(leader_spawn).unwrap().path_id;
    let path = &world_data().waypoints[&path_id];
    for (guid, dist, angle) in ROYAL_GUARD_MEMBERS {
        let info = &world_data().formations[&guid];
        assert_eq!(info.leader_guid, ROYAL_GUARD_LEADER);
        assert_eq!(info.follow_dist, dist);
        assert!((info.follow_angle - angle.to_radians()).abs() < 1e-6);
    }

    let mut app = patrol_app(&ROYAL_GUARD_ESCORTS);
    let leader = entity_of(&app, ROYAL_GUARD_LEADER);
    let members: Vec<(Entity, f32, f32)> = ROYAL_GUARD_MEMBERS
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
    let mut last_arrival = None;
    let mut first_member_positions = HashSet::new();
    for _ in 0..20_000 {
        app.update();
        let now = arrival(&app, leader);
        let first = *app.world().get::<Position>(members[0].0).unwrap();
        first_member_positions.insert((first.x.to_bits(), first.z.to_bits()));
        if let Some(index) = now.filter(|_| now != last_arrival) {
            let point = &path.points[index];
            let heading = (point.position_y - origin.1).atan2(point.position_x - origin.0);
            // A zero-length leg (point 1 on the spawn, a pause node repeating the
            // previous point) has no heading; members keep their previous slots.
            let moved = (point.position_x - origin.0).hypot(point.position_y - origin.1) > 0.01;
            for &(member, dist, angle) in members.iter().filter(|_| moved) {
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
        last_arrival = now;
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
    // TrinityCore 313610 Stormwind Harbor Guard: creature_addon.mount 2410.
    // TrinityCore 313914 General Marcus Jonathan: no spawn row; creature_template_addon.mount 2410.
    const HARBOR_GUARD: u32 = RETAIL + 313_610;
    const MARCUS: u32 = RETAIL + 313_914;
    let harbor_guard = spawn_of(HARBOR_GUARD);
    let marcus = spawn_of(MARCUS);
    assert!(!world_data().addons.contains_key(&MARCUS));
    let app = patrol_app(&[HARBOR_GUARD, MARCUS, PATROLLER]);
    for (spawn, mount) in [(harbor_guard, Some(2410)), (marcus, Some(2410))] {
        let entity = entity_of(&app, spawn.guid);
        assert_eq!(
            app.world()
                .get::<Mounted>(entity)
                .map(|mounted| mounted.mount_display_id),
            mount
        );
    }
    let guard = entity_of(&app, PATROLLER);
    assert!(app.world().get::<Mounted>(guard).is_none());
}

#[test]
#[ignore]
fn cenarion_hold_infantry_carries_its_addon_aura() {
    // TrinityCore 42766 Cenarion Hold Infantry (15184), Silithus: creature_addon.auras 18950.
    const INFANTRY: u32 = RETAIL + 42_766;
    let spawn = spawn_of(INFANTRY);
    assert_eq!(world_data().addon_for(spawn).unwrap().auras, vec![18950]);
    let conn = rusqlite::Connection::open_with_flags(
        world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let (store, _) = crate::spell_info::load_spell_data(&conn).unwrap();
    let factions = crate::faction::load_faction_templates(&conn).unwrap();
    let mut app = patrol_app(&[INFANTRY]);
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

/// Run until the patroller has newly arrived at `count` points.
fn run_arrivals(app: &mut App, patroller: Entity, count: usize) {
    let mut arrivals = 0;
    let mut last = arrival(app, patroller);
    for _ in 0..20_000 {
        app.update();
        let now = arrival(app, patroller);
        if now.is_some() && now != last {
            arrivals += 1;
            if arrivals == count {
                return;
            }
        }
        last = now;
    }
    panic!("only {arrivals} of {count} arrivals");
}

/// The point the patroller has arrived at, if any. TrinityCore paths have
/// zero-length legs (point 1 on the spawn, a pause node repeating the previous
/// point), across which `arrived` stays set while `current_index` advances.
fn arrival(app: &App, patroller: Entity) -> Option<usize> {
    let state = app.world().get::<WaypointState>(patroller).unwrap();
    state.arrived.then_some(state.current_index)
}

fn position(app: &App, entity: Entity) -> Position {
    *app.world().get::<Position>(entity).unwrap()
}

/// Kill through the combat death path (real `RespawnDelay` timer).
fn kill(app: &mut App, entity: Entity) {
    app.world_mut().get_mut::<Health>(entity).unwrap().current = 0.0;
    crate::combat::mark_npc_dead_world(app.world_mut(), entity);
}

fn is_dead(app: &App, entity: Entity) -> bool {
    app.world()
        .get::<crate::combat::DeathState>(entity)
        .is_some()
}

fn gap(app: &App, a: Entity, b: Entity) -> f32 {
    let (a, b) = (position(app, a), position(app, b));
    (a.x - b.x).hypot(a.z - b.z)
}

#[test]
#[ignore]
fn killed_patroller_lies_still_then_respawns_at_spawn_and_restarts_its_path() {
    let spawn = spawn_of(PATROLLER);
    let (sx, sy, sz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    let mut app = respawning_patrol_app(&[PATROLLER]);
    let guard = entity_of(&app, PATROLLER);
    run_arrivals(&mut app, guard, 4);
    // Mid-leg toward point 5.
    for _ in 0..10 {
        app.update();
    }
    assert_eq!(
        app.world()
            .get::<WaypointState>(guard)
            .unwrap()
            .current_index,
        4
    );
    kill(&mut app, guard);
    assert_eq!(
        app.world()
            .get::<crate::combat::DeathState>(guard)
            .unwrap()
            .respawn_timer,
        360.0,
        "spawntimesecs 300 + 60 s normal corpse decay"
    );
    let corpse = position(&app, guard);
    for _ in 0..30 {
        app.update();
        assert_eq!(
            position(&app, guard),
            corpse,
            "a dead patroller does not move"
        );
    }

    while app
        .world()
        .get::<crate::combat::DeathState>(guard)
        .is_some()
    {
        app.update();
    }
    let state = app.world().get::<WaypointState>(guard).unwrap();
    assert_eq!(state.current_index, 0, "path restarted at point 1");
    assert_eq!(state.leg_origin, (sx, sz));
    let respawned = position(&app, guard);
    assert_eq!((respawned.x, respawned.y, respawned.z), (sx, sy, sz));

    // Point 1 is the spawn: the respawned patroller is on it at once, then walks on.
    assert_eq!(arrival(&app, guard), Some(0));
    run_arrivals(&mut app, guard, 1);
    assert_eq!(
        app.world()
            .get::<WaypointState>(guard)
            .unwrap()
            .current_index,
        1,
        "then walks on to point 2, not the point it died heading for"
    );
}

#[test]
#[ignore]
fn killed_idle_northshire_guard_respawns_at_its_spawn_after_its_timer() {
    // TrinityCore 280021: MovementType 0 Northshire Guard, spawntimesecs 120, normal rank.
    const IDLE_GUARD: u32 = RETAIL + 280_021;
    let spawn = spawn_of(IDLE_GUARD);
    assert_eq!(spawn.movement_type, 0);
    let (sx, sy, sz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    let mut app = respawning_patrol_app(&[IDLE_GUARD]);
    let guard = entity_of(&app, IDLE_GUARD);
    *app.world_mut().get_mut::<Position>(guard).unwrap() = Position {
        x: sx + 5.0,
        y: sy,
        z: sz,
    };
    kill(&mut app, guard);
    // 180 s is 3,600 ticks at 20 Hz.
    for _ in 0..3_590 {
        app.update();
    }
    assert!(is_dead(&app, guard), "still dead at 179.5 s");
    for _ in 0..20 {
        app.update();
    }
    assert!(!is_dead(&app, guard), "respawned by 180.5 s");
    let health = *app.world().get::<Health>(guard).unwrap();
    assert_eq!(health.current, health.max);
    let respawned = position(&app, guard);
    assert_eq!((respawned.x, respawned.y, respawned.z), (sx, sy, sz));
}

#[test]
#[ignore]
fn dead_escort_holds_still_then_respawns_and_rejoins_its_leader() {
    let mut app = respawning_patrol_app(&ROYAL_GUARD_ESCORTS);
    let leader = entity_of(&app, ROYAL_GUARD_LEADER);
    let (dead_escort, escort) = (
        entity_of(&app, ROYAL_GUARD_ESCORTS[1]),
        entity_of(&app, ROYAL_GUARD_ESCORTS[2]),
    );
    run_arrivals(&mut app, leader, 3);

    kill(&mut app, dead_escort);
    let corpse = position(&app, dead_escort);
    let escort_before = position(&app, escort);
    run_arrivals(&mut app, leader, 2);
    assert_eq!(
        position(&app, dead_escort),
        corpse,
        "dead escort left behind"
    );
    assert_ne!(
        position(&app, escort),
        escort_before,
        "live escort kept following"
    );

    while is_dead(&app, dead_escort) {
        app.update();
    }
    let spawn = spawn_of(ROYAL_GUARD_ESCORTS[1]);
    let respawned = position(&app, dead_escort);
    let (sx, _, sz) = wow_to_bevy_xz(spawn.position_x, spawn.position_y);
    assert!(
        (respawned.x - sx).hypot(respawned.z - sz) < 1.0,
        "respawned at its own spawn"
    );
    run_arrivals(&mut app, leader, 2);
    assert!(
        (gap(&app, dead_escort, leader) - 5.0).abs() < 0.01,
        "respawned escort back on its 5 yd slot"
    );
}

#[test]
#[ignore]
fn escorts_hold_still_while_their_leader_is_dead_then_rejoin_it() {
    let mut app = respawning_patrol_app(&ROYAL_GUARD_ESCORTS);
    let leader = entity_of(&app, ROYAL_GUARD_LEADER);
    let escorts = [
        entity_of(&app, ROYAL_GUARD_ESCORTS[1]),
        entity_of(&app, ROYAL_GUARD_ESCORTS[2]),
    ];
    run_arrivals(&mut app, leader, 3);
    for _ in 0..5 {
        app.update();
    }

    kill(&mut app, leader);
    let leader_corpse = position(&app, leader);
    let stopped = escorts.map(|escort| position(&app, escort));
    loop {
        app.update();
        if !is_dead(&app, leader) {
            break;
        }
        assert_eq!(position(&app, leader), leader_corpse);
        assert_eq!(
            escorts.map(|escort| position(&app, escort)),
            stopped,
            "a dead leader drags no one"
        );
    }
    let state = app.world().get::<WaypointState>(leader).unwrap();
    assert_eq!(state.current_index, 0, "respawned leader restarts its path");

    run_arrivals(&mut app, leader, 2);
    for escort in escorts {
        assert!(
            (gap(&app, escort, leader) - 5.0).abs() < 0.01,
            "escort rejoined its respawned leader"
        );
    }
}

/// Stormwind's Retail Training Dummy (153292, TrinityCore guid 850239) and PvP
/// Training Dummy (114832, 850247), TrinityCore-only templates with ScriptName
/// `npc_training_dummy`, MovementType 0.
const STORMWIND_DUMMIES: [u32; 2] = [RETAIL + 850_239, RETAIL + 850_247];

#[test]
#[ignore]
fn stormwind_training_dummies_get_their_script_and_stand_still() {
    let mut app = patrol_app(&[STORMWIND_DUMMIES[0], STORMWIND_DUMMIES[1], PATROLLER]);
    let dummies = STORMWIND_DUMMIES.map(|guid| entity_of(&app, guid));
    let guard = entity_of(&app, PATROLLER);
    for dummy in dummies {
        assert!(
            app.world()
                .get::<crate::creature_scripts::TrainingDummy>(dummy)
                .is_some()
        );
    }
    assert!(
        app.world()
            .get::<crate::creature_scripts::TrainingDummy>(guard)
            .is_none()
    );

    let placed = dummies.map(|dummy| position(&app, dummy));
    for _ in 0..200 {
        app.update();
    }
    assert_eq!(dummies.map(|dummy| position(&app, dummy)), placed);
}

/// Giant Moss Creeper (2349), Hillsbrad: AzerothCore display 6808 is Wrath-only;
/// TrinityCore's Retail model for the entry is 520.
const GIANT_MOSS_CREEPER: [u32; 1] = [RETAIL + 323_400];

#[test]
#[ignore]
fn wrath_only_display_spawns_with_its_retail_display() {
    let app = patrol_app(&GIANT_MOSS_CREEPER);
    let creeper = entity_of(&app, GIANT_MOSS_CREEPER[0]);
    let display = app.world().get::<ModelDisplay>(creeper).unwrap();
    assert_eq!(display.display_id, 520);
}

#[test]
#[ignore]
fn every_eastern_kingdoms_spawn_display_exists_in_retail() {
    let conn = rusqlite::Connection::open_with_flags(
        world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let retail: HashSet<u32> = conn
        .prepare("SELECT ID FROM creature_display_info")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let data = world_data();
    let absent: HashSet<u32> = data.spawns_by_map[&0]
        .iter()
        .flat_map(|&index| {
            let spawn = &data.spawns[index];
            let models = &data.templates[&spawn.template_entry].models;
            spawn
                .display_id
                .into_iter()
                .chain(models.iter().map(|model| model.display_id))
        })
        .filter(|display| !retail.contains(display))
        .collect();
    assert!(
        absent.is_empty(),
        "map 0 displays absent from Retail: {absent:?}"
    );
}

/// Diseased Young Wolf (299, ContentTuning 73: levels 1-30, warrior,
/// HealthScalingExpansion 0, HealthModifier 1).
const DISEASED_YOUNG_WOLF: u32 = RETAIL + 280_052;
/// Hogger (448, ContentTuning 73, HealthModifier 3).
const HOGGER: u32 = RETAIL + 280_348;
/// Stormwind City Guard (68, ContentTuning 883: level 90, HealthScalingExpansion 11,
/// warrior, HealthModifier 1.5).
const STORMWIND_CITY_GUARD: u32 = RETAIL + 313_646;

/// Creature::SelectLevel and GetMaxHealthByLevel: a tuned creature is its
/// ContentTuning maximum, with ceil(ExpectedStat CreatureHealth × HealthModifier).
#[test]
#[ignore]
fn tuned_creature_health_is_expected_stat_health_at_its_range_maximum() {
    let app = patrol_app(&[
        STORMWIND_DUMMIES[0],
        STORMWIND_DUMMIES[1],
        DISEASED_YOUNG_WOLF,
        HOGGER,
        STORMWIND_CITY_GUARD,
    ]);
    let stats = |guid| {
        let entity = entity_of(&app, guid);
        let health = app.world().get::<Health>(entity).unwrap();
        let level = app.world().get::<UnitLevel>(entity).unwrap();
        let scaling = app.world().get::<LevelScaling>(entity).unwrap();
        (
            health.current,
            health.max,
            level.0,
            (scaling.min_level, scaling.max_level, scaling.delta),
        )
    };
    // ExpectedStat level 90 (ExpansionID -2, no Midnight row): float32 141,482.015625,
    // so ceil(× 1.5) is 212,224.
    assert_eq!(
        stats(STORMWIND_CITY_GUARD),
        (212_224.0, 212_224.0, 90, (90, 90, 0))
    );
    // ContentTuning 181 and 864 (levels 1-90), HealthModifier 25 and 8.
    assert_eq!(
        stats(STORMWIND_DUMMIES[0]),
        (3_537_051.0, 3_537_051.0, 90, (1, 90, 0))
    );
    assert_eq!(
        stats(STORMWIND_DUMMIES[1]),
        (1_131_857.0, 1_131_857.0, 90, (1, 90, 0))
    );
    // ExpectedStat level 30: 4,378.775.
    assert_eq!(
        stats(DISEASED_YOUNG_WOLF),
        (4_379.0, 4_379.0, 30, (1, 30, 0))
    );
    assert_eq!(stats(HOGGER), (13_137.0, 13_137.0, 30, (1, 30, 0)));
}

/// Spawn actual continent rows on their own maps, through the same difficulty-0
/// template selection as startup. The returned keys are creature entries.
fn spawn_scaling_entries(entries: &[u32]) -> (World, HashMap<u32, Entity>) {
    let data = world_data();
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let mut entities = HashMap::new();
    for map_id in [0, 1] {
        let spawns: Vec<_> = entries
            .iter()
            .filter_map(|entry| {
                data.spawns
                    .iter()
                    .find(|spawn| spawn.template_entry == *entry && u32::from(spawn.map) == map_id)
            })
            .collect();
        let spawned = spawn_creatures(
            &mut commands,
            data,
            crate::maps::InMap::continent(map_id),
            0,
            false,
            spawns.iter().copied(),
            Some(&mut crate::test_data::ground_map()),
        );
        for spawn in spawns {
            entities.insert(spawn.template_entry, spawned[&spawn.guid]);
        }
    }
    queue.apply(&mut world);
    assert_eq!(
        entities.len(),
        entries.len(),
        "every entry must have a spawn"
    );
    (world, entities)
}

/// Assert observable level, health and lethal damage against a real player
/// entity, not just the template's imported level columns.
fn assert_scaled_spawn(world: &mut World, creature: Entity, player_level: u8, expected: (u8, f32)) {
    let player = world.spawn(UnitLevel(player_level)).id();
    let attacker = crate::level_scaling::UnitScaling::of(world, player);
    let victim = crate::level_scaling::UnitScaling::of(world, creature);
    let level = victim.level_against(&attacker);
    let stats = world.get::<ScaledStats>(creature).unwrap();
    let effective_health = stats.0.max_health(level);
    assert_eq!((level, effective_health), expected);
    let health = *world.get::<Health>(creature).unwrap();
    assert_eq!(health.current, health.max);
    // A hit equal to effective max health must kill the shared native pool.
    let loss = crate::level_scaling::health_loss(&attacker, &victim, effective_health);
    let mut pool = world.get_mut::<Health>(creature).unwrap();
    assert!(crate::death::apply_health_damage(&mut pool, loss));
    assert_eq!(pool.current, 0.0);
    world.despawn(player);
}

#[test]
#[ignore = "requires the imported world.db"]
fn creaturelevel_northshire_spawns_are_level_one_with_78_effective_health() {
    // CT 73 is 1-30; ExpectedStat(-2, 1) = 78, class mod 4 = 1,
    // HealthModifier = 1 for all three. Native health remains 4,379.
    let entries = [49874, 49871, 299];
    let (mut world, entities) = spawn_scaling_entries(&entries);
    for entry in entries {
        let creature = entities[&entry];
        let scaling = world.get::<LevelScaling>(creature).unwrap();
        assert_eq!((scaling.min_level, scaling.max_level), (1, 30));
        assert_eq!(world.get::<UnitLevel>(creature).unwrap().0, 30);
        assert_eq!(world.get::<Health>(creature).unwrap().max, 4_379.0);
        assert_scaled_spawn(&mut world, creature, 1, (1, 78.0));
    }
}

#[test]
#[ignore = "requires the imported world.db"]
fn creaturelevel_elwynn_and_durotar_spawns_scale_to_level_one() {
    // Elwynn: Stonetusk Boar, Mangy Wolf (CT 73). Durotar: Elder Mottled
    // Boar, Vile Familiar, Felstalker, Razormane Quilboar (CT 70).
    let entries = [113, 525, 3100, 3101, 3102, 3111];
    let (mut world, entities) = spawn_scaling_entries(&entries);
    for entry in entries {
        assert_scaled_spawn(&mut world, entities[&entry], 1, (1, 78.0));
    }
}

#[test]
#[ignore = "requires the imported world.db"]
fn creaturelevel_higher_tuning_preserves_floor_and_player_scaling() {
    // Flesh Eater (3), Duskwood, CT 13: 10-30. ExpectedStat(-2, 10)
    // = 377.3422546 and (-2, 30) = 4378.7749023; modifiers = 1.
    for (player_level, expected) in [(1, (10, 378.0)), (10, (10, 378.0)), (40, (30, 4_379.0))] {
        let (mut world, entities) = spawn_scaling_entries(&[3]);
        assert_scaled_spawn(&mut world, entities[&3], player_level, expected);
    }
}

/// Riverpaw Outrunner (478, level 9, faction 20), Diseased Young Wolf (299,
/// civilian), Forest Spider (30, level 5, faction 22) and Stormwind's Heroic
/// Training Dummy (31146).
const RIVERPAW_OUTRUNNER: u32 = RETAIL + 280_315;
const CIVILIAN_WOLF: u32 = RETAIL + 280_055;
const FOREST_SPIDER: u32 = RETAIL + 280_109;
const HEROIC_TRAINING_DUMMY: u32 = RETAIL + 850_240;

/// A level 5 Human 4 yards east of `guid`'s spawn point.
fn spawn_human_near(app: &mut App, guid: u32) -> Entity {
    let spawn = spawn_of(guid);
    let (x, y, z) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    app.world_mut()
        .spawn((
            Position { x: x + 4.0, y, z },
            shared::components::Player {
                name: "Anduin".into(),
                race: 1,
                class: 1,
                appearance: Default::default(),
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitLevel(5),
            UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
            crate::networking::CombatTarget::default(),
        ))
        .id()
}

#[test]
#[ignore]
fn world_spawns_aggro_by_template_hostile_attack_civilian_and_dummy_do_not() {
    let Some(templates) = crate::faction::world_db_faction_templates() else {
        return;
    };
    let mut app = patrol_app_with(
        &[
            RIVERPAW_OUTRUNNER,
            CIVILIAN_WOLF,
            FOREST_SPIDER,
            HEROIC_TRAINING_DUMMY,
        ],
        |app| {
            app.insert_resource(templates);
            app.add_plugins((
                crate::combat::CombatPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        },
    );
    let players: Vec<(u32, Entity)> = [
        RIVERPAW_OUTRUNNER,
        CIVILIAN_WOLF,
        FOREST_SPIDER,
        HEROIC_TRAINING_DUMMY,
    ]
    .into_iter()
    .map(|guid| (guid, spawn_human_near(&mut app, guid)))
    .collect();
    for _ in 0..100 {
        app.update();
    }
    let state = |guid| {
        app.world()
            .get::<crate::creature_ai::CreatureState>(entity_of(&app, guid))
            .cloned()
    };
    let player_health = |guid: u32| {
        let player = players.iter().find(|(g, _)| *g == guid).unwrap().1;
        app.world().get::<Health>(player).unwrap().current
    };
    for guid in [RIVERPAW_OUTRUNNER, FOREST_SPIDER] {
        let player = players.iter().find(|(g, _)| *g == guid).unwrap().1;
        assert_eq!(
            state(guid),
            Some(crate::creature_ai::CreatureState::Pursuing {
                target: player.to_bits()
            }),
            "spawn {guid} aggroes the Human"
        );
        assert!(player_health(guid) < 100.0, "spawn {guid} hits the Human");
    }
    assert_eq!(
        state(CIVILIAN_WOLF),
        Some(crate::creature_ai::CreatureState::Idle),
        "civilian wolf does not aggro on sight"
    );
    assert_eq!(player_health(CIVILIAN_WOLF), 100.0);
    assert_eq!(state(HEROIC_TRAINING_DUMMY), None, "dummy has no combat AI");
    assert_eq!(player_health(HEROIC_TRAINING_DUMMY), 100.0);
}

fn spawns_of(entry: u32) -> Vec<&'static CreatureSpawn> {
    world_data()
        .spawns
        .iter()
        .filter(|spawn| spawn.template_entry == entry)
        .collect()
}

/// Newton Burnside (2456) behind the Retail bank counter, not AzerothCore's
/// (-8935.27, 613.14), now street outside the bank; Auctioneer Jaxon (15659)
/// above the Retail auction house floor (98.06), not AzerothCore's z 96.68.
#[test]
#[ignore]
fn stormwind_banker_and_auctioneer_stand_at_retail_positions() {
    let newton = spawns_of(2456);
    assert_eq!(newton.len(), 1);
    assert_eq!(
        (newton[0].guid, newton[0].position_x, newton[0].position_y),
        (RETAIL + 314_008, -8908.35, 639.24)
    );
    let jaxon = spawns_of(15659);
    assert_eq!(jaxon.len(), 1);
    assert_eq!(jaxon[0].guid, RETAIL + 314_060);
    assert!(jaxon[0].position_z > 98.06, "z {}", jaxon[0].position_z);
    assert!(
        world_data().spawns_by_map[&0]
            .iter()
            .chain(&world_data().spawns_by_map[&1])
            .all(|&index| world_data().spawns[index].guid > RETAIL),
        "maps 0 and 1 spawn only Retail spawns"
    );
}

/// Homeless Stormwind Citizen (42384) has no AzerothCore template: TrinityCore's
/// template, its Retail models, and its ContentTuning level range.
#[test]
#[ignore]
fn retail_only_creature_spawns_from_its_trinitycore_template() {
    let citizens = spawns_of(42384);
    // 71 TrinityCore spawns in PhaseId 0, 2 in DEFAULT_PHASE 169.
    assert_eq!(citizens.len(), 73);
    let template = &world_data().templates[&42384];
    assert_eq!(template.name, "Homeless Stormwind Citizen");
    assert_eq!(
        (template.min_level, template.max_level),
        (5, 30),
        "ContentTuning 6"
    );
    assert!(!template.models.is_empty());
    const CITIZEN: u32 = RETAIL + 224_152;
    assert!(citizens.iter().any(|spawn| spawn.guid == CITIZEN));
    let app = patrol_app(&[CITIZEN]);
    let display = app
        .world()
        .get::<ModelDisplay>(entity_of(&app, CITIZEN))
        .unwrap();
    assert!(
        template
            .models
            .iter()
            .any(|model| model.display_id == display.display_id)
    );
}

/// Creature::UpdateLevelDependantStats (Creature.cpp:1665-1666): a creature's
/// armor is ExpectedStat CreatureArmor at its own level × ArmorModifier; the
/// ArmorConstant a level 10 attacker meets is the level 10 ExpansionID -2 row.
#[test]
#[ignore]
fn spawned_creatures_carry_expected_stat_armor() {
    let app = patrol_app(&[HOGGER]);
    let hogger = entity_of(&app, HOGGER);
    // Level 30, CreatureArmor 155, warrior ExpectedStatMod 4 × 1, ArmorModifier 1.
    assert_eq!(
        app.world().get::<CreatureArmor>(hogger),
        Some(&CreatureArmor(155.0))
    );
    assert_eq!(world_data().armor_constants.at(10), Some(210.0));
    assert_eq!(world_data().armor_constants.at(30), Some(361.0));
}

/// Summon Enabler Stalker (53488): models 169 (Probability 0) and 16925 (the invisible
/// stalker, model file 124642), `CREATURE_FLAG_EXTRA_TRIGGER`, `UNIT_FLAG_NOT_SELECTABLE`.
/// TrinityCore guid 276318 in Stormwind has no modelid; Stockade guid 375809 has modelid 16925.
const STALKER_STORMWIND: u32 = RETAIL + 276_318;
const STALKER_STOCKADE: u32 = RETAIL + 375_809;
const INVISIBLE_STALKER_DISPLAY: u32 = 16_925;

/// ObjectMgr::ChooseDisplayId: a trigger takes its first invisible model; a spawn's modelid wins
/// (ChooseCreatureFlags: the template's unit flags where the spawn sets none).
#[test]
#[ignore]
fn summon_enabler_stalker_spawns_invisible_and_not_selectable() {
    assert_eq!(spawn_of(STALKER_STORMWIND).display_id, None);
    assert_eq!(
        spawn_of(STALKER_STOCKADE).display_id,
        Some(INVISIBLE_STALKER_DISPLAY)
    );
    let app = patrol_app(&[STALKER_STORMWIND, STALKER_STOCKADE]);
    for guid in [STALKER_STORMWIND, STALKER_STOCKADE] {
        let stalker = entity_of(&app, guid);
        assert_eq!(
            app.world().get::<ModelDisplay>(stalker),
            Some(&ModelDisplay {
                display_id: INVISIBLE_STALKER_DISPLAY
            }),
            "guid {guid}"
        );
        let flags = app.world().get::<UnitFlags>(stalker).expect("unit flags");
        assert!(!flags.is_selectable(), "guid {guid}: {flags:?}");
    }
}

/// How often each display of `entry` is chosen over 4000 spawns without a modelid (TDB1210
/// gives every Stockade spawn one; summons and continent spawns mostly have none).
fn display_counts(entry: u32) -> HashMap<u32, u32> {
    let template = &world_data().templates[&entry];
    let spawn = CreatureSpawn {
        guid: 1,
        template_entry: entry,
        map: 34,
        zone_id: 717,
        position_x: 0.0,
        position_y: 0.0,
        position_z: 0.0,
        orientation: 0.0,
        wander_distance: 0.0,
        movement_type: 0,
        spawntimesecs: 120,
        display_id: None,
        unit_flags: None,
        spawn_difficulties: vec![0],
        equipment_id: 0,
    };
    let spawn = &spawn;
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(34);
    let mut counts = HashMap::new();
    for _ in 0..4000 {
        *counts
            .entry(choose_display(template, spawn, &mut rng).0)
            .or_default() += 1;
    }
    counts
}

/// CreatureTemplate::GetRandomValidModel: the Stockade Guard (46405) takes 2989, 2990 or 2992
/// (Probability 1 each), never 2991 (Probability 0); Rifle Commander Coe (46417) always takes
/// 2991, his only model with a weight, not his first (2989).
#[test]
#[ignore]
fn stockade_models_are_chosen_by_probability() {
    let guard = display_counts(46_405);
    assert_eq!(
        guard.keys().copied().collect::<HashSet<_>>(),
        HashSet::from([2989, 2990, 2992]),
        "{guard:?}"
    );
    for count in guard.values() {
        assert!((1200..1470).contains(count), "{guard:?}");
    }
    assert_eq!(display_counts(46_417), HashMap::from([(2991, 4000)]));
}

/// SelectRandomWeightedContainerElement: with every weight 0 (Petty Criminal 46382: 35069, 35070)
/// each model is equally likely.
#[test]
#[ignore]
fn models_without_weights_are_chosen_uniformly() {
    let criminal = display_counts(46_382);
    assert_eq!(criminal.len(), 2, "{criminal:?}");
    for display in [35069, 35070] {
        assert!((1850..2150).contains(&criminal[&display]), "{criminal:?}");
    }
}

/// Stockade TDB1210 creature_addon / creature_template_addon poses (Creature::LoadCreaturesAddon):
/// guard 375782 `emote` 333 (EMOTE_STATE_READY1H), guard 375779 StandState 3 (sleep, with aura
/// 55474 Cosmetic - Sleep Zzz), guard 375776 has no spawn row and takes template 46405's
/// StandState 1 (sit); Petty Criminal 375706 takes template 46382's StandState 3, 375707 its spawn
/// row's 1, 375708 its spawn row's 0. Every row has SheathState 1 (melee weapons drawn).
#[test]
#[ignore]
fn stockade_spawns_take_their_addon_pose() {
    use shared::components::{SheathState, StandState, UnitPose};
    const POSES: [(u32, StandState, u32); 6] = [
        (RETAIL + 375_782, StandState::Stand, 333),
        (RETAIL + 375_779, StandState::Sleep, 0),
        (RETAIL + 375_776, StandState::Sit, 0),
        (RETAIL + 375_706, StandState::Sleep, 0),
        (RETAIL + 375_707, StandState::Sit, 0),
        (RETAIL + 375_708, StandState::Stand, 0),
    ];
    const GUIDS: [u32; 6] = [
        POSES[0].0, POSES[1].0, POSES[2].0, POSES[3].0, POSES[4].0, POSES[5].0,
    ];
    let app = patrol_app(&GUIDS);
    for (guid, stand_state, emote_state) in POSES {
        assert_eq!(
            app.world().get::<UnitPose>(entity_of(&app, guid)),
            Some(&UnitPose {
                stand_state,
                sheath_state: SheathState::Melee,
                emote_state,
            }),
            "guid {guid}"
        );
    }
}

/// Creature::LoadEquipment(creature.equipment_id): Stockade Guard 375782 (equipment_id 1) wields
/// TDB1210 creature_equip_template (46405, 1): item 5305 (ItemModifiedAppearance mod 0 ->
/// ItemAppearance 1844 -> ItemDisplayInfo 7526, a one-hand weapon) and shield 1984 (appearance
/// 337, display 1705); Petty Criminal 375706 (equipment_id 0) wields nothing.
#[test]
#[ignore]
fn stockade_guard_wields_its_virtual_items() {
    use shared::components::{EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};
    const GUARD: u32 = RETAIL + 375_782;
    const CRIMINAL: u32 = RETAIL + 375_706;
    let app = patrol_app(&[GUARD, CRIMINAL]);
    let entry = |slot, item_id, display_info_id, inventory_type| EquippedAppearanceEntry {
        slot,
        item_id: Some(item_id),
        definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
        display_info_id: Some(display_info_id),
        inventory_type,
        hidden: false,
    };
    assert_eq!(
        app.world()
            .get::<EquipmentAppearance>(entity_of(&app, GUARD)),
        Some(&EquipmentAppearance {
            entries: vec![
                entry(EquipmentVisualSlot::MainHand, 5305, 7526, 13),
                entry(EquipmentVisualSlot::OffHand, 1984, 1705, 14),
            ],
        })
    );
    assert_eq!(
        app.world()
            .get::<EquipmentAppearance>(entity_of(&app, CRIMINAL)),
        None
    );
}

/// Timber (1132, guid 305397, rank 4) and Hogger (448, rank 1) spawn with the
/// `CreatureClassification` of their `creature_template.rank`.
const TIMBER: u32 = RETAIL + 305_397;

#[test]
#[ignore]
fn timber_spawns_rare_and_hogger_elite() {
    let app = patrol_app(&[TIMBER, HOGGER]);
    let classification = |guid| {
        *app.world()
            .get::<shared::components::CreatureClassification>(entity_of(&app, guid))
            .expect("spawned creatures carry their classification")
    };
    assert_eq!(
        classification(TIMBER),
        shared::components::CreatureClassification::Rare
    );
    assert_eq!(
        classification(HOGGER),
        shared::components::CreatureClassification::Elite
    );
}
