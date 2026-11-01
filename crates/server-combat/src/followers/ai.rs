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
pub struct FollowerReach(pub f32);

/// A class follower casts only through its spec APL; this brain keeps hazards,
/// target selection, pursuit, pet orders and regrouping.
#[derive(Component)]
pub struct AplControlled;

/// The simulator's headless leader: a party owner that fights with this
/// profile. It owns the party, so it carries no `Follower` component.
#[derive(Component)]
pub(super) struct LeaderProfile(pub(super) super::Profile);

/// `unit` as the follower brain sees it: its `Follower`, or a simulator leader
/// following itself.
pub fn acting_follower(world: &World, unit: Entity) -> Option<Follower> {
    if let Some(follower) = world.get::<Follower>(unit) {
        return Some(follower.clone());
    }
    world.get::<LeaderProfile>(unit).map(|leader| Follower {
        owner: unit,
        profile: leader.0.clone(),
    })
}

pub(super) const MELEE_REACH: f32 = 2.0;

pub(super) fn update(world: &mut World) {
    super::pull_history::sample_all(world);
    let followers: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .filter(|(_, follower)| world.get_entity(follower.owner).is_ok())
        .map(|(entity, follower)| (entity, follower.clone()))
        .collect();
    let owners: std::collections::HashSet<_> = followers
        .iter()
        .map(|(_, follower)| follower.owner)
        .collect();
    for owner in owners {
        super::owner_trail::record(world, owner);
    }
    for (unit, follower) in &followers {
        super::utility::sample_health(world, *unit);
        super::utility::sample_health(world, follower.owner);
    }
    for (unit, follower) in &followers {
        if follower.profile.role == GroupRole::Tank
            && crate::spell_cast::targets::is_alive(world, *unit)
        {
            let party = party_units(world, follower.owner);
            super::tank_frontal::record_observed_frontal(world, *unit, follower, &party);
            let enemies = engaged_enemies(world, follower, *unit, &party);
            if let Some((target, _)) = enemies.first() {
                super::pull_set::log_admission(world, *unit, *target);
                let reach = super::party_unintended_reach(world, &party)
                    .into_iter()
                    .flat_map(|(_, caster, target)| [caster, target])
                    .fold(0.0_f32, f32::max);
                super::tank_pull_back::update(world, *unit, follower, &party, reach);
            } else if super::tank_pull_back::active(world, *unit) {
                super::tank_pull_back::update(world, *unit, follower, &party, 0.0);
            }
        }
    }
    for (unit, follower) in &followers {
        super::positioning::update_combat_exclusions(world, *unit, follower);
    }
    for (unit, follower) in followers {
        if crate::spell_cast::targets::is_alive(world, unit)
            && world
                .get::<crate::creature_ai::flight_follow::FlightFollow>(unit)
                .is_none()
        {
            update_follower(world, unit, &follower);
        }
    }
}

pub(super) fn update_follower(world: &mut World, unit: Entity, follower: &Follower) {
    debug!(target: "game_server::follower_target", ?unit, owner = ?follower.owner, current = ?world.get::<CombatTarget>(unit).and_then(|target| target.0), position = ?world.get::<shared::components::Position>(unit), "follower tick");
    escape_pet_hazard(world, unit);
    if super::hazards::escape(world, unit) || super::candlelight::seek(world, unit) {
        return;
    }
    stop_attacking_cc(world, unit);
    let party = party_units(world, follower.owner);
    if follower.profile.role != GroupRole::Tank
        && super::tank_pull_back::party_should_wait(world, follower.owner)
    {
        wait_for_tank_pull_back(world, unit, &party);
        return;
    }
    super::tank_pull_back_party::restore_pets(world, unit);
    let mut enemies = engaged_enemies(world, follower, unit, &party);
    let pending = super::target_reach::retain_reachable(world, unit, follower, &mut enemies);
    let lost_target = world
        .get::<CombatTarget>(unit)
        .and_then(|target| target.0)
        .is_some_and(|target| !enemies.iter().any(|(enemy, _)| *enemy == target));
    if lost_target {
        if world.get::<CastState>(unit).is_some_and(|cast| {
            Entity::try_from_bits(cast.target).is_some_and(|target| {
                crate::spell_cast::targets::is_hostile(world, unit, target)
                    && !enemies.iter().any(|(enemy, _)| *enemy == target)
            })
        }) {
            crate::spell_cast::cancel_cast(world, unit);
        }
        crate::combat::attack_stop(world, unit);
        hold_combat(world, unit);
    }
    if pending {
        debug!(target: "game_server::follower_target", ?unit, owner = ?follower.owner, "target reachability pending");
        return;
    }
    super::tank_frontal::update_goal(world, unit, follower, &party);
    let collection_target =
        super::tank_planner::update_caster_goal(world, unit, follower, &party, &enemies);
    let selection = collection_target
        .filter(|target| enemies.iter().any(|(enemy, _)| enemy == target))
        .map_or_else(|| select_enemy(world, &enemies), EnemySelection::Attack);
    let enemy = match selection {
        EnemySelection::Attack(enemy) => Some(enemy),
        _ => None,
    };
    trace_target_selection(world, unit, follower.owner, &party, enemy);
    position_for_apl(world, unit, follower, selection);
}

