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

/// Run until the patroller has newly arrived at `count` points.
fn run_arrivals(app: &mut App, patroller: Entity, count: usize) {
    let arrived_now = |app: &App| app.world().get::<WaypointState>(patroller).unwrap().arrived;
    let mut arrivals = 0;
    let mut was_arrived = arrived_now(app);
    for _ in 0..20_000 {
        app.update();
        let arrived = arrived_now(app);
        if arrived && !was_arrived {
            arrivals += 1;
            if arrivals == count {
                return;
            }
        }
        was_arrived = arrived;
    }
    panic!("only {arrivals} of {count} arrivals");
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
fn killed_northshire_guard_lies_still_then_respawns_at_spawn_and_restarts_its_path() {
    let spawn = spawn_of(NORTHSHIRE_GUARD);
    let (sx, sy, sz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    let mut app = respawning_patrol_app(&[NORTHSHIRE_GUARD]);
    let guard = entity_of(&app, NORTHSHIRE_GUARD);
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
        490.0,
        "spawntimesecs 430 + 60 s normal corpse decay"
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

    // The guard spawns on point 1: it arrives there first, then walks on.
    run_arrivals(&mut app, guard, 1);
    assert_eq!(
        app.world()
            .get::<WaypointState>(guard)
            .unwrap()
            .current_index,
        0
    );
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
    // 79926: MovementType 0 Northshire Guard, spawntimesecs 430, normal rank.
    let spawn = spawn_of(79926);
    assert_eq!(spawn.movement_type, 0);
    let (sx, sy, sz) =
        crate::terrain_height::wow_to_bevy(spawn.position_x, spawn.position_y, spawn.position_z);
    let mut app = respawning_patrol_app(&[79926]);
    let guard = entity_of(&app, 79926);
    *app.world_mut().get_mut::<Position>(guard).unwrap() = Position {
        x: sx + 5.0,
        y: sy,
        z: sz,
    };
    kill(&mut app, guard);
    // 490 s is 9,800 ticks at 20 Hz.
    for _ in 0..9_790 {
        app.update();
    }
    assert!(is_dead(&app, guard), "still dead at 489.5 s");
    for _ in 0..20 {
        app.update();
    }
    assert!(!is_dead(&app, guard), "respawned by 490.5 s");
    let health = *app.world().get::<Health>(guard).unwrap();
    assert_eq!(health.current, health.max);
    let respawned = position(&app, guard);
    assert_eq!((respawned.x, respawned.y, respawned.z), (sx, sy, sz));
}

#[test]
#[ignore]
fn dead_escort_holds_still_then_respawns_and_rejoins_its_leader() {
    let mut app = respawning_patrol_app(&[MOUNTAINEER_LEADER, 4479, 4480]);
    let leader = entity_of(&app, MOUNTAINEER_LEADER);
    let (dead_escort, escort) = (entity_of(&app, 4479), entity_of(&app, 4480));
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
    let spawn = spawn_of(4479);
    let respawned = position(&app, dead_escort);
    let (sx, _, sz) = wow_to_bevy_xz(spawn.position_x, spawn.position_y);
    assert!(
        (respawned.x - sx).hypot(respawned.z - sz) < 1.0,
        "respawned at its own spawn"
    );
    run_arrivals(&mut app, leader, 2);
    assert!(
        (gap(&app, dead_escort, leader) - 2.0).abs() < 0.01,
        "respawned escort back on its 2 yd slot"
    );
}

#[test]
#[ignore]
fn escorts_hold_still_while_their_leader_is_dead_then_rejoin_it() {
    let mut app = respawning_patrol_app(&[MOUNTAINEER_LEADER, 4479, 4480]);
    let leader = entity_of(&app, MOUNTAINEER_LEADER);
    let escorts = [entity_of(&app, 4479), entity_of(&app, 4480)];
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
            (gap(&app, escort, leader) - 2.0).abs() < 0.01,
            "escort rejoined its respawned leader"
        );
    }
}

/// Stormwind Grandmaster's (31144) and Expert's (32666) Training Dummies,
/// ScriptName `npc_training_dummy`, MovementType 0.
const STORMWIND_DUMMIES: [u32; 2] = [201235, 202726];

#[test]
#[ignore]
fn stormwind_training_dummies_get_their_script_and_stand_still() {
    let mut app = patrol_app(&[STORMWIND_DUMMIES[0], STORMWIND_DUMMIES[1], NORTHSHIRE_GUARD]);
    let dummies = STORMWIND_DUMMIES.map(|guid| entity_of(&app, guid));
    let guard = entity_of(&app, NORTHSHIRE_GUARD);
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
const GIANT_MOSS_CREEPER: [u32; 1] = [14583];

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

/// Stormwind City Guard (68, level 75 warrior, exp 2, HealthModifier 1.5) and
/// Diseased Young Wolf (299, level 1 warrior, exp 0, HealthModifier 1).
const STORMWIND_GUARD: u32 = 189;
const DISEASED_YOUNG_WOLF: u32 = 79648;

/// Creature::SelectLevel: ceil(creature_classlevelstats.basehp<exp> × HealthModifier).
#[test]
#[ignore]
fn creature_health_is_class_level_base_hp_times_health_modifier() {
    let app = patrol_app(&[
        STORMWIND_DUMMIES[0],
        STORMWIND_DUMMIES[1],
        STORMWIND_GUARD,
        DISEASED_YOUNG_WOLF,
    ]);
    let health = |guid| {
        let health = app.world().get::<Health>(entity_of(&app, guid)).unwrap();
        (health.current, health.max)
    };
    // 12600 (level 80 basehp2) × 0.000187 = 2.36.
    assert_eq!(health(STORMWIND_DUMMIES[0]), (3.0, 3.0));
    // 3052 (level 60 basehp0) × 0.000187 = 0.57.
    assert_eq!(health(STORMWIND_DUMMIES[1]), (1.0, 1.0));
    // 10635 (level 75 basehp2) × 1.5 = 15952.5.
    assert_eq!(health(STORMWIND_GUARD), (15953.0, 15953.0));
    assert_eq!(health(DISEASED_YOUNG_WOLF), (42.0, 42.0));
}

/// Riverpaw Outrunner (478, level 9, faction 20), Kobold Vermin (6, civilian),
/// Forest Spider (30, level 5, faction 22) and Heroic Training Dummy (31146).
const RIVERPAW_OUTRUNNER: u32 = 80474;
const KOBOLD_VERMIN: u32 = 79992;
const FOREST_SPIDER: u32 = 79883;
const HEROIC_TRAINING_DUMMY: u32 = 48325;

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
            KOBOLD_VERMIN,
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
        KOBOLD_VERMIN,
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
        state(KOBOLD_VERMIN),
        Some(crate::creature_ai::CreatureState::Idle),
        "civilian kobold does not aggro on sight"
    );
    assert_eq!(player_health(KOBOLD_VERMIN), 100.0);
    assert_eq!(state(HEROIC_TRAINING_DUMMY), None, "dummy has no combat AI");
    assert_eq!(player_health(HEROIC_TRAINING_DUMMY), 100.0);
}
