//! Field decisions use native radii and actual positions, never tank stationarity.
use bevy::prelude::*;
use shared::components::{Health, Position};

use super::kit::Entry;
use crate::maps::InMap;
use crate::spell_cast::targets::{self, Relation};
use crate::spell_info::SpellStore;

pub(super) fn allows(
    world: &mut World,
    caster: Entity,
    target: Entity,
    enemies: &[(Entity, Option<Entity>)],
    entry: &Entry,
) -> bool {
    let fields = crate::spell_cast::scripts::owned_ground_fields(world, caster, entry.spell_id);
    let covered = fields.iter().any(|(center, radius)| {
        inside_count(world, enemies, *center, *radius) >= entry.conditions.min_enemies
    });
    if entry.conditions.ground && covered {
        return false;
    }
    let Some(radius) = spell_radius(world, caster, entry) else {
        return false;
    };
    let center_unit = if entry.conditions.source_center
        || entry.conditions.target == super::kit::Target::SelfUnit
    {
        caster
    } else {
        target
    };
    let Some(center) = position(world, center_unit) else {
        return false;
    };
    if inside_count(world, enemies, center, radius) < entry.conditions.min_enemies {
        return false;
    }
    // An implicit area must not recruit a fresh pack or break protected CC.
    // Movement remains owned by the existing navigation/positioning controller.
    // The query spans every loaded map: cheap alive/copy/radius filters must run
    // before the faction/detection relation check.
    let map = world.get::<InMap>(caster).copied();
    !world
        .query::<(Entity, &Health, &Position, &InMap)>()
        .iter(world)
        .any(|(unit, health, at, unit_map)| {
            health.current > 0.0
                && Some(*unit_map) == map
                && center.distance(Vec3::new(at.x, at.y, at.z)) <= radius
                && targets::relation_matches(world, caster, unit, Relation::Enemy)
                && (!enemies.iter().any(|(enemy, _)| *enemy == unit)
                    || crate::auras::has_breakable_by_damage_cc(world, unit))
        })
}

fn spell_radius(world: &World, caster: Entity, entry: &Entry) -> Option<f32> {
    let scripted = crate::spell_cast::scripts::ground_pulse_radius(world, caster, entry.spell_id);
    let metadata = world
        .resource::<SpellStore>()
        .get(entry.spell_id)
        .and_then(|spell| {
            spell
                .effects
                .iter()
                .flat_map(|effect| {
                    effect.radius.iter().flatten().map(|radius| {
                        if radius.radius > 0.0 {
                            radius.radius
                        } else {
                            radius.radius_max
                        }
                    })
                })
                .filter(|radius| *radius > 0.0)
                .max_by(f32::total_cmp)
        });
    scripted.or(metadata).filter(|radius| *radius > 0.0)
}

fn inside_count(
    world: &World,
    enemies: &[(Entity, Option<Entity>)],
    center: Vec3,
    radius: f32,
) -> usize {
    enemies
        .iter()
        .filter(|(enemy, _)| {
            !crate::auras::has_breakable_by_damage_cc(world, *enemy)
                && position(world, *enemy).is_some_and(|at| center.distance(at) <= radius)
        })
        .count()
}

fn position(world: &World, unit: Entity) -> Option<Vec3> {
    world
        .get::<Position>(unit)
        .map(|at| Vec3::new(at.x, at.y, at.z))
}
