//! Who is in combat with whom: TrinityCore's `CombatManager` (Combat/CombatManager.h:27-46).
//! "A unit is 'in combat' iff it has one or more non-suppressed CombatReference
//! entries in its CombatManager": each pair of units fighting holds one reference, and
//! `CombatStatus` is whether a unit holds any.
//!
//! A reference between a creature that holds a fight (one with a threat list) and
//! another unit has no timeout: it lasts until either side stops combat
//! (`combat::combat_stop`: death, evade, map change). A reference between two players
//! is PvP and ends `PVP_COMBAT_TIMEOUT_SECS` after it was last refreshed
//! (`PvPCombatReference::PVP_COMBAT_TIMEOUT`, CombatManager.h:84). Creatures without a
//! threat list (triggers, critters, training dummies: `AggroMode::Passive`) have no AI
//! here that would end their fights, so their references time out the same way, as TC
//! `npc_training_dummy` ends each attacker's combat 5 s after its last hit
//! (npcs_special.cpp:1382-1415).
//!
//! A harmful spell with initial aggro starts combat when it launches
//! (`spell_cast::starts_combat`, Spell.cpp:8538-8541), before any damage. The reference
//! it makes is suppressed on the target's side ("This will only cause combat - the
//! target will engage once the projectile hits"): the caster is in combat, the target
//! is not until the spell lands and refreshes the reference (Spell.cpp:2763-2764,
//! `CombatReference::Refresh`, CombatManager.cpp:88-106).
//!
//! Not modelled: the other suppressions (vanish, feign death), pets and guardians.

use std::collections::HashMap;

use bevy::prelude::*;
use shared::components::{CombatStatus, Health, Player, ThreatTable};

use crate::creature_ai::CreatureState;

/// `PvPCombatReference::PVP_COMBAT_TIMEOUT` (CombatManager.h:84).
pub(crate) const PVP_COMBAT_TIMEOUT_SECS: f64 = 5.0;

/// A unit's combat references by the other unit.
#[derive(Component, Debug, Default)]
pub(crate) struct CombatRefs(HashMap<Entity, CombatRef>);

/// One side of a combat reference.
#[derive(Clone, Copy, Debug)]
struct CombatRef {
    /// When a timed reference ends; `None` for a PvE reference, which never times out.
    expires_at: Option<f64>,
    /// Held without putting this side in combat (`CombatReference::IsSuppressedFor`).
    suppressed: bool,
}

/// `CombatManager::SetInCombatWith`: `a` and `b` are in combat with each other, and a
/// timed reference between them restarts its timeout (`RefreshTimer`). Refused like
/// `CombatManager::CanBeginCombat` (CombatManager.cpp:24-58) for the same unit twice, a
/// dead unit or an evading creature.
pub(crate) fn set_in_combat_with(world: &mut World, a: Entity, b: Entity) {
    begin_combat(world, a, b, false);
}

/// `SetInCombatWith(b, addSecondUnitSuppressed = true)` (CombatManager.cpp:191-218): a new
/// reference puts only `a` in combat; `b` holds it suppressed. An existing reference is
/// refreshed instead, which lifts both sides' suppression.
pub(crate) fn set_in_combat_with_suppressed(world: &mut World, a: Entity, b: Entity) {
    begin_combat(world, a, b, true);
}

fn begin_combat(world: &mut World, a: Entity, b: Entity, suppress_b: bool) {
    if !can_begin_combat(world, a, b) {
        return;
    }
    let existing = world
        .get::<CombatRefs>(a)
        .is_some_and(|refs| refs.0.contains_key(&b));
    let timed = !holds_fight(world, a) || !holds_fight(world, b) || both_players(world, a, b);
    let expires_at =
        timed.then(|| world.resource::<Time>().elapsed_secs_f64() + PVP_COMBAT_TIMEOUT_SECS);
    for (unit, other, suppressed) in [(a, b, false), (b, a, suppress_b && !existing)] {
        world
            .entity_mut(unit)
            .entry::<CombatRefs>()
            .or_default()
            .get_mut()
            .0
            .insert(
                other,
                CombatRef {
                    expires_at,
                    suppressed,
                },
            );
        update_combat_status(world, unit);
    }
}

