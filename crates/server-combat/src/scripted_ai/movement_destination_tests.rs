//! The point generator publishes its target to the APL for as long as it runs.
use super::*;
use crate::simc_apl::Facts;

fn read_distance(world: &World, unit: Entity) -> Result<f64, String> {
    use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
    NativeFacts::new(
        world,
        unit,
        &Bindings::default(),
        Encounter::dungeon(0.0),
        1,
    )
    .value("movement.distance")
}

fn walker() -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let unit = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            SpawnPoint {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                wander_distance: 0.0,
                speed_walk: 1.0,
                speed_run: 1.0,
            },
            CreatureMotion::Still,
        ))
        .id();
    (world, unit)
}

fn tick(world: &mut World, seconds: f32) {
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_secs_f32(seconds));
    point_movement_system(world);
}

const TARGET: Position = Position {
    x: 14.0,
    y: 0.0,
    z: 0.0,
};

#[test]
fn point_movement_publishes_its_target_until_arrival() {
    let (mut world, unit) = walker();
    assert_eq!(read_distance(&world, unit).unwrap(), 0.0, "idle");
    move_point(&mut world, unit, 7, TARGET);
    let target = MovementDestination(Vec3::new(14.0, 0.0, 0.0));
    assert_eq!(world.get::<MovementDestination>(unit), Some(&target));
    tick(&mut world, 1.0);
    assert_eq!(
        world.get::<CreatureMotion>(unit),
        Some(&CreatureMotion::Run)
    );
    assert_eq!(read_distance(&world, unit).unwrap(), 7.0);
    // A charge or jump that pauses the generator replaces then clears the destination.
    world.entity_mut(unit).remove::<MovementDestination>();
    tick(&mut world, 0.5);
    assert_eq!(world.get::<MovementDestination>(unit), Some(&target));
    assert_eq!(read_distance(&world, unit).unwrap(), 3.5);
    tick(&mut world, 1.0);
    assert!(world.get::<PointMovement>(unit).is_none());
    assert!(world.get::<MovementDestination>(unit).is_none());
    assert_eq!(read_distance(&world, unit).unwrap(), 0.0);
}

#[test]
fn cleared_point_movement_drops_its_destination() {
    let (mut world, unit) = walker();
    move_point(&mut world, unit, 7, TARGET);
    tick(&mut world, 1.0);
    clear_point_movement(&mut world, unit);
    assert!(world.get::<MovementDestination>(unit).is_none());
    assert_eq!(read_distance(&world, unit).unwrap(), 0.0);
}
