use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Position};
use shared::group::GroupRole;

use super::Follower;
use crate::creature_ai::{CreatureState, FollowOwner};
use crate::maps::InMap;
use crate::networking::CombatTarget;

/// Navigation's stopping distance, from the selected player's spell range.
#[derive(Component, Clone, Copy)]
pub(crate) struct FollowerReach(pub f32);

pub(super) fn update(world: &mut World) {
    let followers: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .map(|(entity, follower)| (entity, follower.clone()))
        .collect();
    for (unit, follower) in followers {
        if crate::spell_cast::targets::is_alive(world, unit) {
            update_follower(world, unit, &follower);
        }
    }
}

fn update_follower(world: &mut World, unit: Entity, follower: &Follower) {
    if world.get::<CastState>(unit).is_some() {
        return;
    }
    let party = party_units(world, follower.owner);
    if follower.profile.heal_spell != 0 {
        if let Some(patient) = lowest_health(world, &party, unit) {
            if !pursue(world, unit, patient, follower.profile.heal_spell, false) {
                regroup(world, unit, follower.owner);
                return;
            }
            cast(world, unit, patient, follower.profile.heal_spell);
            return;
        }
    }
    let Some(enemy) = select_enemy(world, follower, unit, &party) else {
        regroup(world, unit, follower.owner);
        return;
    };
    if interrupt_enemy(world, unit, enemy, follower.profile.spec_id) {
        return;
    }
    if !pursue(
        world,
        unit,
        enemy,
        follower.profile.attack_spell,
        follower.profile.melee,
    ) {
        regroup(world, unit, follower.owner);
        return;
    }
    if needs_taunt(world, unit, enemy, follower) {
        if cast(world, unit, enemy, follower.profile.taunt_spell) {
            return;
        }
    }
    if follower.profile.cooldown_spell != 0
        && cast(world, unit, enemy, follower.profile.cooldown_spell)
    {
        return;
    }
    if follower.profile.attack_spell != 0 {
        cast(world, unit, enemy, follower.profile.attack_spell);
    }
}

pub(super) fn party_units(world: &mut World, owner: Entity) -> Vec<Entity> {
    let mut units: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .filter(|(_, follower)| follower.owner == owner)
        .map(|(entity, _)| entity)
        .collect();
    units.push(owner);
    units
}

