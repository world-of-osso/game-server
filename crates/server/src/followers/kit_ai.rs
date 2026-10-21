//! Execute named project priorities without changing spell costs, timers or movement.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::{Health, Level};

use super::kit::{Entry, ResourceCondition, Slot, Target};
use super::{Follower, FollowerCatalog, ai};
use crate::maps::InMap;
use crate::networking::CombatTarget;

const PROJECT_URGENT_HEAL_THRESHOLD: f32 = 0.40;

pub(super) fn update(
    world: &mut World,
    unit: Entity,
    follower: &Follower,
    party: &[Entity],
    enemy: Option<Entity>,
) -> bool {
    let mut entries: Vec<_> = world
        .get_resource::<FollowerCatalog>()
        .into_iter()
        .flat_map(|catalog| &catalog.kits)
        .filter(|entry| entry.spec_id == follower.profile.spec_id)
        .cloned()
        .collect();
    if entries.is_empty() {
        return false;
    }
    super::tank_defense::sort_major_entries(world, unit, &mut entries);
    let level = world.get::<Level>(unit).map_or(0, |level| level.0);
    let enemies = ai::engaged_enemies(world, follower, unit, party);
    if world.get::<CastState>(unit).is_none() && ai::prepare_pet(world, unit, follower) {
        return true;
    }
    let candidates: Vec<_> = enemy.into_iter().collect();
    let mut interrupt_attempted = false;
    for entry in entries
        .iter()
        .filter(|entry| entry.available(follower.profile.spec_id, level))
    {
        let interrupt_entry = super::kit_interrupt::is_interrupt_entry(
            entry,
            world.resource::<crate::spell_info::SpellStore>(),
        );
        let pet_interrupt = follower.profile.spec_id == 266;
        if !interrupt_attempted && (interrupt_entry || pet_interrupt) {
            interrupt_attempted = true;
            if super::kit_interrupt::try_interrupt_candidates(
                world,
                unit,
                &follower.profile,
                &candidates,
            ) {
                return true;
            }
        }
        if matches!(entry.slot, Slot::Passive | Slot::Utility | Slot::Interrupt) {
            continue;
        }
        let spell_id = match super::kit_overrides::resolve_action(world, unit, entry.spell_id) {
            Ok(spell) => spell,
            Err(error) => {
                debug!(
                    ?unit,
                    base = entry.spell_id,
                    ?error,
                    "Follower action unavailable"
                );
                continue;
            }
        };
        let mut entry = entry.clone();
        entry.spell_id = spell_id;
        let Some(target) = select_target(world, unit, party, enemy, &entry) else {
            continue;
        };
        if conditions_met(world, unit, target, party, &enemies, &entry)
            && attempt_cast(world, unit, target, follower, &entry)
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
    let by_health = eligible
        .filter_map(|target| health_fraction(world, target).map(|fraction| (target, fraction)));
    by_health
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(target, _)| target)
}

fn patient_is_eligible(world: &World, unit: Entity, target: Entity, entry: &Entry) -> bool {
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
    same_map && missing_requested_aura && correct_role && can_remove_aura && native_target_eligible
}

