//! Avoid secondary spell hits on unrelated idle hostiles.
use bevy::prelude::*;
use shared::components::{CombatStatus, Npc};

use crate::creature_ai::CreatureState;
use crate::maps::InMap;
use crate::spell_cast::targets;

pub(super) fn safe(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    let Some((caster_radius, target_radius)) =
        super::unintended_reach::action_unintended_reach(world, caster, spell, target)
    else {
        return false;
    };
    if caster_radius <= 0.0 && target_radius <= 0.0 {
        return true;
    }
    let Some(map) = world.get::<InMap>(caster).copied() else {
        return false;
    };
    let Some(caster_position) = targets::position(world, caster) else {
        return false;
    };
    let Some(target_position) = targets::position(world, target) else {
        return false;
    };
    let primary = if targets::relation_matches(world, caster, target, targets::Relation::Enemy) {
        Some(target)
    } else {
        world
            .get::<crate::networking::CombatTarget>(caster)
            .and_then(|target| target.0)
    };
    let planned = primary
        .map(|primary| super::pull_set::members(world, primary, caster))
        .unwrap_or_default();
    !world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .any(|actor| {
            actor != target
                && !planned.contains(&actor)
                && targets::is_alive(world, actor)
                && world.get::<Npc>(actor).is_some()
                && world
                    .get::<CreatureState>(actor)
                    .is_some_and(CreatureState::is_idle)
                && !world
                    .get::<CombatStatus>(actor)
                    .is_some_and(|status| status.0)
                && crate::combat_refs::active_opponents(world, actor)
                    .next()
                    .is_none()
                && targets::relation_matches(world, caster, actor, targets::Relation::Enemy)
                && targets::position(world, actor).is_none_or(|point| {
                    (caster_radius > 0.0 && caster_position.distance(point) <= caster_radius)
                        || (target_radius > 0.0 && target_position.distance(point) <= target_radius)
                })
        })
}
