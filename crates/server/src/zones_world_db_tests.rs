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
                false,
                spawns,
            );
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
        .map(|&index| resolve_display_id(data, data.spawns[index].template_entry))
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
