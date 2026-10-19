//! Grounding-only target32: real source spells/geometry, no model or reach data.
use super::{plan_targets, targets};
use crate::{ground::GroundMap, maps::InMap, spell_info::SpellInfo};
use bevy::prelude::*;
use shared::components::{Position, Rotation};

const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};
const MAP: InMap = InMap {
    map_id: 543,
    instance_id: 9132,
};

fn source_spell(id: u32) -> SpellInfo {
    let spell = super::test_harness::world_db()
        .unwrap()
        .store
        .get(id)
        .unwrap()
        .clone();
    assert!(
        spell.effects[0].implicit_targets.contains(&32),
        "native{id} must select32"
    );
    spell
}

fn caster_world() -> (World, Entity) {
    let mut world = World::new();
    world.insert_resource(super::test_harness::world_db().unwrap().store.clone());
    // No ModelDisplay, Player, ModelReaches or invented creature statistics.
    let caster = world
        .spawn((
            MAP,
            CENTER,
            Rotation {
                x: 0.0,
                y: std::f32::consts::FRAC_PI_2,
                z: 0.0,
            },
        ))
        .id();
    (world, caster)
}

fn selected(world: &World, caster: Entity, spell: &SpellInfo) -> targets::Dest {
    targets::effect_dest(world, caster, Some(caster), &spell.effects[0], None).unwrap()
}

fn planned(world: &mut World, caster: Entity, spell: &SpellInfo) -> targets::Dest {
    plan_targets(world, caster, spell, Some(caster), false)
        .unwrap()
        .into_iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .dest
        .unwrap()
}

fn source_floor(world: &mut World, position: Vec3) -> f32 {
    let probe = world
        .resource_mut::<GroundMap>()
        .creature_ground_in(MAP, position);
    let crate::ground::GroundProbe::Supported(floor) = probe else {
        panic!("real Ramparts source geometry required: {probe:?}");
    };
    floor
}

fn assert_floor_from_cast_ray(world: &mut World, before: targets::Dest, after: targets::Dest) {
    // TC Object.cpp709–748 queries once with the pre-clamp Y, then assigns Z.
    // A ray from the assigned Y would be a different query, not TC's contract.
    let ray = Vec3::new(after.position.x, before.position.y, after.position.z);
    let expected = source_floor(world, ray);
    assert_eq!(
        after.position.y.to_bits(),
        expected.to_bits(),
        "bit-exact source floor at final XZ with the same pre-clamp Y/ray"
    );
}

fn assert_xy_and_facing_retained(before: targets::Dest, after: targets::Dest) {
    assert_eq!(
        (after.position.x, after.position.z, after.orientation),
        (before.position.x, before.position.z, before.orientation),
        "TARGET32_CASTER_SUMMON_XY_UNIMPLEMENTED: grounding must not change existing XY/facing"
    );
}

#[test]
#[ignore = "RED: target32 dry non-flyer must land on source floor without model/reach data"]
fn target32_grounding_dry_source30717_without_model_reach() {
    let (mut world, caster) = caster_world();
    world.insert_resource(crate::test_data::ground_map());
    let spell = source_spell(30717);
    let before = selected(&world, caster, &spell);
    assert!(source_floor(&mut world, before.position) < before.position.y - 1.0);
    let after = planned(&mut world, caster, &spell);
    assert_xy_and_facing_retained(before, after);
    assert_floor_from_cast_ray(&mut world, before, after);
}

#[test]
#[ignore = "RED: target32 CanFly must preserve above-floor altitude and raise below-floor altitude"]
fn target32_grounding_gravity_disabled_above_and_below_floor() {
    for below in [false, true] {
        let (mut world, caster) = caster_world();
        world.insert_resource(crate::test_data::ground_map());
        world
            .entity_mut(caster)
            .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
        let spell = source_spell(30717);
        let first = selected(&world, caster, &spell);
        let floor = source_floor(&mut world, first.position);
        if below {
            world.get_mut::<Position>(caster).unwrap().y = floor - 0.5;
        }
        let before = selected(&world, caster, &spell);
        let after = planned(&mut world, caster, &spell);
        assert_xy_and_facing_retained(before, after);
        if below {
            assert_floor_from_cast_ray(&mut world, before, after);
        } else {
            assert_eq!(after.position.y.to_bits(), before.position.y.to_bits());
        }
    }
}

#[test]
fn target32_grounding_invalid_or_missing_height_keeps_source688_destination() {
    for empty_resource in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (mut world, caster) = caster_world();
        if empty_resource {
            world.insert_resource(GroundMap::new(directory.path(), Default::default()));
        }
        let spell = source_spell(688);
        let before = selected(&world, caster, &spell);
        let after = planned(&mut world, caster, &spell);
        assert_xy_and_facing_retained(before, after);
        assert_eq!(
            after.position.y, before.position.y,
            "TC INVALID_HEIGHT preserves Z instead of rejecting the cast"
        );
    }
}

#[test]
#[ignore = "RED: target32 FORCE final-ground-Z branch must not preserve a flyer's altitude"]
fn target32_grounding_force_grounds_flyer_without_changing_existing_xy() {
    let (mut world, caster) = caster_world();
    world.insert_resource(crate::test_data::ground_map());
    world
        .entity_mut(caster)
        .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
    let mut spell = source_spell(30717);
    assert_eq!(spell.misc.as_ref().unwrap().attributes[9] & 0x1, 0);
    // Counterfactual source-defined flag; no authored database row is changed.
    spell.misc.as_mut().unwrap().attributes[9] |= 0x1;
    let before = selected(&world, caster, &spell);
    let after = planned(&mut world, caster, &spell);
    assert_xy_and_facing_retained(before, after);
    assert_floor_from_cast_ray(&mut world, before, after);
}