fn lowest_health(world: &World, party: &[Entity], healer: Entity) -> Option<Entity> {
    party
        .iter()
        .copied()
        .filter(|unit| world.get::<InMap>(*unit) == world.get::<InMap>(healer))
        .filter_map(|unit| {
            let health = world.get::<Health>(unit)?;
            let injured = health.current > 0.0 && health.current < health.max;
            injured.then_some((unit, health.current / health.max))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(unit, _)| unit)
}

fn select_enemy(
    world: &mut World,
    follower: &Follower,
    unit: Entity,
    party: &[Entity],
) -> Option<Entity> {
    let map = *world.get::<InMap>(unit)?;
    let selected = world
        .get::<CombatTarget>(follower.owner)
        .and_then(|target| target.0);
    let mut enemies: Vec<_> = world
        .query::<(Entity, &Health, &InMap, Option<&CombatTarget>)>()
        .iter(world)
        .filter(|(_, health, other, _)| **other == map && health.current > 0.0)
        .filter(|(enemy, _, _, target)| {
            selected == Some(*enemy)
                || target
                    .and_then(|target| target.0)
                    .is_some_and(|victim| party.contains(&victim))
        })
        .map(|(enemy, _, _, target)| (enemy, target.and_then(|target| target.0)))
        .collect();
    enemies.retain(|(enemy, _)| {
        crate::spell_cast::targets::relation_matches(
            world,
            unit,
            *enemy,
            crate::spell_cast::targets::Relation::Enemy,
        )
    });
    enemies.sort_by_key(|(enemy, victim)| {
        let lost_threat =
            follower.profile.role == GroupRole::Tank && victim.is_some_and(|victim| victim != unit);
        (!lost_threat, selected != Some(*enemy), enemy.to_bits())
    });
    enemies.first().map(|(enemy, _)| *enemy)
}

/// Approved player-spec bridge, not authored NPC kits. IDs/names and effect68
/// are verified in local SpellName/SpellEffect12.1.0.69933 (Skull Bash's
/// effect68 child93985 is referenced by its source description). Holy/Restoration
/// have no selected kick; Demonology's pet interrupt is not a player cast.
fn interrupt_spell(spec: u32) -> Option<u32> {
    match spec {
        66 => Some(96231),    // Protection Paladin: Rebuke.
        73 => Some(6552),     // Protection Warrior: Pummel.
        63 => Some(2139),     // Fire Mage: Counterspell.
        262 => Some(57994),   // Elemental Shaman: Wind Shear.
        253 => Some(147362),  // Beast Mastery Hunter: Counter Shot.
        1467 => Some(351338), // Devastation Evoker: Quell.
        269 => Some(116705),  // Windwalker Monk: Spear Hand Strike.
        103 => Some(106839),  // Feral Druid: Skull Bash.
        _ => None,
    }
}

fn interrupt_enemy(world: &mut World, unit: Entity, enemy: Entity, spec: u32) -> bool {
    let interruptible = world
        .get::<CastState>(enemy)
        .is_some_and(|cast| cast.interruptible);
    if !interruptible {
        return false;
    }
    let Some(spell) = interrupt_spell(spec) else {
        return false;
    };
    cast(world, unit, enemy, spell)
}

fn needs_taunt(world: &World, tank: Entity, enemy: Entity, follower: &Follower) -> bool {
    follower.profile.taunt_spell != 0
        && world
            .get::<CombatTarget>(enemy)
            .and_then(|target| target.0)
            .is_some_and(|victim| victim != tank)
}

fn pursue(world: &mut World, unit: Entity, target: Entity, spell_id: u32, melee: bool) -> bool {
    let reach = if melee {
        Some(2.0)
    } else {
        spell_reach(world, spell_id)
    };
    let Some(reach) = reach else {
        warn!("Follower {unit} cannot pursue: spell {spell_id} lacks authored range metadata");
        return false;
    };
    world.entity_mut(unit).remove::<FollowOwner>().insert((
        CreatureState::Pursuing {
            target: target.to_bits(),
        },
        FollowerReach(reach),
        CombatTarget(Some(target)),
    ));
    true
}

fn spell_reach(world: &World, spell_id: u32) -> Option<f32> {
    world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .and_then(|spell| spell.misc.as_ref())
        .and_then(|misc| misc.range)
        .map(|range| range.max_hostile.max(range.max_friendly))
        .filter(|range| *range > 0.0)
        .map(|range| (range - 1.0).max(2.0))
}

fn cast(world: &mut World, unit: Entity, target: Entity, spell: u32) -> bool {
    match crate::spell_cast::creature_cast(world, unit, spell, Some(target), false) {
        Ok(()) => true,
        Err(
            crate::spell_cast::CastFailure::OnGlobalCooldown
            | crate::spell_cast::CastFailure::InProgress,
        ) => true,
        Err(failure) => {
            debug!("Follower {unit} spell {spell} at {target}: {failure:?}");
            false
        }
    }
}

fn regroup(world: &mut World, unit: Entity, owner: Entity) {
    crate::combat::combat_stop(world, unit);
    let following_owner = world
        .get::<FollowOwner>(unit)
        .is_some_and(|follow| follow.owner == owner);
    world
        .entity_mut(unit)
        .insert((CreatureState::Idle, CombatTarget(None)));
    if !following_owner {
        world.entity_mut(unit).insert(FollowOwner::new(owner));
    }
    if let Some(position) = world.get::<Position>(unit).copied() {
        if let Some(mut home) = world.get_mut::<crate::creature_ai::SpawnPoint>(unit) {
            home.x = position.x;
            home.y = position.y;
            home.z = position.z;
        }
    }
}