/// `CombatManager::EndAllCombat` (`Unit::CombatStop` → `EndAllPvECombat` and
/// `EndAllPvPCombat`): every reference of `unit` ends on both sides, and each side
/// left without one leaves combat.
pub(crate) fn end_all_combat(world: &mut World, unit: Entity) {
    let others: Vec<Entity> = world
        .get_mut::<CombatRefs>(unit)
        .map(|mut refs| refs.0.drain().map(|(other, _)| other).collect())
        .unwrap_or_default();
    for other in others {
        if let Some(mut refs) = world.get_mut::<CombatRefs>(other) {
            refs.0.remove(&unit);
            update_combat_status(world, other);
        }
    }
    if world.get_entity(unit).is_ok() {
        world.entity_mut(unit).insert(CombatStatus(false));
    }
}

/// TC EffectSanctuary/CombatStop(false,false): end PvE on both sides,
/// suppress only this player's side of retained PvP references.
pub(crate) fn sanctuary(world: &mut World, unit: Entity) {
    let others: Vec<Entity> = world
        .get::<CombatRefs>(unit)
        .map(|refs| refs.0.keys().copied().collect())
        .unwrap_or_default();
    for other in others {
        if both_players(world, unit, other) {
            if let Some(mut refs) = world.get_mut::<CombatRefs>(unit) {
                refs.0.get_mut(&other).expect("listed reference").suppressed = true;
            }
            continue;
        }
        if let Some(mut refs) = world.get_mut::<CombatRefs>(unit) {
            refs.0.remove(&other);
        }
        if let Some(mut refs) = world.get_mut::<CombatRefs>(other) {
            refs.0.remove(&unit);
        }
        if let Some(mut threat) = world.get_mut::<ThreatTable>(other) {
            threat.remove(unit.to_bits());
        }
        update_combat_status(world, other);
    }
    update_combat_status(world, unit);
}

/// `CombatManager::Update` (CombatManager.cpp:139-157): timed references that ran out
/// end, as do references to units that left the world.
pub(crate) fn expire_combat_refs_system(world: &mut World) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let mut units = world.query::<(Entity, &CombatRefs)>();
    let expired: Vec<(Entity, Vec<Entity>)> = units
        .iter(world)
        .map(|(unit, refs)| {
            let gone: Vec<Entity> = refs
                .0
                .iter()
                .filter(|&(&other, reference)| {
                    reference.expires_at.is_some_and(|at| now >= at)
                        || world.get_entity(other).is_err()
                })
                .map(|(&other, _)| other)
                .collect();
            (unit, gone)
        })
        .filter(|(_, gone)| !gone.is_empty())
        .collect();
    for (unit, gone) in expired {
        let mut refs = world.get_mut::<CombatRefs>(unit).expect("queried");
        for other in gone {
            refs.0.remove(&other);
        }
        update_combat_status(world, unit);
    }
}

fn can_begin_combat(world: &World, a: Entity, b: Entity) -> bool {
    let dead = |unit: Entity| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current <= 0.0)
    };
    let evading =
        |unit: Entity| world.get::<CreatureState>(unit) == Some(&CreatureState::Returning);
    a != b && !dead(a) && !dead(b) && !evading(a) && !evading(b)
}

/// A player, or a creature with a threat list, whose AI ends its fights.
fn holds_fight(world: &World, unit: Entity) -> bool {
    world.get::<Player>(unit).is_some() || world.get::<ThreatTable>(unit).is_some()
}

fn both_players(world: &World, a: Entity, b: Entity) -> bool {
    world.get::<Player>(a).is_some() && world.get::<Player>(b).is_some()
}

/// `CombatManager::UpdateOwnerCombatState`: `CombatStatus` follows whether `unit` holds
/// a reference not suppressed for it, written only when it changes.
fn update_combat_status(world: &mut World, unit: Entity) {
    let in_combat = world
        .get::<CombatRefs>(unit)
        .is_some_and(|refs| refs.0.values().any(|reference| !reference.suppressed));
    let current = world.get::<CombatStatus>(unit).map(|status| status.0);
    if current != Some(in_combat) {
        world.entity_mut(unit).insert(CombatStatus(in_combat));
    }
}

#[cfg(test)]
#[path = "combat_refs_tests.rs"]
mod tests;
