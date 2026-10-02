//! Chairs from data/world.db (skipped when absent): seats, their occupants and the
//! sitting state they give.

use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};
use shared::components::MovementControl;

use super::*;
use crate::game_objects::{GameObjectSpawn, game_object_bundle, load_game_object_spawns};

/// Northshire Abbey Wooden Bench 151953: 4 seats (Data0), high (Data1 2), size 1.
const NORTHSHIRE_BENCH: u32 = 151_953;
/// Lion's Pride Inn Wooden Chair 177499: 1 seat, medium (Data1 1).
const GOLDSHIRE_CHAIR: u32 = 177_499;

fn spawn_of(entry: u32) -> Option<GameObjectSpawn> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let spawns = load_game_object_spawns(&conn).unwrap();
    Some(
        spawns
            .0
            .into_iter()
            .find(|spawn| spawn.info.entry == entry)
            .expect("chair spawn in world.db"),
    )
}

fn spawn_chair(world: &mut World, spawn: &GameObjectSpawn) -> Entity {
    world
        .spawn((game_object_bundle(spawn), spawn.chair.clone().unwrap()))
        .id()
}

fn spawn_player(world: &mut World, wow: Vec3) -> Entity {
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.x, wow.y, wow.z);
    world
        .spawn((
            Position { x, y, z },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            PlayerStandState::default(),
            MovementControl::default(),
        ))
        .id()
}

fn wow_of(world: &World, player: Entity) -> Vec3 {
    wow_position(world.get::<Position>(player).unwrap())
}

fn state_of(world: &World, player: Entity) -> StandState {
    world.get::<PlayerStandState>(player).unwrap().0
}

/// Seat `seat` of the bench in WoW (x, y): 1 yd apart across its facing, centred.
fn bench_seat(spawn: &GameObjectSpawn, seat: f32) -> Vec2 {
    let across = spawn.orientation + std::f32::consts::FRAC_PI_2;
    let offset = seat - 1.5;
    Vec2::new(
        spawn.position.x + offset * across.cos(),
        spawn.position.y + offset * across.sin(),
    )
}

#[test]
fn a_bench_seats_each_user_on_the_nearest_free_seat_high_and_facing_its_way() {
    let Some(spawn) = spawn_of(NORTHSHIRE_BENCH) else {
        return;
    };
    assert_eq!(spawn.chair, Some(Chair::new(4, 2)));
    let mut world = World::new();
    let bench = spawn_chair(&mut world, &spawn);
    let near_last = bench_seat(&spawn, 3.0).extend(spawn.position.z) + Vec3::X * 2.0;
    let first = spawn_player(&mut world, near_last);
    let second = spawn_player(&mut world, near_last);

    sit(&mut world, first, bench);
    assert_eq!(state_of(&world, first), StandState::SitHighChair);
    let seated = wow_of(&world, first);
    assert!(
        seated.truncate().distance(bench_seat(&spawn, 3.0)) < 1e-3,
        "{seated}"
    );
    assert!((seated.z - spawn.position.z).abs() < 1e-3);
    assert!(
        (world.get::<Rotation>(first).unwrap().y
            - (spawn.orientation + std::f32::consts::FRAC_PI_2))
            .abs()
            < 1e-6
    );
    assert_eq!(world.get::<MovementControl>(first).unwrap().epoch, 1);

    // The last seat is taken: the next nearest is the third.
    sit(&mut world, second, bench);
    let seated = wow_of(&world, second).truncate();
    assert!(seated.distance(bench_seat(&spawn, 2.0)) < 1e-3, "{seated}");

    // Standing up frees the seat for the next user.
    world
        .entity_mut(first)
        .insert(PlayerStandState(StandState::Stand));
    let third = spawn_player(&mut world, near_last);
    sit(&mut world, third, bench);
    let seated = wow_of(&world, third).truncate();
    assert!(seated.distance(bench_seat(&spawn, 3.0)) < 1e-3, "{seated}");
}

#[test]
fn a_full_chair_seats_nobody() {
    let Some(spawn) = spawn_of(GOLDSHIRE_CHAIR) else {
        return;
    };
    assert_eq!(spawn.chair, Some(Chair::new(1, 1)));
    let mut world = World::new();
    let chair = spawn_chair(&mut world, &spawn);
    let first = spawn_player(&mut world, spawn.position + Vec3::X);
    let second = spawn_player(&mut world, spawn.position + Vec3::Y);
    sit(&mut world, first, chair);
    assert_eq!(state_of(&world, first), StandState::SitMediumChair);
    sit(&mut world, second, chair);
    assert_eq!(state_of(&world, second), StandState::Stand);
    assert!(wow_of(&world, second).distance(spawn.position + Vec3::Y) < 1e-3);
}
