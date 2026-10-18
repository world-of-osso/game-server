//! Target32 scope controls on real Ramparts ground; 62/138/18 must not change.
use super::targets::{Dest, effect_dest, ground_destination};
use crate::{
    ground::{GroundMap, GroundProbe},
    maps::InMap,
    spell_info::SpellEffect,
};
use bevy::prelude::*;
use shared::components::{Npc, Position, Rotation};

const WEST: Position = Position {
    x: -1423.4910,
    y: 103.7056,
    z: -1710.4010,
};

fn elevated_caster() -> (World, Entity, InMap) {
    let mut world = World::new();
    world.insert_resource(crate::test_data::ground_map());
    let map = InMap {
        map_id: 543,
        instance_id: 1,
    };
    let caster = world
        .spawn((
            map,
            WEST,
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Npc {
                template_id: 17_536,
                name: "Nazan".into(),
            },
        ))
        .id();
    (world, caster, map)
}

fn resolve(world: &mut World, caster: Entity, selector: u32) -> Dest {
    // These are selector-unit controls, not fabricated authored spells.
    let effect = SpellEffect {
        implicit_targets: [selector, 0],
        ..Default::default()
    };
    let requested = Some([WEST.x, WEST.y, WEST.z]);
    let destination = effect_dest(world, caster, Some(caster), &effect, requested);
    ground_destination(world, caster, &effect, destination)
        .unwrap()
        .unwrap()
}

#[test]
fn target32_controls_caster_ground62_and_requested_ground138_ignore_flight_and_keep_xy() {
    let (mut world, caster, map) = elevated_caster();
    world
        .entity_mut(caster)
        .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
    let source = Vec3::new(WEST.x, WEST.y, WEST.z);
    let GroundProbe::Supported(floor) = world.resource_mut::<GroundMap>().ground_in(map, source)
    else {
        panic!("real Ramparts floor required for existing62/138 controls");
    };
    assert!(
        floor < WEST.y,
        "authored flight point is above the real floor"
    );
    for selector in [62, 138] {
        let result = resolve(&mut world, caster, selector);
        assert_eq!(
            result.position,
            Vec3::new(WEST.x, floor, WEST.z),
            "dedicated ground selector{selector} remains grounded even for a flyer"
        );
    }
}

#[test]
fn target32_controls_caster_destination18_does_not_ground_or_move() {
    let (mut world, caster, _) = elevated_caster();
    let source = Vec3::new(WEST.x, WEST.y, WEST.z);
    assert_eq!(resolve(&mut world, caster, 18).position, source);
    world
        .entity_mut(caster)
        .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
    assert_eq!(resolve(&mut world, caster, 18).position, source);
}
