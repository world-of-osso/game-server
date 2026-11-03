//! Havoc's walk to the melee edge publishes that edge to the APL until it arrives.
use super::*;
use crate::simc_apl::Facts;
use crate::simc_apl_native::{Bindings, Encounter, MovementDestination, NativeFacts};
use shared::components::{CharacterAppearance, CharacterVisual, CreatureMotion};

fn read_distance(world: &World, actor: Entity) -> Result<f64, String> {
    NativeFacts::new(
        world,
        actor,
        &Bindings::default(),
        Encounter::dungeon(0.0),
        1,
    )
    .value("movement.distance")
}

fn visual() -> CharacterVisual {
    CharacterVisual {
        race: 10,
        class: 12,
        appearance: CharacterAppearance::default(),
    }
}

#[test]
fn retreat_setup_publishes_the_melee_edge_until_it_arrives() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    world
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(500));
    let target = world
        .spawn((
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            visual(),
        ))
        .id();
    let actor = world
        .spawn((
            Position {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            CreatureMotion::Still,
            visual(),
        ))
        .id();
    assert_eq!(read_distance(&world, actor).unwrap(), 0.0, "idle");
    let range = crate::scripted_ai::model_reach::melee_range(&world, actor, target).unwrap();
    let edge = Vec3::new(range - 0.02, 0.0, 0.0);
    assert!(!prepare_havoc_retreat(&mut world, actor, target).unwrap());
    let published = world.get::<MovementDestination>(actor).unwrap().0;
    assert!(published.distance(edge) < 1e-4, "{published:?} vs {edge:?}");
    let x = world.get::<Position>(actor).unwrap().x;
    assert!((x - 6.5).abs() < 1e-4, "one 0.5s run stride: {x}");
    assert_eq!(
        world.get::<CreatureMotion>(actor),
        Some(&CreatureMotion::Run)
    );
    let distance = read_distance(&world, actor).unwrap();
    assert!(
        (distance - f64::from(x - published.x)).abs() < 1e-5,
        "{distance}"
    );
    let mut arrived = false;
    for _ in 0..4 {
        if prepare_havoc_retreat(&mut world, actor, target).unwrap() {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "reaches the melee edge");
    assert!(world.get::<RetreatSetup>(actor).is_none());
    assert!(world.get::<MovementDestination>(actor).is_none());
}
