use bevy::time::TimeUpdateStrategy;
use shared::components::Player;

use super::*;
use crate::creature_ai::{CreatureAiPlugin, WanderPhase, WanderState};
use crate::world_data::{WaypointPath, WaypointPoint};

/// WoW (x, y) of the wanderer's spawn: grid (0, 0).
const SPAWN: (f32, f32) = (100.0, 100.0);
/// WoW x of a player in grid (2, 0), two grids east of the spawn grid.
const TWO_GRIDS_AWAY_X: f32 = 1200.0;
/// WoW x of a player in grid (1, 0), next to the spawn grid.
const NEIGHBOUR_GRID_X: f32 = 700.0;
const PATROL_PATH: u32 = 77;

fn grid_app(waypoints: HashMap<u32, WaypointPath>) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(50),
    ));
    app.insert_resource(WorldData {
        waypoints,
        ..Default::default()
    });
    app.add_plugins((GridActivityPlugin, CreatureAiPlugin));
    app.update();
    app
}

fn bevy_position(wow_x: f32, wow_y: f32) -> Position {
    let (x, y, z) = wow_to_bevy(wow_x, wow_y, 0.0);
    Position { x, y, z }
}

fn spawn_point(position: Position) -> SpawnPoint {
    SpawnPoint {
        x: position.x,
        y: position.y,
        z: position.z,
        wander_distance: 5.0,
        speed_walk: 1.0,
        speed_run: 1.0,
    }
}

fn spawn_wanderer(app: &mut App) -> Entity {
    let position = bevy_position(SPAWN.0, SPAWN.1);
    app.world_mut()
        .spawn((
            position,
            InMap::continent(0),
            spawn_point(position),
            WanderState {
                phase: WanderPhase::Idle,
                target_x: position.x,
                target_z: position.z,
                idle_remaining: 0.0,
                seed: 3,
            },
        ))
        .id()
}

