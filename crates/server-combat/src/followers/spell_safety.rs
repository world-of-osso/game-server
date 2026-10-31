//! Avoid secondary spell hits on unrelated idle hostiles.
use bevy::prelude::*;
use shared::components::{CombatStatus, Npc};

use crate::creature_ai::CreatureState;
use crate::maps::InMap;
use crate::spell_cast::targets;

pub(super) fn safe(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    action_safe(world, caster, target, Some(spell))
}

fn controlling_follower(world: &World, actor: Entity) -> Option<Entity> {
    world
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
        })
}

/// Admit native chain recipients only from the follower's planned first wave.
pub fn chain_candidate_allowed(
    world: &World,
    actor: Entity,
    primary: Entity,
    candidate: Entity,
) -> bool {
    let Some(follower) = controlling_follower(world, actor) else {
        return true;
    };
    let caller = world
        .get::<crate::networking::CombatTarget>(follower)
        .and_then(|target| target.0)
        .unwrap_or(primary);
    super::pull_set::members(world, caller, actor).contains(&candidate)
        && !super::pull_set::pending_helpers(world, caller, actor).contains(&candidate)
}

/// Shared admission for a follower spell or a follower-owned pet's direct attack.
pub fn action_safe(world: &World, actor: Entity, target: Entity, spell: Option<u32>) -> bool {
    let Some(follower) = controlling_follower(world, actor) else {
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
        debug!(target: "game_server::follower_safety", ?actor, ?target, ?spell, ?primary, ?pending, reason = "pending_target", "follower action refused");
        return false;
    }
    let Some(spell) = spell else { return true };
    let Some((caster_radius, target_radius)) =
        super::unintended_reach::spell_unintended_reach(world, actor, spell, target)
    else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_unintended_reach", "follower action refused");
        return false;
    };
    if caster_radius <= 0.0 && target_radius <= 0.0 {
        return true;
    }
    if !pending.is_empty() {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, ?primary, ?pending, caster_radius, target_radius, reason = "pending_helpers", "follower action refused");
        return false;
    }
    let Some((caster_radius, target_radius)) =
        super::unintended_reach::spell_reach_without_chain(world, actor, spell, target)
    else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_nonchain_reach", "follower action refused");
        return false;
    };
    if caster_radius <= 0.0 && target_radius <= 0.0 {
        return true;
    }
    let caster = actor;
    let Some(map) = world.get::<InMap>(caster).copied() else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_map", "follower action refused");
        return false;
    };
    let Some(caster_position) = targets::position(world, caster) else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_caster_position", "follower action refused");
        return false;
    };
    let Some(target_position) = targets::position(world, target) else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_target_position", "follower action refused");
        return false;
    };
    let planned = primary
        .map(|primary| super::pull_set::members(world, primary, caster))
        .unwrap_or_default();
    let outsider = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .find(|&actor| {
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
        });
    if let Some(outsider) = outsider {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, ?primary, ?outsider, caster_radius, target_radius, reason = "idle_outsider", "follower action refused");
        return false;
    }
    true
}