fn is_tank(world: &World, unit: Entity) -> bool {
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
    if !action_history_allows(world, unit, entry)
        || !entry
            .conditions
            .resources
            .iter()
            .all(|rule| resource_allows(world, unit, rule))
        || !health_and_auras_allow(world, unit, target, party, entry)
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

fn resource_allows(world: &World, unit: Entity, rule: &ResourceCondition) -> bool {
    use crate::spell_cast::powers::{PowerTypeTable, power_value};
    let Some((current, maximum)) = power_value(world, unit, rule.power) else {
        return false;
    };
    let scale = world
        .resource::<PowerTypeTable>()
        .get(rule.power)
        .expect("rotation resource requires native PowerType metadata")
        .display_modifier;
    assert!(
        scale > 0,
        "rotation resource requires positive DisplayModifier"
    );
    let current = current / scale as f32;
    let deficit = maximum / scale as f32 - current;
    rule.min.is_none_or(|minimum| current >= minimum)
        && rule.below.is_none_or(|maximum| current < maximum)
        && rule.min_deficit.is_none_or(|minimum| deficit >= minimum)
}

fn action_history_allows(world: &World, unit: Entity, entry: &Entry) -> bool {
    world
        .get::<crate::spell_cast::SpellHistory>(unit)
        .and_then(|history| history.last_cast_in(&entry.conditions.no_repeat_group))
        != Some(entry.spell_id)
}

fn health_allows(world: &World, unit: Entity, target: Entity, entry: &Entry) -> bool {
    if entry.conditions.tank_defensive_sequence {
        return super::tank_defense::allows(world, unit, health_fraction(world, unit));
    }
    entry.conditions.health_below.is_none_or(|threshold| {
        health_fraction(world, target).is_some_and(|health| health < threshold)
    })
}

fn health_and_auras_allow(
    world: &World,
    unit: Entity,
    target: Entity,
    party: &[Entity],
    entry: &Entry,
) -> bool {
    let condition = &entry.conditions;
    if condition
        .required_aura
        .is_some_and(|aura| !crate::auras::has_aura(world, unit, aura))
    {
        return false;
    }
    if entry.slot == Slot::Dispel
        && !crate::auras::has_dispellable_aura(world, unit, target, &condition.dispel_types)
    {
        return false;
    }
    if !health_allows(world, unit, target, entry) {
        return false;
    }
    let aura_target = if entry.slot == Slot::Mitigation {
        unit
    } else {
        target
    };
    if !maintenance_allows(world, unit, aura_target, entry) {
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

fn maintenance_allows(world: &World, caster: Entity, target: Entity, entry: &Entry) -> bool {
    let Some(aura) = entry.conditions.missing_aura else {
        return true;
    };
    match maintenance_aura_present(world, caster, target, entry, aura) {
        Ok(present) => !present,
        Err(error) => {
            debug!(?caster, aura, ?error, "Follower aura forecast unavailable");
            false
        }
    }
}

fn refresh_window_seconds(
    world: &World,
    caster: Entity,
    target: Entity,
    entry: &Entry,
    aura: u32,
) -> Result<Option<f64>, crate::spell_cast::CastFailure> {
    use crate::spell_cast::CastFailure;
    match (
        entry.conditions.refresh_seconds,
        entry.conditions.refresh_fraction,
    ) {
        (None, None) => Ok(None),
        (Some(seconds), None) => Ok(Some(f64::from(seconds))),
        (None, Some(fraction)) => {
            let full = crate::auras::maximum_duration_ms(world, caster, target, aura).ok_or_else(
                || {
                    CastFailure::Unsupported(format!(
                        "aura {aura} has no native finite full duration"
                    ))
                },
            )?;
            Ok(Some(f64::from(full) / 1_000.0 * f64::from(fraction)))
        }
        (Some(_), Some(_)) => Err(CastFailure::Unsupported(
            "choose one aura refresh time unit".into(),
        )),
    }
}

fn maintenance_aura_present(
    world: &World,
    caster: Entity,
    target: Entity,
    entry: &Entry,
    aura: u32,
) -> Result<bool, crate::spell_cast::CastFailure> {
    let periodic_damage = world
        .resource::<crate::spell_info::SpellStore>()
        .get(aura)
        .is_some_and(|spell| {
            spell.effects.iter().any(|effect| {
                crate::auras::is_aura_effect(effect)
                    && effect.aura == crate::auras::PERIODIC_DAMAGE_AURA
            })
        });
    let owned = entry.slot == Slot::Attack && periodic_damage;
    let window = refresh_window_seconds(world, caster, target, entry, aura)?;
    let now = crate::auras::aura_now(world);
    // SimC76f8118 sc_druid.cpp13833: target.get_dot("moonfire", &source).
    Ok(world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .filter(|active| {
                    let owner_matches = !owned || active.caster == caster;
                    active.spell_id == aura && owner_matches
                })
                .any(|active| match window {
                    None => true,
                    Some(window) => active.expires_at.is_none_or(|expiry| expiry - now > window),
                })
        }))
}

fn combat_allows(
    world: &World,
    unit: Entity,
    target: Entity,
    enemies: &[(Entity, Option<Entity>)],
    entry: &Entry,
) -> bool {
    let condition = &entry.conditions;
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
