//! SimC movement distance is configured fixture state, never target separation.
use super::*;
use shared::components::Position;

fn fixture() -> (World, Entity, Entity) {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
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
fn native_movement_speed_reads_current_native_modifier_even_while_idle() {
    let (mut world, actor, _) = fixture();
    world.entity_mut(actor).insert(FixtureMovement {
        distance_to_move: 0.0,
    });
    let bindings = Bindings::default();
    let read = |world: &World| {
        NativeFacts::new(world, actor, &bindings, patchwerk(), 1).value("movement.speed")
    };
    assert_eq!(read(&world).unwrap(), 7.0);
    world.entity_mut(actor).insert(crate::auras::AuraStatMods {
        movement_speed: 1.5,
        ..Default::default()
    });
    assert_eq!(read(&world).unwrap(), 10.5);
}

#[test]
fn native_movement_remains_divides_remaining_configured_distance_by_live_speed() {
    let (mut world, actor, _) = fixture();
    world.entity_mut(actor).insert(FixtureMovement {
        distance_to_move: 14.0,
    });
    let bindings = Bindings::default();
    let read = |world: &World| {
        NativeFacts::new(world, actor, &bindings, patchwerk(), 1).value("movement.remains")
    };
    assert_eq!(read(&world).unwrap(), 2.0);
    world.entity_mut(actor).insert(crate::auras::AuraStatMods {
        movement_speed: 2.0,
        ..Default::default()
    });
    assert_eq!(read(&world).unwrap(), 1.0);
    world
        .get_mut::<FixtureMovement>(actor)
        .unwrap()
        .distance_to_move = 0.0;
    assert_eq!(read(&world).unwrap(), 0.0);
}

#[test]
fn native_movement_predicates_keep_missing_and_dungeon_configuration_explicit() {
    let (world, actor, _) = fixture();
    let bindings = Bindings::default();
    for operand in ["movement.remains", "movement.speed"] {
        let missing = NativeFacts::new(&world, actor, &bindings, patchwerk(), 1)
            .value(operand)
            .unwrap_err();
        assert!(
            missing.contains("NATIVE_MOVEMENT_DISTANCE_UNAVAILABLE"),
            "{missing}"
        );
        let dungeon = NativeFacts::new(
            &world,
            actor,
            &bindings,
            Encounter::Dungeon { started_at: 0.0 },
            1,
        )
        .value(operand)
        .unwrap_err();
        assert!(
            dungeon.contains("DUNGEON_MOVEMENT_DISTANCE_UNAVAILABLE"),
            "{dungeon}"
        );
    }
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
