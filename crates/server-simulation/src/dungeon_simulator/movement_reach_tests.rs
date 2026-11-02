//! Movement uses complete native paths and preserves the native reach boundary.
use super::*;
use crate::creature_ai::PetMotion;
use std::time::Duration;

pub(super) fn altar_walker() -> NativeMovement {
    let root = super::super::graph::extraction::asset_root().join("navmesh");
    NativeMovement::open(&root, 2993).unwrap()
}

pub(super) fn short_walk(walker: &NativeMovement) -> (Vec3, Vec3) {
    let from = Vec3::new(1336.9683, -62.376408, -2058.8662);
    let to = Vec3::new(1331.6643, -62.065018, -2046.7841);
    let legs = walker.walk_legs(from, to, "test").unwrap();
    (legs.front().unwrap().0, legs.back().unwrap().0)
}

fn pet_world(from: Vec3, to: Vec3) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(altar_walker());
    let position = |point: Vec3| Position {
        x: point.x,
        y: point.y,
        z: point.z,
    };
    let target = world.spawn(position(to)).id();
    let pet = world
        .spawn((
            position(from),
            Health {
                current: 100.0,
                max: 100.0,
            },
            PetMotion::Chase(target),
        ))
        .id();
    (world, pet)
}

fn tick(world: &mut World) {
    world
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(50));
    update(world);
}

#[test]
fn changed_destination_is_replanned_after_cached_route_ends() {
    let walker = altar_walker();
    let (from, to) = short_walk(&walker);
    let first = from.lerp(to, 0.5);
    let (mut world, owner) = pet_world(from, first);
    world
        .entity_mut(owner)
        .remove::<PetMotion>()
        .insert(MoveIntent(first));
    for _ in 0..100 {
        tick(&mut world);
    }
    world.entity_mut(owner).insert(MoveIntent(to));
    for _ in 0..100 {
        tick(&mut world);
    }
    let actual = point(world.get::<Position>(owner).unwrap());
    assert!(actual.distance(to) < 0.1, "{actual:?} != {to:?}");
}

#[test]
fn chase_replans_if_target_moves_at_cached_reach_boundary() {
    let walker = altar_walker();
    let (from, to) = short_walk(&walker);
    let first = from.lerp(to, 0.8);
    let (mut world, pet) = pet_world(from, first);
    let target = match world.get::<PetMotion>(pet).unwrap() {
        PetMotion::Chase(target) => *target,
        _ => unreachable!(),
    };
    for _ in 0..100 {
        tick(&mut world);
    }
    world.entity_mut(target).insert(Position {
        x: to.x,
        y: to.y,
        z: to.z,
    });
    for _ in 0..100 {
        tick(&mut world);
    }
    let actual = point(world.get::<Position>(pet).unwrap());
    assert!(
        server::test_support::within_reach(actual, to, crate::creature_ai::CHASE_REACH),
        "pet stopped at {actual:?} from {to:?}"
    );
}

#[test]
fn in_reach_pet_and_follower_remain_stationary() {
    let walker = altar_walker();
    let (from, to) = short_walk(&walker);
    let nearby = from.lerp(to, 0.05);
    for follow in [false, true] {
        let (mut world, actor) = pet_world(from, nearby);
        if follow {
            let owner = match world.get::<PetMotion>(actor).unwrap() {
                PetMotion::Chase(owner) => *owner,
                _ => unreachable!(),
            };
            world
                .entity_mut(actor)
                .remove::<PetMotion>()
                .insert(crate::creature_ai::FollowOwner::new(owner));
        }
        for _ in 0..80 {
            tick(&mut world);
        }
        assert_eq!(point(world.get::<Position>(actor).unwrap()), from);
    }
}

#[test]
fn unreachable_pet_path_returns_named_error_without_graph_fallback() {
    let walker = altar_walker();
    let (from, _) = short_walk(&walker);
    let (mut world, pet) = pet_world(from, Vec3::splat(1_000_000.0));
    let error = move_one(&mut world, pet, 0.05).unwrap_err();
    assert!(error.contains("native walk unreachable"), "{error}");
    assert_eq!(point(world.get::<Position>(pet).unwrap()), from);
}

#[test]
fn altar_mover_follows_real_detour_not_frozen_graph_detour() {
    let walker = altar_walker();
    let from = Vec3::new(1081.99, -9.396648, -2108.46);
    let to = Vec3::new(1105.28, -38.116116, -1988.8);
    let legs = walker.walk_legs(from, to, "test").unwrap();
    let yards: f32 = legs.iter().map(|(_, yards)| yards).sum();
    assert!((160.0..180.0).contains(&yards), "native walk {yards}");
    let mut expected = Travel {
        destination: to,
        legs,
    };
    let expected = advance(
        from,
        &mut expected,
        crate::creature_ai::BASE_RUN_SPEED * 0.05,
        0.0,
    );
    let (mut world, actor) = pet_world(from, to);
    world
        .entity_mut(actor)
        .remove::<PetMotion>()
        .insert(MoveIntent(to));
    tick(&mut world);
    let actual = point(world.get::<Position>(actor).unwrap());
    assert!(
        actual.distance(expected) < 0.001,
        "{actual:?} != {expected:?}"
    );
    let (_, remaining) = remaining_route(&world, actor).unwrap();
    assert!((remaining - (yards - crate::creature_ai::BASE_RUN_SPEED * 0.05)).abs() < 0.01);
}
