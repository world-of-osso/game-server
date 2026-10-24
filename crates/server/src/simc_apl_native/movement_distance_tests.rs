//! SimC movement distance is configured fixture state, never target separation.
use super::*;
use shared::components::Position;

fn fixture() -> (World, Entity, Entity) {
    let mut world = World::new();
    let target = world
        .spawn(Position {
            x: 50.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let actor = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            crate::networking::CombatTarget(Some(target)),
        ))
        .id();
    (world, actor, target)
}

fn patchwerk() -> Encounter {
    Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn read_distance(world: &World, actor: Entity, encounter: Encounter) -> Result<f64, String> {
    NativeFacts::new(world, actor, &Bindings::default(), encounter, 1).value("movement.distance")
}

#[test]
fn native_movement_distance_reads_configured_state_not_target_range() {
    let (mut world, actor, target) = fixture();
    world.entity_mut(actor).insert(FixtureMovement {
        distance_to_move: 12.5,
    });
    assert_eq!(read_distance(&world, actor, patchwerk()).unwrap(), 12.5);
    world.get_mut::<Position>(target).unwrap().x = 100.0;
    assert_eq!(read_distance(&world, actor, patchwerk()).unwrap(), 12.5);
    world
        .get_mut::<FixtureMovement>(actor)
        .unwrap()
        .distance_to_move = 7.25;
    assert_eq!(read_distance(&world, actor, patchwerk()).unwrap(), 7.25);
}

#[test]
fn native_movement_distance_explicit_zero_does_not_follow_actor_motion() {
    let (mut world, actor, _) = fixture();
    world.entity_mut(actor).insert(FixtureMovement {
        distance_to_move: 0.0,
    });
    world.get_mut::<Position>(actor).unwrap().x = 250.0;
    assert_eq!(read_distance(&world, actor, patchwerk()).unwrap(), 0.0);
}

#[test]
fn native_movement_distance_missing_configuration_remains_a_named_gap() {
    let (world, actor, _) = fixture();
    let error = read_distance(&world, actor, patchwerk()).unwrap_err();
    assert!(
        error.contains("NATIVE_MOVEMENT_DISTANCE_UNAVAILABLE"),
        "unconfigured movement must not invent zero: {error}",
    );
}

#[test]
fn native_movement_distance_dungeon_does_not_borrow_fixture_configuration() {
    let (mut world, actor, _) = fixture();
    world.entity_mut(actor).insert(FixtureMovement {
        distance_to_move: 12.5,
    });
    let error = read_distance(&world, actor, Encounter::Dungeon { started_at: 0.0 }).unwrap_err();
    assert!(
        error.contains("DUNGEON_MOVEMENT_DISTANCE_UNAVAILABLE"),
        "{error}"
    );
}
