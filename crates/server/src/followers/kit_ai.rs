//! Execute named project priorities without changing spell costs, timers or movement.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Level};

use super::kit::{Entry, Slot, Target};
use super::{Follower, FollowerCatalog, ai};
use crate::maps::InMap;
use crate::networking::CombatTarget;

const PROJECT_URGENT_HEAL_THRESHOLD: f32 = 0.40;

#[cfg(test)]
#[path = "healer_triage_tests.rs"]
mod healer_triage_tests;

pub(super) fn update(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
    enemy: Option<Entity>,
) -> bool {
    let level = world.get::<Level>(unit).map_or(0, |level| level.0);
    let mut entries: Vec<_> = world
        .get_resource::<FollowerCatalog>()
        .into_iter()
        .flat_map(|catalog| &catalog.kits)
        .filter(|entry| entry.available(follower.profile.spec_id, level))
        .cloned()
        .map(super::utility::normalize_entry)
        .collect();
    if entries.is_empty() {
        return false;
    }
    entries.sort_by_key(|entry| match entry.slot {
        Slot::Interrupt => 0,
        Slot::Heal
            if entry
                .conditions
                .health_below
                .is_some_and(|threshold| threshold <= 0.2) =>
        {
            1
        }
        Slot::Defensive => 2,
        Slot::Dispel => 3,
        Slot::Control => 4,
        _ => 5,
    });
    let enemies = ai::engaged_enemies(world, follower, unit, party);
    if world.get::<CastState>(unit).is_none() && ai::prepare_pet(world, unit, follower) {
        return true;
    }
    let triage = (follower.profile.role == shared::group::GroupRole::Healer)
        .then(|| super::healer_triage::update(world, unit, follower, party, &entries));
    if triage.as_ref().is_some_and(|outcome| outcome.acted) {
        return true;
    }
    if super::utility::try_support(world, unit, follower, party, &enemies) {
        return true;
    }
    let candidates = super::utility_interrupt::ordered_casts(world, &enemies);
    let mut interrupt_attempted = false;
    for entry in entries
        .iter()
        .filter(|entry| entry.available(follower.profile.spec_id, level))
    {
        let policy_owned = triage.as_ref().is_some_and(|outcome| {
            outcome.owned.contains(&entry.spell_id)
                || super::healer_triage::manages(entry)
                || (!outcome.safe_to_dps && matches!(entry.slot, Slot::Attack | Slot::Aoe))
        });
        if policy_owned {
            continue;
        }
        let interrupt_entry = super::kit_interrupt::is_interrupt_entry(
            entry,
            world.resource::<crate::spell_info::SpellStore>(),
        );
        let pet_interrupt = follower.profile.spec_id == 266;
        if !interrupt_attempted && (interrupt_entry || pet_interrupt || entry.slot == Slot::Control)
        {
            interrupt_attempted = true;
            if super::utility_interrupt::try_party(world, follower.owner, party, &candidates) {
                return true;
            }
        }
        let entry_enemy = if entry.slot == Slot::Control {
            candidates.first().copied()
        } else {
            enemy
        };
        let Some(target) = select_target(world, unit, party, entry_enemy, entry) else {
            continue;
        };
        if conditions_met(world, unit, target, party, &enemies, entry)
            && attempt_cast(world, unit, target, follower, entry)
        {
            return true;
        }
    }
    if world.get::<CastState>(unit).is_some() {
        return true;
    }
    if let Some(enemy) = enemy {
        ai::order_pet_attack(world, unit, enemy);
    } else if enemies.is_empty() {
        ai::regroup(world, unit, follower.owner);
    } else {
        ai::hold_combat(world, unit);
    }
    true
}

fn health_fraction(world: &World, unit: Entity) -> Option<f32> {
    let health = world.get::<Health>(unit)?;
    (health.current > 0.0 && health.max > 0.0).then_some(health.current / health.max)
}

fn select_target(
    world: &World,
    unit: Entity,
    party: &[Entity],
    enemy: Option<Entity>,
    entry: &Entry,
) -> Option<Entity> {
    // The provider owns kicks at their existing priority position.
    if matches!(entry.slot, Slot::Passive | Slot::Utility | Slot::Interrupt) {
        return None;
    }
    match entry.conditions.target {
        Target::Enemy => enemy,
        Target::SelfUnit => Some(unit),
        Target::Party | Target::Tank => choose_patient(world, unit, party, entry),
    }
}

