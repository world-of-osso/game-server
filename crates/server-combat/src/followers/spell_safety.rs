//! Avoid secondary spell hits on unrelated idle hostiles.
use bevy::prelude::*;
use shared::components::{CombatStatus, Npc};

use crate::creature_ai::CreatureState;
use crate::maps::InMap;
use crate::spell_cast::targets;

pub(super) fn safe(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    action_safe(world, caster, target, Some(spell))
}

/// Shared admission for a follower spell or a follower-owned pet's direct attack.
pub fn action_safe(world: &World, actor: Entity, target: Entity, spell: Option<u32>) -> bool {
    let follower = world
        .get::<super::Follower>(actor)
        .map(|_| actor)
        .or_else(|| {
            let owner = world
                .get::<crate::hunter_pet::HunterPet>(actor)
                .map(|pet| pet.owner)
                .or_else(|| {
                    world
                        .get::<crate::spell_cast::summon::SummonedCreature>(actor)
                        .map(|pet| pet.owner)
                })?;
            world.get::<super::Follower>(owner).map(|_| owner)
        });
    let Some(follower) = follower else {
        return true;
    };
    let enemy_target = targets::relation_matches(world, actor, target, targets::Relation::Enemy);
    let primary = world
        .get::<crate::networking::CombatTarget>(follower)
        .and_then(|target| target.0)
        .or_else(|| enemy_target.then_some(target));
    let pending = primary
        .map(|primary| super::pull_set::pending_helpers(world, primary, actor))
        .unwrap_or_default();
    if enemy_target && pending.contains(&target) {
        return false;
    }
    let Some(spell) = spell else { return true };
    let Some((caster_radius, target_radius)) =
        super::unintended_reach::spell_unintended_reach(world, actor, spell, target)
    else {
        return false;
    };
    if caster_radius <= 0.0 && target_radius <= 0.0 {
        return true;
    }
    if !pending.is_empty() {
        return false;
    }
    let caster = actor;
    let Some(map) = world.get::<InMap>(caster).copied() else {
        return false;
    };
    let Some(caster_position) = targets::position(world, caster) else {
        return false;
    };
    let Some(target_position) = targets::position(world, target) else {
        return false;
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
