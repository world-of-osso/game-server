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

pub(super) const MELEE_REACH: f32 = 2.0;

pub(super) fn update(world: &mut World) {
    super::pull_history::sample_all(world);
    let followers: Vec<_> = world
        .query::<(Entity, &Follower)>()
        .iter(world)
        .map(|(entity, follower)| (entity, follower.clone()))
        .collect();
    for (unit, follower) in &followers {
        if follower.profile.role == GroupRole::Tank
            && crate::spell_cast::targets::is_alive(world, *unit)
        {
            let party = party_units(world, follower.owner);
            super::tank_frontal::record_observed_frontal(world, *unit, follower, &party);
        }
    }
    for (unit, follower) in followers {
        if crate::spell_cast::targets::is_alive(world, unit) {
            update_follower(world, unit, &follower);
        }
    }
}

fn update_follower(world: &mut World, unit: Entity, follower: &Follower) {
    escape_pet_hazard(world, unit);
    if super::hazards::escape(world, unit) {
        return;
    }
    stop_attacking_cc(world, unit);
    let party = party_units(world, follower.owner);
    super::tank_frontal::update_goal(world, unit, follower, &party);
    let collection_target = super::tank_planner::update_caster_goal(world, unit, follower, &party);
    let selection = collection_target.map_or_else(
        || select_enemy(world, follower, unit, &party),
        EnemySelection::Attack,
    );
    let enemy = match selection {
        EnemySelection::Attack(enemy) => Some(enemy),
        _ => None,
    };
    if super::kit_ai::update(world, unit, follower, &party, enemy) {
        return;
    }
    if enemy.is_some_and(|enemy| interrupt_enemy(world, unit, enemy, follower.profile.spec_id)) {
        return;
    }
    if enemy.is_some_and(|enemy| defend_follower(world, unit, enemy, follower.profile.class_id)) {
        return;
    }
    if world.get::<CastState>(unit).is_some() {
        return;
    }
    if prepare_pet(world, unit, follower) {
        return;
    }
    if heal_party_member(world, unit, follower, &party) {
        return;
    }
    let Some(enemy) = enemy else {
        if matches!(selection, EnemySelection::Hold) {
            hold_combat(world, unit);
        } else {
            regroup(world, unit, follower.owner);
        }
        return;
    };
    attack_enemy(world, unit, follower, enemy);
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

pub(super) fn prepare_pet(world: &mut World, unit: Entity, follower: &Follower) -> bool {
    // The approved player-spec bridge's Demonology summon names its own creature
    // in SpellEffect; Hunter's empty stable never receives a guessed pet.
    if follower.profile.spec_id != 266 || crate::hunter_pet::summoned_pet(world, unit).is_some() {
        return false;
    }
    match crate::spell_cast::creature_cast(world, unit, 30146, Some(unit), false) {
        Ok(()) => true,
        Err(failure) => {
            warn!("Follower {unit} cannot prepare authored Felguard: {failure:?}");
            true
        }
    }
}

fn heal_party_member(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
) -> bool {
    if follower.profile.heal_spell == 0 {
        return false;
    }
    let Some(patient) = lowest_health(world, party, unit) else {
        return false;
    };
    if pursue(world, unit, patient, follower.profile.heal_spell, false) {
        cast(world, unit, patient, follower.profile.heal_spell);
    } else {
        regroup(world, unit, follower.owner);
    }
    true
}

fn attack_enemy(world: &mut World, unit: Entity, follower: &Follower, enemy: Entity) {
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
    order_pet_attack(world, unit, enemy);
    if needs_taunt(world, unit, enemy, follower)
        && cast(world, unit, enemy, follower.profile.taunt_spell)
    {
        return;
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

enum EnemySelection {
    Attack(Entity),
    Hold,
    None,
}

fn select_enemy(
    world: &mut World,
    follower: &Follower,
    unit: Entity,
    party: &[Entity],
) -> EnemySelection {
    let enemies = engaged_enemies(world, follower, unit, party);
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
    let selected = world
        .get::<CombatTarget>(follower.owner)
        .and_then(|target| target.0);
    // TC Unit::Attack's victim is distinct from Player::SetTarget selection.
    let owner_victim = world
        .get::<crate::combat::AutoAttack>(follower.owner)
        .map(|attack| attack.victim);
    let mut enemies: Vec<_> = world
        .query::<(Entity, &Health, &InMap, Option<&CombatTarget>)>()
        .iter(world)
        .filter(|(_, health, other, _)| **other == map && health.current > 0.0)
        .filter(|(enemy, _, _, target)| {
            owner_victim == Some(*enemy)
                || target
                    .and_then(|target| target.0)
                    .is_some_and(|victim| party.contains(&victim))
                || world
                    .get::<shared::components::ThreatTable>(*enemy)
                    .is_some_and(|threat| {
                        threat.entries().iter().any(|entry| {
                            party.iter().any(|member| member.to_bits() == entry.entity)
                        })
                    })
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
    enemies
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

pub(super) fn interrupt_enemy(world: &mut World, unit: Entity, enemy: Entity, spec: u32) -> bool {
    let interruptible = world
        .get::<CastState>(enemy)
        .is_some_and(|cast| cast.interruptible);
    if !interruptible {
        return false;
    }
    let Some((caster, spell)) = interrupt_source(world, unit, spec) else {
        return false;
    };
    if caster != unit && super::hazards::target_is_hazardous(world, caster, caster) {
        return false;
    }
    match crate::spell_cast::creature_cast_interrupting_self(world, caster, spell, Some(enemy)) {
        Ok(()) => true,
        Err(failure) => {
            debug!("Follower {unit} interrupt {spell} at {enemy}: {failure:?}");
            false
        }
    }
}

/// CreatureFamily29/SkillLine761 teaches Felguard Axe Toss89766 at17; source
/// SpellEffect64 triggers the real347008 effect68. Never cast an unlearned pet spell.
fn interrupt_source(world: &World, unit: Entity, spec: u32) -> Option<(Entity, u32)> {
    if spec != 266 {
        return interrupt_spell(spec).map(|spell| (unit, spell));
    }
    let pet = crate::hunter_pet::summoned_pet(world, unit)?;
    let charm = world.get::<crate::pet_control::PetCharm>(pet)?;
    charm.spell_slot(89766).map(|_| (pet, 89766))
}

/// Approved bridge policy: defend a wounded companion being actively attacked.
/// No invented health-percentage threshold or NPC-kit parity claim.
fn defend_follower(world: &mut World, unit: Entity, enemy: Entity, class: u8) -> bool {
    let wounded = world
        .get::<Health>(unit)
        .is_some_and(|health| health.current < health.max);
    let attacked = world
        .get::<CombatTarget>(enemy)
        .is_some_and(|target| target.0 == Some(unit));
    if !wounded || !attacked {
        return false;
    }
    let Some(spell) = defensive_spell(class) else {
        return false;
    };
    match crate::spell_cast::creature_cast_interrupting_self(world, unit, spell, Some(unit)) {
        Ok(()) => true,
        Err(failure) => {
            debug!("Follower {unit} defensive {spell}: {failure:?}");
            false
        }
    }
}

fn defensive_spell(class: u8) -> Option<u32> {
    Some(match class {
        1 => 871,     // Shield Wall.
        2 => 498,     // Divine Protection.
        3 => 186265,  // Aspect of the Turtle.
        5 => 19236,   // Desperate Prayer.
        7 => 108271,  // Astral Shift.
        8 => 45438,   // Ice Block.
        9 => 104773,  // Unending Resolve.
        10 => 115203, // Fortifying Brew.
        11 => 22812,  // Barkskin.
        13 => 363916, // Obsidian Scales.
        _ => return None,
    })
}

fn needs_taunt(world: &World, tank: Entity, enemy: Entity, follower: &Follower) -> bool {
    follower.profile.taunt_spell != 0
        && world
            .get::<CombatTarget>(enemy)
            .and_then(|target| target.0)
            .is_some_and(|victim| victim != tank)
}

pub(super) fn pursue(
    world: &mut World,
    unit: Entity,
    target: Entity,
    spell_id: u32,
    melee: bool,
) -> bool {
    if super::tank_planner::apply_admitted_position(world, unit, Some(target))
        || super::tank_planner::apply_admitted_caster_target(world, unit)
        || super::tank_planner::hold_unbudgeted_pursuit(world, unit, target)
    {
        return true;
    }
    let reach = if melee {
        Some(MELEE_REACH)
    } else {
        spell_reach(world, spell_id)
    };
    let Some(reach) = reach else {
        warn!("Follower {unit} cannot pursue: spell {spell_id} lacks authored range metadata");
        return false;
    };
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

pub(super) fn regroup(world: &mut World, unit: Entity, owner: Entity) {
    crate::combat::combat_stop(world, unit);
    if super::tank_planner::apply_admitted_position(world, unit, None)
        || super::tank_planner::apply_admitted_caster_target(world, unit)
    {
        return;
    }
    let Some(unsafe_approach) = super::hazards::approach_is_unsafe(world, unit, owner) else {
        return;
    };
    if unsafe_approach {
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