fn wait_for_tank_pull_back(world: &mut World, unit: Entity, party: &[Entity]) {
    super::tank_pull_back_party::stop_damage(world, unit);
    if let Some(tank) = super::tank_pull_back_party::find_tank(world, party) {
        super::tank_pull_back_party::follow_tank(world, unit, tank);
    } else {
        hold_combat(world, unit);
    }
}

fn trace_target_selection(
    world: &World,
    unit: Entity,
    owner: Entity,
    party: &[Entity],
    enemy: Option<Entity>,
) {
    if !bevy::log::tracing::enabled!(target: "game_server::follower_target", bevy::log::tracing::Level::DEBUG)
    {
        return;
    }
    let Some(enemy) = enemy else {
        debug!(target: "game_server::follower_target", ?unit, ?owner, "no selected enemy");
        return;
    };
    let mut combat_party = party.to_vec();
    let map = world.get::<InMap>(unit);
    combat_party.extend(party.iter().filter_map(|&member| {
        crate::hunter_pet::summoned_pet(world, member).filter(|&pet| {
            world.get::<InMap>(pet) == map && crate::spell_cast::targets::is_alive(world, pet)
        })
    }));
    let refs: Vec<_> = combat_party
        .iter()
        .copied()
        .filter(|&member| crate::combat_refs::has_active_reference_to(world, enemy, member))
        .collect();
    let victim = world.get::<CombatTarget>(enemy).and_then(|target| target.0);
    let threat: Vec<_> = world
        .get::<shared::components::ThreatTable>(enemy)
        .into_iter()
        .flat_map(|table| table.entries())
        .filter(|entry| {
            combat_party
                .iter()
                .any(|member| member.to_bits() == entry.entity)
        })
        .map(|entry| entry.entity)
        .collect();
    let owner_victim = world
        .get::<crate::combat::AutoAttack>(owner)
        .is_some_and(|attack| attack.victim == enemy);
    debug!(target: "game_server::follower_target", ?unit, ?owner, ?enemy, ?victim, ?refs, ?threat, owner_victim, "selected enemy");
}

/// TC a352b1fa CasterAI::UpdateAI, CombatAI.cpp:166–177. Stop a cast
/// before completion, not missiles that have already left the caster.
fn stop_attacking_cc(world: &mut World, unit: Entity) {
    let protected_cast = world.get::<CastState>(unit).is_some_and(|cast| {
        Entity::try_from_bits(cast.target).is_some_and(|target| {
            crate::spell_cast::targets::is_hostile(world, unit, target)
                && crate::auras::has_breakable_by_damage_cc(world, target)
        })
    });
    if protected_cast {
        crate::spell_cast::cancel_cast(world, unit);
    }
    let protected_swing = world
        .get::<crate::combat::AutoAttack>(unit)
        .is_some_and(|attack| crate::auras::has_breakable_by_damage_cc(world, attack.victim));
    if protected_swing {
        crate::combat::attack_stop(world, unit);
    }
}