fn choose_patient(world: &World, unit: Entity, party: &[Entity], entry: &Entry) -> Option<Entity> {
    let eligible = party
        .iter()
        .copied()
        .filter(|target| patient_is_eligible(world, unit, *target, entry));
<<<<<<< HEAD
    let by_health = eligible
        .filter_map(|target| health_fraction(world, target).map(|fraction| (target, fraction)));
    by_health
        .min_by(|left, right| {
            let harm = |target| {
                if entry.slot == Slot::Dispel {
                    super::utility_interrupt::dispel_harm(
                        world,
                        unit,
                        target,
                        &entry.conditions.dispel_types,
                    )
                } else {
                    0
                }
            };
            harm(right.0)
                .cmp(&harm(left.0))
                .then_with(|| left.1.total_cmp(&right.1))
                .then_with(|| left.0.to_bits().cmp(&right.0.to_bits()))
=======
    let healer = world
        .get::<Follower>(unit)
        .is_some_and(|follower| follower.profile.role == shared::group::GroupRole::Healer);
    let ranked = eligible.filter_map(|target| {
        let fraction = health_fraction(world, target)?;
        let score = if healer {
            super::healer_triage::read_patient(world, target)?.score
        } else {
            1.0 - fraction
        };
        Some((target, score))
    });
    ranked
        .max_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| right.0.to_bits().cmp(&left.0.to_bits()))
>>>>>>> d907fad50
        })
        .map(|(target, _)| target)
}

pub(super) fn patient_is_eligible(
    world: &World,
    unit: Entity,
    target: Entity,
    entry: &Entry,
) -> bool {
    let same_map = world.get::<InMap>(target) == world.get::<InMap>(unit);
    let missing_requested_aura = entry
        .conditions
        .missing_aura
        .is_none_or(|aura| !crate::auras::has_aura(world, target, aura));
    let correct_role = entry.conditions.target != Target::Tank || is_tank(world, target);
    let can_remove_aura = entry.slot != Slot::Dispel
        || crate::auras::has_dispellable_aura(world, unit, target, &entry.conditions.dispel_types);
    let native_target_eligible = world
        .resource::<crate::spell_info::SpellStore>()
        .get(entry.spell_id)
        .is_some_and(|spell| {
            crate::spell_cast::check_target_aura_restrictions(world, unit, Some(target), spell)
                .is_ok()
        });
    let defensive_eligible = entry.slot != Slot::Defensive
        || (super::utility::projected_death(world, target)
            && !super::utility::active_defensive(world, target));
    same_map
        && missing_requested_aura
        && correct_role
        && can_remove_aura
        && native_target_eligible
        && defensive_eligible
}

pub(super) fn is_tank(world: &World, unit: Entity) -> bool {
    if world
        .get::<Follower>(unit)
        .is_some_and(|follower| follower.profile.role == shared::group::GroupRole::Tank)
    {
        return true;
    }
    let Some(spec) = world.get::<crate::class_progression::ActiveSpec>(unit) else {
        return false;
    };
    world
        .get_resource::<crate::spell_info::ClassSpells>()
        .is_some_and(|classes| {
            classes
                .classes
                .iter()
                .flat_map(|class| &class.specs)
                .any(|entry| entry.spec_id == spec.0 && entry.role == 0)
        })
}

pub(super) fn conditions_met(
    world: &mut World,
    unit: Entity,
    target: Entity,
    party: &[Entity],
    enemies: &[(Entity, Option<Entity>)],
    entry: &Entry,
) -> bool {
    let condition = &entry.conditions;
    let party_in_combat = party.iter().any(|member| {
        world.get::<InMap>(*member) == world.get::<InMap>(unit)
            && world
                .get::<shared::components::CombatStatus>(*member)
                .is_some_and(|status| status.0)
    });
    if super::utility::is_raid_buff(entry.spell_id) && party_in_combat {
        return false;
    }
    if entry.slot == Slot::Defensive
        && (!super::utility::projected_death(world, target)
            || super::utility::active_defensive(world, target))
    {
        return false;
    }
    if !health_and_auras_allow(world, unit, target, party, entry)
        || !combat_allows(world, unit, target, enemies, entry)
    {
        return false;
    }
    let enough_injured = party
        .iter()
        .filter(|member| {
            world.get::<InMap>(**member) == world.get::<InMap>(unit)
                && health_fraction(world, **member)
                    .is_some_and(|health| health < condition.health_below.unwrap_or(1.0))
        })
        .count()
        >= condition.min_injured;
    enough_injured
        && (condition.min_enemies == 0
            || super::kit_area::allows(world, unit, target, enemies, entry))
}

fn health_and_auras_allow(
    world: &World,
    unit: Entity,
    target: Entity,
    party: &[Entity],
    entry: &Entry,
) -> bool {
    let condition = &entry.conditions;
    if entry.slot == Slot::Dispel
        && !crate::auras::has_dispellable_aura(world, unit, target, &condition.dispel_types)
    {
        return false;
    }
    if condition.health_below.is_some_and(|threshold| {
        health_fraction(world, target).is_none_or(|health| health >= threshold)
    }) {
        return false;
    }
    let aura_target = if entry.slot == Slot::Mitigation {
        unit
    } else {
        target
    };
    if condition
        .missing_aura
        .is_some_and(|aura| maintenance_aura_present(world, unit, aura_target, entry.slot, aura))
    {
        return false;
    }
    if condition.exclusive_aura
        && condition.missing_aura.is_some_and(|aura| {
            party
                .iter()
                .any(|member| crate::auras::has_aura(world, *member, aura))
        })
    {
        return false;
    }
    true
}

