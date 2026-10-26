use super::*;
use shared::components::Position;

#[test]
fn native_apl_target_distance_reads_selected_live_positions() {
    let mut world = World::new();
    let actor = world
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let primary = world
        .spawn(Position {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        })
        .id();
    let candidate = world
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 12.0,
        })
        .id();
    let bindings = Bindings::default();
    let encounter = Encounter::Dungeon { started_at: 0.0 };
    let action = Action {
        name: "aspect_of_the_eagle".into(),
        target: Some(candidate.to_bits()),
        source_row: None,
        options: Default::default(),
    };
    let targets = [primary, candidate];
    let read = |world: &World, action: Option<&Action>| {
        let facts = NativeFacts::new(world, actor, &bindings, encounter, 2)
            .with_targets(Some(primary), &targets);
        match action {
            Some(action) => facts.value_for(action, "target.distance"),
            None => facts.value("target.distance"),
        }
        .unwrap()
    };
    assert_eq!(read(&world, None), 5.0);
    assert_eq!(read(&world, Some(&action)), 12.0);
    world.get_mut::<Position>(candidate).unwrap().z = 9.0;
    assert_eq!(read(&world, Some(&action)), 9.0);
    world.get_mut::<Position>(actor).unwrap().z = 3.0;
    assert_eq!(read(&world, Some(&action)), 6.0);
    assert!((read(&world, None) - 34.0_f64.sqrt()).abs() < 0.000_001);
}

#[test]
fn native_apl_target_distance_does_not_invent_missing_positions() {
    let mut world = World::new();
    let actor = world
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let target = world.spawn_empty().id();
    let bindings = Bindings::default();
    let read = |world: &World| {
        NativeFacts::new(
            world,
            actor,
            &bindings,
            Encounter::Dungeon { started_at: 0.0 },
            1,
        )
        .with_targets(Some(target), &[target])
        .value("target.distance")
    };
    assert!(read(&world).unwrap_err().contains("POSITION_UNAVAILABLE"));
    world.entity_mut(target).insert(Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    assert_eq!(read(&world).unwrap(), 0.0);
    world.entity_mut(actor).remove::<Position>();
    assert!(read(&world).unwrap_err().contains("POSITION_UNAVAILABLE"));
}