fn spawn_player(app: &mut App, wow_x: f32) -> Entity {
    app.world_mut()
        .spawn((
            InMap::continent(0),
            Player {
                name: "Walker".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
            bevy_position(wow_x, SPAWN.1),
        ))
        .id()
}

fn advance(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn position(app: &App, entity: Entity) -> Position {
    *app.world().get::<Position>(entity).unwrap()
}

fn dormant(app: &App, entity: Entity) -> bool {
    app.world().get::<GridDormant>(entity).is_some()
}

#[test]
fn cold_world_registration_applies_dormancy_without_rebuilding_creature_tables() {
    use bevy::ecs::system::{IntoSystem, System};
    use shared::components::{Health, Npc};
    use std::time::{Duration, Instant};
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GridActivityPlugin);
    app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    app.add_systems(
        PostStartup,
        |mut commands: Commands, data: Res<WorldData>| {
            for map in crate::maps::HOSTED_CONTINENTS {
                crate::zones::spawn_map_creatures(
                    &mut commands,
                    &data,
                    crate::zones::MapContent::continent(map),
                    false,
                );
            }
        },
    );
    app.world_mut().run_schedule(PostStartup);
    let world = app.world_mut();
    let population = world.query::<&Npc>().iter(world).count();
    assert!(
        population > 180_000,
        "cold proof needs the actual hosted population"
    );
    let mut static_creatures = world.query_filtered::<(Entity, &Position, &InMap, &Health),
        (With<Npc>, Without<WaypointState>, Without<FormationMember>)>();
    let (creature, position, map, health) = static_creatures.iter(world).next().unwrap();
    let (creature, position, map, health) = (creature, *position, *map, health.current);
    let mut registration = IntoSystem::into_system(register_creatures);
    registration.initialize(world);
    registration
        .run_without_applying_deferred((), world)
        .unwrap();
    let started = Instant::now();
    registration.apply_deferred(world);
    let elapsed = started.elapsed();
    let dormant_population = world
        .query_filtered::<&Npc, With<GridDormant>>()
        .iter(world)
        .count();
    assert_eq!(dormant_population, population);
    eprintln!("cold {population} creature dormancy application: {elapsed:?}");
    assert!(
        elapsed < Duration::from_millis(50),
        "dormancy registration rebuilt creature tables: {elapsed:?}"
    );
    let player = world
        .spawn((
            map,
            position,
            Player {
                name: "ColdWake".into(),
                race: 1,
                class: 1,
                appearance: default(),
            },
        ))
        .id();
    app.world_mut().run_schedule(FixedPreUpdate);
    assert!(
        !dormant(&app, creature),
        "nearby player must wake the real creature"
    );
    assert_eq!(app.world().get::<Health>(creature).unwrap().current, health);
    app.world_mut().despawn(player);
    app.world_mut().run_schedule(FixedPreUpdate);
    assert!(dormant(&app, creature), "leaving must restore dormancy");
    assert_eq!(app.world().get::<Health>(creature).unwrap().current, health);
}

#[test]
fn grid_of_matches_azerothcore_grid_boundaries() {
    let map = InMap::continent(0);
    let (x, _, z) = wow_to_bevy(533.0, -1.0, 0.0);
    assert_eq!(grid_of(map, x, z), (map, 0, -1));
    let (x, _, z) = wow_to_bevy(533.4, 1066.7, 0.0);
    assert_eq!(grid_of(map, x, z), (map, 1, 2));
}

#[test]
fn creature_two_grids_from_any_player_does_not_wander() {
    let mut app = grid_app(HashMap::new());
    spawn_player(&mut app, TWO_GRIDS_AWAY_X);
    let wolf = spawn_wanderer(&mut app);
    let start = position(&app, wolf);
    advance(&mut app, 40);
    assert!(dormant(&app, wolf));
    assert_eq!(position(&app, wolf), start);
    let state = app.world().get::<WanderState>(wolf).unwrap();
    assert_eq!(state.phase, WanderPhase::Idle);
    assert_eq!(state.seed, 3, "dormant wander state must not tick");
}

#[test]
fn creature_wakes_and_wanders_when_player_enters_neighbour_grid() {
    let mut app = grid_app(HashMap::new());
    let player = spawn_player(&mut app, TWO_GRIDS_AWAY_X);
    let wolf = spawn_wanderer(&mut app);
    let start = position(&app, wolf);
    advance(&mut app, 10);
    assert!(dormant(&app, wolf));

    *app.world_mut().get_mut::<Position>(player).unwrap() =
        bevy_position(NEIGHBOUR_GRID_X, SPAWN.1);
    advance(&mut app, 10);
    assert!(!dormant(&app, wolf));
    assert_ne!(position(&app, wolf), start);
}

#[test]
fn creature_goes_dormant_when_the_last_nearby_player_leaves_the_world() {
    let mut app = grid_app(HashMap::new());
    let player = spawn_player(&mut app, NEIGHBOUR_GRID_X);
    let wolf = spawn_wanderer(&mut app);
    advance(&mut app, 2);
    assert!(!dormant(&app, wolf));

    app.world_mut().despawn(player);
    advance(&mut app, 2);
    assert!(dormant(&app, wolf));
    let parked = position(&app, wolf);
    advance(&mut app, 40);
    assert_eq!(position(&app, wolf), parked);
}

#[test]
fn patroller_stays_active_where_its_path_passes_a_player() {
    // Spawn in grid (0, 0); the path runs east into grid (3, 0).
    let path = WaypointPath {
        points: vec![
            WaypointPoint {
                position_x: SPAWN.0,
                position_y: SPAWN.1,
                ..Default::default()
            },
            WaypointPoint {
                position_x: 1700.0,
                position_y: SPAWN.1,
                ..Default::default()
            },
        ],
    };
    let mut app = grid_app(HashMap::from([(PATROL_PATH, path)]));
    // Grid (4, 0): four grids from the spawn, next to the path's far end.
    spawn_player(&mut app, 2200.0);
    let start = bevy_position(SPAWN.0, SPAWN.1);
    let guard = app
        .world_mut()
        .spawn((
            start,
            InMap::continent(0),
            spawn_point(start),
            WaypointState::new(PATROL_PATH, 1.0, 1.0, (start.x, start.z)),
        ))
        .id();
    advance(&mut app, 10);
    assert!(!dormant(&app, guard));
    assert_ne!(position(&app, guard), start);
}

#[test]
fn a_player_on_another_map_does_not_wake_the_creature_under_its_coordinates() {
    let mut app = grid_app(HashMap::new());
    let player = spawn_player(&mut app, SPAWN.0);
    app.world_mut()
        .entity_mut(player)
        .insert(InMap::continent(1));
    let wolf = spawn_wanderer(&mut app);
    advance(&mut app, 5);
    assert!(
        dormant(&app, wolf),
        "Kalimdor's player leaves map 0 grids asleep"
    );

    app.world_mut()
        .entity_mut(player)
        .insert(InMap::continent(0));
    advance(&mut app, 5);
    assert!(!dormant(&app, wolf));
}
