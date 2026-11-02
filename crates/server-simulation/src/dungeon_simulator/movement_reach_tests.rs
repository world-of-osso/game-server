//! Chase movement uses the native reach boundary without borrowing unrelated dungeon geometry.
use super::*;
use crate::creature_ai::PetMotion;
use std::time::Duration;

fn stockade_space() -> Space {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/dungeons/stockade-34.json");
    Space(serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap())
}

fn pet_world(space: Option<Space>, from: Vec3, to: Vec3) -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    if let Some(space) = space {
        world.insert_resource(space);
    }
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
#[ignore = "requires extracted Ruby Mythic graph"]
fn ruby_owner_can_return_to_tank_from_observed_stall_positions() {
    let space = Space(super::super::graph::read_graph().unwrap());
    let from = Vec3::new(1480.93005, 126.46381, 176.06);
    let to = Vec3::new(1462.78003, 126.27969, 174.649);
    let (mut world, owner) = pet_world(Some(space), from, to);
    world
        .entity_mut(owner)
        .remove::<PetMotion>()
        .insert(MoveIntent(to));
    for frame in 0..600 {
        let y = if frame % 2 == 0 { 126.27969 } else { 126.5823 };
        world
            .entity_mut(owner)
            .insert(MoveIntent(Vec3::new(to.x, y, to.z)));
        tick(&mut world);
        let current = point(world.get::<Position>(owner).unwrap());
        if current.distance(to) < 4.0 {
            return;
        }
    }
    panic!(
        "owner remained at {:?} while returning to the tank; travel={}",
        world.get::<Position>(owner),
        inspect_travel(&world, owner)
    );
}

#[test]
fn changed_destination_is_replanned_after_cached_route_ends() {
    let (mut world, owner) = pet_world(None, Vec3::ZERO, Vec3::X * 2.0);
    world
        .entity_mut(owner)
        .remove::<PetMotion>()
        .insert(MoveIntent(Vec3::X * 2.0));
    tick(&mut world);
    world.entity_mut(owner).insert(MoveIntent(Vec3::X * 2.5));
    for _ in 0..20 {
        tick(&mut world);
    }
    assert!(point(world.get::<Position>(owner).unwrap()).distance(Vec3::X * 2.5) < 0.001);
}

#[test]
fn chase_replans_if_target_moves_at_cached_reach_boundary() {
    let (mut world, pet) = pet_world(None, Vec3::ZERO, Vec3::X * 10.0);
    let target = match world.get::<PetMotion>(pet).unwrap() {
        PetMotion::Chase(target) => *target,
        _ => unreachable!(),
    };
    for _ in 0..100 {
        tick(&mut world);
        if server::test_support::within_reach(
            point(world.get::<Position>(pet).unwrap()),
            Vec3::X * 10.0,
            crate::creature_ai::CHASE_REACH,
        ) {
            break;
        }
    }
    world.entity_mut(target).insert(Position {
        x: 10.5,
        y: 0.0,
        z: 0.0,
    });
    for _ in 0..20 {
        tick(&mut world);
    }
    let actual = point(world.get::<Position>(pet).unwrap());
    assert!(
        server::test_support::within_reach(actual, Vec3::X * 10.5, crate::creature_ai::CHASE_REACH),
        "pet stopped at {actual:?}"
    );
}

#[test]
fn in_reach_pet_stays_in_reach_for_eighty_native_movement_ticks() {
    for from in [Vec3::ZERO, Vec3::new(1.83, 0.0, 0.0)] {
        let (mut world, pet) = pet_world(Some(stockade_space()), from, Vec3::ZERO);
        for frame in 0..80 {
            tick(&mut world);
            let actual = point(world.get::<Position>(pet).unwrap());
            assert_eq!(actual, from, "an in-reach pet moved on tick{frame}");
            assert!(server::test_support::within_reach(
                actual,
                Vec3::ZERO,
                crate::creature_ai::CHASE_REACH,
            ));
        }
    }
}

#[test]
fn in_reach_follow_owner_does_not_walk_the_actor_away_from_the_dummy() {
    let from = Vec3::new(1.83, 0.0, 0.0);
    let (mut world, actor) = pet_world(Some(stockade_space()), from, Vec3::ZERO);
    let owner = match world.get::<PetMotion>(actor).unwrap() {
        PetMotion::Chase(owner) => *owner,
        _ => unreachable!(),
    };
    world
        .entity_mut(actor)
        .remove::<PetMotion>()
        .insert(crate::creature_ai::FollowOwner::new(owner));
    for frame in 0..80 {
        tick(&mut world);
        assert_eq!(
            point(world.get::<Position>(actor).unwrap()),
            from,
            "actor left its in-reach owner/dummy on tick{frame}",
        );
    }
}

#[test]
fn open_map_pet_without_a_frozen_graph_chases_directly_then_holds_reach() {
    let from = Vec3::new(1.83, 0.0, 0.0);
    let to = Vec3::new(20.0, 0.0, 0.0);
    let (mut world, pet) = pet_world(None, from, to);
    let mut previous = from.distance(to);
    for _ in 0..100 {
        tick(&mut world);
        let actual = point(world.get::<Position>(pet).unwrap());
        assert_eq!(actual.y, 0.0);
        assert_eq!(actual.z, 0.0);
        let distance = actual.distance(to);
        assert!(
            distance <= previous,
            "direct chase moved away from its target"
        );
        previous = distance;
    }
    assert!((previous - crate::creature_ai::CHASE_REACH).abs() < 0.0001);
}

#[test]
fn pet_holds_when_its_chase_target_has_no_walkable_graph_path() {
    let mut space = stockade_space();
    let from = space.0.packs[0].point();
    let to = space
        .0
        .packs
        .iter()
        .map(|pack| pack.point())
        .find(|point| point.distance(from) > 10.0)
        .unwrap();
    space.0.edges.clear();
    space.0.teleports.clear();
    let (mut world, pet) = pet_world(Some(space), from, to);
    for _ in 0..10 {
        tick(&mut world);
    }
    assert_eq!(point(world.get::<Position>(pet).unwrap()), from);
    assert!(matches!(
        world.get::<PetMotion>(pet),
        Some(PetMotion::Chase(_))
    ));
}

#[test]
fn out_of_reach_pet_on_a_real_dungeon_graph_still_uses_frozen_anchors() {
    let space = stockade_space();
    let edge = space.0.edges.iter().find(|edge| edge.yards > 20.0).unwrap();
    let pack_point = |id| {
        space
            .0
            .packs
            .iter()
            .find(|pack| pack.id == id)
            .unwrap()
            .point()
    };
    let from = pack_point(edge.from) + Vec3::Y * 10.0;
    let to = pack_point(edge.to) + Vec3::Y * 10.0;
    assert!(!server::test_support::within_reach(
        from,
        to,
        crate::creature_ai::CHASE_REACH,
    ));
    let first_anchor = space.route(from, to).unwrap()[0];
    assert_ne!(
        first_anchor, to,
        "control must have an actual anchor detour"
    );
    let expected = from.lerp(
        first_anchor,
        crate::creature_ai::BASE_RUN_SPEED * 0.05 / from.distance(first_anchor),
    );
    let (mut world, pet) = pet_world(Some(space), from, to);
    tick(&mut world);
    let actual = point(world.get::<Position>(pet).unwrap());
    assert!(
        actual.distance(expected) < 0.0001,
        "dungeon chase bypassed its frozen anchor: {actual:?} != {expected:?}"
    );
}