pub(super) fn hold_combat(world: &mut World, unit: Entity) {
    // A controlled enemy is still engaged: never combat_stop or reset its threat.
    world
        .entity_mut(unit)
        .remove::<FollowOwner>()
        .insert((CreatureState::Idle, CombatTarget(None)));
    crate::pet_control::order_companion_pet(world, unit, None);
}

fn escape_pet_hazard(world: &mut World, unit: Entity) {
    let Some(pet) = crate::hunter_pet::summoned_pet(world, unit) else {
        return;
    };
    if super::hazards::escape(world, pet) {
        crate::pet_control::order_companion_pet(world, unit, None);
    }
}

pub(super) fn order_pet_attack(world: &mut World, unit: Entity, enemy: Entity) {
    let Some(pet) = crate::hunter_pet::summoned_pet(world, unit) else {
        return;
    };
    let inside_field = super::hazards::target_is_hazardous(world, pet, pet);
    let Some(unsafe_approach) = super::hazards::approach_is_unsafe(world, pet, enemy) else {
        return;
    };
    let hold = inside_field || unsafe_approach;
    let target = (!hold).then_some(enemy);
    crate::pet_control::order_companion_pet(world, unit, target);
}

fn position_for_apl(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    selection: EnemySelection,
) {
    match selection {
        EnemySelection::Attack(enemy) => {
            let reach = if follower.profile.melee {
                Some(MELEE_REACH)
            } else {
                super::target_reach::attack_range(world, unit, follower)
                    .map(|range| (range - 1.0).max(MELEE_REACH))
            };
            if reach.is_some_and(|reach| pursue_at_reach(world, unit, enemy, reach)) {
                order_pet_attack(world, unit, enemy);
                if needs_taunt(world, unit, enemy, follower)
                    && let Some(spell) = super::class_spawn::known_taunt(world, unit)
                {
                    cast(world, unit, enemy, spell);
                }
            } else {
                regroup(world, unit, follower.owner);
            }
        }
        EnemySelection::Hold => hold_combat(world, unit),
        EnemySelection::None => regroup(world, unit, follower.owner),
    }
}

/// Attackable enemies the follower's party is fighting, in this brain's priority order.
pub fn engaged_targets(world: &mut World, unit: Entity) -> Vec<Entity> {
    let Some(follower) = acting_follower(world, unit) else {
        return Vec::new();
    };
    let party = party_units(world, follower.owner);
    engaged_enemies(world, &follower, unit, &party)
        .into_iter()
        .map(|(enemy, _)| enemy)
        .filter(|&enemy| !crate::auras::has_breakable_by_damage_cc(world, enemy))
        .collect()
}

/// The owner and every follower in the owner's party.
pub fn party_units(world: &mut World, owner: Entity) -> Vec<Entity> {
    let mut units: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .filter(|(_, follower)| follower.owner == owner)
        .map(|(entity, _)| entity)
        .collect();
    units.push(owner);
    units
}

enum EnemySelection {
    Attack(Entity),
    Hold,
    None,
}

fn select_enemy(world: &World, enemies: &[(Entity, Option<Entity>)]) -> EnemySelection {
    let attackable = enemies
        .iter()
        .find(|(enemy, _)| !crate::auras::has_breakable_by_damage_cc(world, *enemy));
    match attackable {
        Some(&(enemy, _)) => EnemySelection::Attack(enemy),
        None if enemies.is_empty() => EnemySelection::None,
        None => EnemySelection::Hold,
    }
}