fn maintenance_aura_present(
    world: &World,
    caster: Entity,
    target: Entity,
    slot: Slot,
    aura: u32,
) -> bool {
    let periodic_damage = world
        .resource::<crate::spell_info::SpellStore>()
        .get(aura)
        .is_some_and(|spell| {
            spell.effects.iter().any(|effect| {
                crate::auras::is_aura_effect(effect)
                    && effect.aura == crate::auras::PERIODIC_DAMAGE_AURA
            })
        });
    if slot != Slot::Attack || !periodic_damage {
        return crate::auras::has_aura(world, target, aura);
    }
    // SimC76f8118 sc_druid.cpp13833: target.get_dot("moonfire", &source).
    world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|active| active.spell_id == aura && active.caster == caster)
        })
}

pub(super) fn combat_allows(
    world: &World,
    unit: Entity,
    target: Entity,
    enemies: &[(Entity, Option<Entity>)],
    entry: &Entry,
) -> bool {
    let condition = &entry.conditions;
    if super::utility::is_raid_buff(entry.spell_id) && !enemies.is_empty() {
        return false;
    }
    if entry.slot == Slot::Taunt && !is_tank(world, unit) {
        return false;
    }
    if condition.combat && enemies.is_empty() {
        return false;
    }
    if condition.requires_cast && world.get::<CastState>(target).is_none() {
        return false;
    }
    // Do not create a hold→regen→expiry→hold cycle on the sole enemy.
    // Existing CC still expires natively; kicks/nonbreakable stuns are unchanged.
    if enemies.len() == 1 && is_damage_breakable_control(world, entry) {
        return false;
    }
    if entry.slot == Slot::Control
        && world
            .get::<crate::auras::ActiveAuras>(target)
            .is_some_and(|auras| auras.cannot_auto_attack())
    {
        return false;
    }
    let protected = if entry.slot == Slot::Mitigation {
        unit
    } else {
        target
    };
    if condition.attacked && !enemies.iter().any(|(_, victim)| *victim == Some(protected)) {
        return false;
    }
    if condition.lost_threat
        && world
            .get::<CombatTarget>(target)
            .and_then(|target| target.0)
            .is_none_or(|victim| victim == unit)
    {
        return false;
    }
    true
}

fn is_damage_breakable_control(world: &World, entry: &Entry) -> bool {
    entry.slot == Slot::Control
        && world
            .resource::<crate::spell_info::SpellStore>()
            .get(entry.spell_id)
            .and_then(|spell| spell.interrupts.as_ref())
            .is_some_and(|flags| {
                flags.aura_interrupt_flags[0] & crate::auras::ANY_DAMAGE_INTERRUPT_MASK != 0
            })
}

fn attempt_cast(
    world: &mut World,
    unit: Entity,
    target: Entity,
    follower: &Follower,
    entry: &Entry,
) -> bool {
    // An already admitted heal must finish; selecting it again is not a new
    // emergency. Re-starting it each tick would consume no mana and heal nobody.
    if world
        .get::<CastState>(unit)
        .is_some_and(|cast| cast.spell_id == entry.spell_id && cast.target == target.to_bits())
    {
        return true;
    }
    let urgent = entry.slot == Slot::Control
        || entry.slot == Slot::Defensive
        || (entry.slot == Slot::Heal
            && entry
                .conditions
                .health_below
                .is_some_and(|health| health <= PROJECT_URGENT_HEAL_THRESHOLD));
    if world.get::<CastState>(unit).is_some() && !urgent {
        return false;
    }
    if target != unit
        && entry.slot != Slot::Control
        && !ai::pursue(
            world,
            unit,
            target,
            entry.spell_id,
            follower.profile.melee && entry.conditions.target == Target::Enemy,
        )
    {
        return false;
    }
    if entry.conditions.target == Target::Enemy
        && entry.slot != Slot::Control
        && super::utility::try_movement(world, unit, target, follower, entry.spell_id)
    {
        return true;
    }
    let result = if urgent {
        crate::spell_cast::creature_cast_interrupting_self(
            world,
            unit,
            entry.spell_id,
            Some(target),
        )
    } else {
        crate::spell_cast::creature_cast(world, unit, entry.spell_id, Some(target), false)
    };
    match result {
        Ok(()) => {
            if entry.conditions.target == Target::Enemy {
                ai::order_pet_attack(world, unit, target);
            }
            true
        }
        Err(crate::spell_cast::CastFailure::OnGlobalCooldown) => true,
        Err(error) => {
            debug!(
                ?unit,
                spell = entry.spell_id,
                ?error,
                "Follower kit cast unavailable"
            );
            false
        }
    }
}
