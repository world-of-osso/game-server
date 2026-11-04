//! Movement uses complete native paths and preserves the native reach boundary.
use super::*;
use crate::creature_ai::PetMotion;
use std::time::Duration;

#[test]
fn murder_row_owner_approaches_tank_from_connected_surface() {
    let root = super::super::graph::extraction::asset_root().join("navmesh");
    let walker = NativeMovement::open(&root, 2813).unwrap();
    let from = Vec3::new(8722.042, 20.234062, 4878.254);
    let tank = Vec3::new(8711.105, 13.170672, 4896.1533);
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world.insert_resource(walker);
    let owner = world
        .spawn((
            Position {
                x: from.x,
                y: from.y,
                z: from.z,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    let tank = world
        .spawn(Position {
            x: tank.x,
            y: tank.y,
            z: tank.z,
        })
        .id();
    move_owner_to_tank(&mut world, owner, tank, 30.0).unwrap();
    let destination = world.get::<MoveIntent>(owner).unwrap().0;
    assert!(destination.distance(Vec3::new(8711.105, 13.170672, 4896.1533)) <= 30.0);
    tick(&mut world);
    assert!(point(world.get::<Position>(owner).unwrap()).distance(from) > 0.01);
    assert!(server::simc_apl::collect_fact_errors(&mut world).is_empty());
}

#[test]
fn unreachable_owner_destination_records_a_fact_error_instead_of_panicking() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    super::super::native_movement::install_parity_arena(&mut world).unwrap();
    let owner = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            MoveIntent(Vec3::splat(1_000_000.0)),
        ))
        .id();
    tick(&mut world);
    let errors = server::simc_apl::collect_fact_errors(&mut world);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].fact, "movement.native");
    assert_eq!(errors[0].actor, owner.to_bits());
    assert!(errors[0].detail.contains("native walk unreachable"));
    assert!(errors[0].detail.contains("from="));
    assert!(errors[0].detail.contains("to="));
    assert!(world.get::<MoveIntent>(owner).is_none());
    assert!(world.get::<BlockedWalk>(owner).is_some());
}

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

#[test]
fn parity_arena_follower_walks_to_owner_through_detour() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    super::super::native_movement::install_parity_arena(&mut world).unwrap();
    let owner = world
        .spawn(Position {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let follower = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            crate::creature_ai::FollowOwner::new(owner),
        ))
        .id();
    for _ in 0..100 {
        tick(&mut world);
    }
    let actual = point(world.get::<Position>(follower).unwrap());
    assert!(
        actual.x >= 15.9 && actual.x <= 16.1,
        "follower position {actual:?}"
    );
    assert!(world.get::<BlockedWalk>(follower).is_none());
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
    let (from, to) = short_walk(&walker);
    let (mut world, pet) = pet_world(from, Vec3::splat(1_000_000.0));
    move_one(&mut world, pet, 0.05).unwrap();
    assert!(
        inspect_travel(&world, pet)["blocked"]
            .as_str()
            .unwrap()
            .contains("native walk unreachable")
    );
    assert_eq!(point(world.get::<Position>(pet).unwrap()), from);
    let target = match world.get::<PetMotion>(pet).unwrap() {
        PetMotion::Chase(target) => *target,
        _ => unreachable!(),
    };
    world.entity_mut(target).insert(Position {
        x: to.x,
        y: to.y,
        z: to.z,
    });
    move_one(&mut world, pet, 0.05).unwrap();
    assert!(world.get::<BlockedWalk>(pet).is_none());
    assert_ne!(point(world.get::<Position>(pet).unwrap()), from);
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