pub(super) fn engaged_enemies(
    world: &mut World,
    follower: &Follower,
    unit: Entity,
    party: &[Entity],
) -> Vec<(Entity, Option<Entity>)> {
    let Some(&map) = world.get::<InMap>(unit) else {
        return Vec::new();
    };
    // Approved follower pickup policy: a native summoned pet's fight belongs to
    // its party owner. Keep healer/party membership separate from combat coverage.
    let mut combat_party = party.to_vec();
    combat_party.extend(party.iter().filter_map(|&member| {
        crate::hunter_pet::summoned_pet(world, member).filter(|&pet| {
            world.get::<InMap>(pet) == Some(&map)
                && crate::spell_cast::targets::is_alive(world, pet)
        })
    }));
    let selected = world
        .get::<CombatTarget>(follower.owner)
        .and_then(|target| target.0);
    // TC Unit::Attack's victim is distinct from Player::SetTarget selection.
    let owner_victim = world
        .get::<crate::combat::AutoAttack>(follower.owner)
        .map(|attack| attack.victim);
    let mut enemies: Vec<_> = world
        .resource::<crate::maps::units::MapUnits>()
        .in_map(map)
        .filter(|&enemy| {
            world
                .get::<Health>(enemy)
                .is_some_and(|health| health.current > 0.0)
        })
        .map(|enemy| {
            let victim = world.get::<CombatTarget>(enemy).and_then(|target| target.0);
            (enemy, victim)
        })
        .filter(|&(enemy, victim)| {
            owner_victim == Some(enemy)
                || combat_party.iter().any(|&member| {
                    crate::combat_refs::has_active_reference_to(world, enemy, member)
                })
                || victim.is_some_and(|victim| combat_party.contains(&victim))
                || world
                    .get::<shared::components::ThreatTable>(enemy)
                    .is_some_and(|threat| {
                        threat.entries().iter().any(|entry| {
                            combat_party
                                .iter()
                                .any(|member| member.to_bits() == entry.entity)
                        })
                    })
        })
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
    enemies
}

fn needs_taunt(world: &World, tank: Entity, enemy: Entity, follower: &Follower) -> bool {
    follower.profile.role == GroupRole::Tank
        && world
            .get::<CombatTarget>(enemy)
            .and_then(|target| target.0)
            .is_some_and(|victim| victim != tank)
}

pub(super) fn pursue_at_reach(world: &mut World, unit: Entity, target: Entity, reach: f32) -> bool {
    if super::candlelight::hold_pursuit(world, unit, target) {
        return true;
    }
    if super::tank_planner::apply_admitted_position(world, unit, Some(target))
        || super::tank_planner::apply_admitted_caster_target(world, unit)
        || super::tank_planner::hold_unbudgeted_pursuit(world, unit, target)
    {
        return true;
    }
    let in_range_and_visible = crate::spell_cast::targets::position(world, unit)
        .zip(crate::spell_cast::targets::position(world, target))
        .is_some_and(|(from, to)| from.distance(to) <= reach)
        && crate::line_of_sight::sees_in_world(world, unit, target);
    if !in_range_and_visible && super::positioning::hold_unsafe_approach(world, unit, target) {
        world.entity_mut(unit).insert(CombatTarget(Some(target)));
        return true;
    }
    let Some(unsafe_approach) = super::hazards::approach_is_unsafe(world, unit, target) else {
        return true;
    };
    let state = if unsafe_approach {
        CreatureState::Idle
    } else {
        CreatureState::Pursuing {
            target: target.to_bits(),
        }
    };
    world.entity_mut(unit).remove::<FollowOwner>().insert((
        state,
        FollowerReach(reach),
        CombatTarget(Some(target)),
    ));
    true
}

fn cast(world: &mut World, unit: Entity, target: Entity, spell: u32) -> bool {
    if !super::spell_safety::safe(world, unit, target, spell) {
        return false;
    }
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

pub(super) fn regroup(world: &mut World, unit: Entity, owner: Entity) {
    crate::combat::combat_stop(world, unit);
    if super::tank_planner::apply_admitted_position(world, unit, None)
        || super::tank_planner::apply_admitted_caster_target(world, unit)
    {
        return;
    }
    if super::positioning::hold_unsafe_approach(world, unit, owner) {
        world.entity_mut(unit).insert(CombatTarget(None));
        return;
    }
    let Some(unsafe_approach) = super::hazards::approach_is_unsafe(world, unit, owner) else {
        debug!(?unit, ?owner, "Follower regroup waiting for a hazard route");
        return;
    };
    if unsafe_approach {
        debug!(?unit, ?owner, "Follower regroup held by hostile hazard");
        world
            .entity_mut(unit)
            .remove::<FollowOwner>()
            .insert((CreatureState::Idle, CombatTarget(None)));
        return;
    }
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
