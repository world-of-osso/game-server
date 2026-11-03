//! Avoid secondary spell hits on hostiles unrelated to the follower's party.
use crate::maps::InMap;
use crate::spell_cast::script_registry::DeclaredReach;
use crate::spell_cast::targets;
use bevy::prelude::*;
use shared::components::Npc;

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

/// Admit native chain recipients from the follower's planned first wave or
/// enemies the party already fights; unengaged enemies stay excluded.
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
    let planned = super::pull_set::members(world, caller, actor).contains(&candidate)
        && !super::pull_set::pending_helpers(world, caller, actor).contains(&candidate);
    planned || engaged_with_party(world, follower, candidate)
}

/// `enemy` is in combat with the follower's party.
fn engaged_with_party(world: &World, follower: Entity, enemy: Entity) -> bool {
    let Some(owner) = world.get::<super::Follower>(follower).map(|f| f.owner) else {
        return false;
    };
    crate::combat_refs::active_opponents(world, enemy).any(|opponent| {
        opponent == owner
            || world
                .get::<super::Follower>(opponent)
                .is_some_and(|member| member.owner == owner)
    })
}

/// A scripted summon action must not launch if its native targets include an
/// unrelated hostile. Check the whole cast without changing its target list.
pub(crate) fn summon_targets_safe(
    world: &World,
    actor: Entity,
    primary: Option<Entity>,
    targets: impl IntoIterator<Item = Entity>,
) -> bool {
    let Some(follower) = controlling_follower(world, actor).filter(|follower| *follower != actor)
    else {
        return true;
    };
    let caller = world
        .get::<crate::networking::CombatTarget>(follower)
        .and_then(|target| target.0)
        .or(primary);
    targets.into_iter().all(|target| {
        world.get::<Npc>(target).is_none()
            || !targets::relation_matches(world, actor, target, targets::Relation::Enemy)
            || engaged_with_party(world, follower, target)
            || caller.is_some_and(|caller| {
                engaged_with_party(world, follower, caller)
                    && chain_candidate_allowed(world, actor, caller, target)
            })
    })
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
    let Some(reach) = super::unintended_reach::action_reach(world, actor, spell, target, true)
    else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_unintended_reach", "follower action refused");
        return false;
    };
    if reach_is_empty(reach) {
        return true;
    }
    if pending
        .iter()
        .any(|&helper| reaches_unit(world, reach, actor, target, helper))
    {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, ?primary, ?pending, ?reach, reason = "pending_helper_in_reach", "follower action refused");
        return false;
    }
    let Some(reach) = super::unintended_reach::action_reach(world, actor, spell, target, false)
    else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_nonchain_reach", "follower action refused");
        return false;
    };
    if reach_is_empty(reach) {
        return true;
    }
    let caster = actor;
    let Some(map) = world.get::<InMap>(caster).copied() else {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_map", "follower action refused");
        return false;
    };
    if targets::position(world, caster).is_none() {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_caster_position", "follower action refused");
        return false;
    }
    if targets::position(world, target).is_none() {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, reason = "missing_target_position", "follower action refused");
        return false;
    }
    let mut planned = primary
        .map(|primary| super::pull_set::members(world, primary, caster))
        .unwrap_or_default();
    planned.extend(super::pull_set::committed_helpers(world, caster));
    let outsider = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .find(|&actor| {
            actor != target
                && !planned.contains(&actor)
                && targets::is_alive(world, actor)
                && world.get::<Npc>(actor).is_some()
                && !engaged_with_party(world, follower, actor)
                && targets::relation_matches(world, caster, actor, targets::Relation::Enemy)
                && reaches_unit(world, reach, caster, target, actor)
        });
    if let Some(outsider) = outsider {
        debug!(target: "game_server::follower_safety", ?actor, ?target, spell, ?primary, ?outsider, ?reach, reason = "unengaged_outsider", "follower action refused");
        return false;
    }
    true
}

fn reach_is_empty(reach: DeclaredReach) -> bool {
    reach.caster_radius <= 0.0 && reach.target_radius <= 0.0 && reach.beam_half_width <= 0.0
}

/// A missing position or unsourced beam size counts as reached.
fn reaches_unit(
    world: &World,
    reach: DeclaredReach,
    caster: Entity,
    target: Entity,
    unit: Entity,
) -> bool {
    let Some(point) = targets::position(world, unit) else {
        return true;
    };
    let caster_position = targets::position(world, caster);
    let target_position = targets::position(world, target);
    let within = |center: Option<Vec3>, radius: f32| {
        radius > 0.0 && center.is_none_or(|center| center.distance(point) <= radius)
    };
    within(caster_position, reach.caster_radius)
        || within(target_position, reach.target_radius)
        || (reach.beam_half_width > 0.0
            && caster_position
                .zip(target_position)
                .and_then(|(origin, end)| {
                    targets::within_beam(world, origin, end, reach.beam_half_width, unit)
                })
                .unwrap_or(true))
}
