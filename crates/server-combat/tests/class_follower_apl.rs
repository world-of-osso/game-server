//! Production library test: live facts do not require fixture-only features.
use bevy::prelude::*;
use server_combat::simc_apl::Facts;
use server_combat::simc_apl_native::{Bindings, Encounter, NativeFacts};
use shared::components::{CharacterAppearance, CharacterVisual, Health, Position};

#[test]
fn class_follower_live_apl_reads_real_target_health_distance_and_count() {
    let mut world = World::new();
    world.insert_resource(Time::<()>::default());
    let actor = world
        .spawn(Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        })
        .id();
    let target = world
        .spawn((
            Position {
                x: 3.0,
                y: 0.0,
                z: 4.0,
            },
            Health {
                current: 250.0,
                max: 1000.0,
            },
        ))
        .id();
    for unit in [actor, target] {
        world.entity_mut(unit).insert(CharacterVisual {
            race: 1,
            class: 1,
            appearance: CharacterAppearance::default(),
        });
    }
    let targets = [target];
    let bindings = Bindings::default();
    let facts = NativeFacts::new(&world, actor, &bindings, Encounter::dungeon(0.0), 1)
        .with_targets(Some(target), &targets);
    assert_eq!(facts.value("target.health.pct").unwrap(), 25.0);
    assert_eq!(facts.value("target.distance").unwrap(), 2.0);
    assert_eq!(facts.value("active_enemies").unwrap(), 1.0);
    assert_eq!(facts.value("raid_event.adds.exists").unwrap(), 0.0);
}
