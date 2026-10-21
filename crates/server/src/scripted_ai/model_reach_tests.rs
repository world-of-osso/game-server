use super::*;
use crate::creature_aggro::AggroDistance;
use bevy::ecs::system::SystemState;

fn read_both_reaches(world: &mut World, unit: Entity) -> (Option<f32>, Option<f32>) {
    let scripted = combat_reach(world, unit);
    let mut state = SystemState::<AggroDistance>::new(world);
    let sight = state.get(world).unwrap().combat_reach(unit);
    (scripted, sight)
}

#[test]
#[ignore = "explicit missing-model-info scaled-default policy regression"]
fn missing_native_model_info_uses_scaled_player_default_reach() {
    let mut world = World::new();
    // Source Henry215011 display121275 has no encounter model-info row.
    world.insert_resource(ModelReaches(HashMap::new()));
    for (scale, expected) in [(0.5, 0.75), (1.0, 1.5), (2.0, 3.0)] {
        let unit = world
            .spawn((ModelDisplay { display_id: 121275 }, UnitScale(scale)))
            .id();
        assert_eq!(
            read_both_reaches(&mut world, unit),
            (Some(expected), Some(expected))
        );
    }
}

#[test]
#[ignore = "explicit known-model native reach preservation control"]
fn sourced_model_info_preserves_scaled_native_reach() {
    let mut world = World::new();
    // Source Crenna209072 display113757 has native combat reach3yd.
    world.insert_resource(ModelReaches([(113757, 3.0)].into()));
    for (scale, expected) in [(0.5, 1.5), (1.0, 3.0), (2.0, 6.0)] {
        let unit = world
            .spawn((ModelDisplay { display_id: 113757 }, UnitScale(scale)))
            .id();
        assert_eq!(
            read_both_reaches(&mut world, unit),
            (Some(expected), Some(expected))
        );
    }
}
