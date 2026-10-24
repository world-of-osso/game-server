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
fn in_reach_pet_stays_in_reach_for_eighty_native_movement_ticks() {
    for from in [Vec3::ZERO, Vec3::new(1.83, 0.0, 0.0)] {
        let (mut world, pet) = pet_world(Some(stockade_space()), from, Vec3::ZERO);
        for frame in 0..80 {
            tick(&mut world);
            let actual = point(world.get::<Position>(pet).unwrap());
            assert_eq!(actual, from, "an in-reach pet moved on tick{frame}");
            assert!(crate::navigation::within_reach(
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
    assert!(!crate::navigation::within_reach(
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
