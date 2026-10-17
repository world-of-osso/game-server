//! Bounded Mainline paths: local 12.1.0.69933 rows and TDB1210 bindings.
use bevy::prelude::*;
use shared::components::{Health, Npc, PowerType, UnitSummonedBy};

use super::script_effect_value;
use crate::auras::ActiveAuras;
use crate::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const REAP: u32 = 1226019;
pub(super) const PROVOKE: u32 = 115546;
pub(super) const DEMONBOLT: u32 = 264178;
pub(super) const CRASH_LIGHTNING: u32 = 187874;
pub(super) const BREATH_OF_FIRE: u32 = 115181;
pub(super) const FAN_OF_KNIVES: u32 = 51723;
pub(super) const SANCTIFY: u32 = 34861;
pub(super) const INCINERATE: u32 = 29722;
pub(super) const UNSTABLE_AFFLICTION: u32 = 30108;
pub(super) const ARCANE_BARRAGE: u32 = 44425;
const REAP_DAMAGE: u32 = 1225827;
pub(super) const PROVOKE_SINGLE: u32 = 116189;
pub(super) const PROVOKE_AREA: u32 = 118635;
const BLACK_OX_STATUE: u32 = 61146;
const HASTY_PROVOCATION: u32 = 328670;
const DEMONBOLT_ENERGIZE: u32 = 280127;
const CRASH_LIGHTNING_BUFF: u32 = 187878;
const KEG_SMASH: u32 = 121253;
const BREATH_BURN: u32 = 123725;
const INCINERATE_ENERGIZE: u32 = 244670;
const AFFLICTION_PERIODIC: u32 = 233490;
const BARRAGE_MANA_REFUND: u32 = 321526;
const HUNDREDTHS_PERCENT_DENOMINATOR: f32 = 10000.0;

/// Root names1225827. Its87/134 line is aimed at the hit unit's destination,
/// extending to the authored radius, rather than a single-target substitution.
pub(super) fn reap(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!("Reap trigger depth exhausted");
        return;
    }
    let mut child = world
        .resource::<SpellStore>()
        .get(REAP_DAMAGE)
        .expect("Reap requires1225827")
        .clone();
    child.cast_destination = Some(
        targets::position(world, target)
            .expect("Reap hit unit requires position")
            .to_array(),
    );
    let plan = spell_cast::plan_targets(world, caster, &child, Some(target), false)
        .expect("Reap requires catalog line selectors");
    spell_cast::missiles::launch(world, caster, &child, plan, depth + 1);
}

fn is_statue(world: &World, target: Entity) -> bool {
    world
        .get::<Npc>(target)
        .is_some_and(|npc| npc.template_id == BLACK_OX_STATUE)
}
pub(super) fn check_provoke(
    world: &World,
    caster: Entity,
    target: Entity,
) -> Result<(), spell_cast::CastFailure> {
    let allowed = if is_statue(world, target) {
        world
            .get::<UnitSummonedBy>(target)
            .is_some_and(|owner| owner.0 == caster.to_bits())
    } else {
        targets::relation_matches(world, caster, target, targets::Relation::Enemy)
    };
    if allowed {
        Ok(())
    } else {
        Err(spell_cast::CastFailure::InvalidTarget)
    }
}
pub(super) fn provoke(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let child = if is_statue(world, target) {
        PROVOKE_AREA
    } else {
        PROVOKE_SINGLE
    };
    spell_cast::cast_triggered(world, caster, child, Some(target), depth + 1);
}

///Provoke's optional speed requires328670. Every selected child still uses
///actual114 taunt,11 lifetime and catalog area.
pub(super) fn provoke_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let hasty = crate::auras::has_aura(world, caster, HASTY_PROVOCATION);
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.index != 2 || hasty)
        .map(|(index, effect)| {
            let units = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
                .expect("Provoke child catalog selectors");
            PlannedEffect {
                index,
                units,
                dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
                chain_multipliers: Vec::new(),
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            }
        })
        .collect()
}

///264178 names two whole shards;280127 carries20 raw units. TDB binds
///spell_warl_demonbolt's AfterCast, not each weapon/proc child or enemy hit.
pub(super) fn demonbolt(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    spell_cast::cast_triggered(world, caster, DEMONBOLT_ENERGIZE, Some(caster), 1);
}
///Current Mainline187874 unconditionally names187878d for its enhancement.
///The older TDB script's two-target condition is not in this Retail tooltip.
pub(super) fn crash_lightning(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    spell_cast::cast_triggered(world, caster, CRASH_LIGHTNING_BUFF, Some(caster), 1);
}
///Only this Monk's Keg Smash enables the named burn/reduced damage child.
pub(super) fn breath_of_fire(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|a| a.spell_id == KEG_SMASH && a.caster == caster)
    }) {
        spell_cast::cast_triggered(world, caster, BREATH_BURN, Some(target), depth + 1);
    }
}

pub(super) fn first_damage_target(
    world: &World,
    caster: Entity,
    spell_id: u32,
    target: Entity,
) -> bool {
    world
        .resource::<spell_cast::ResolvingHits>()
        .0
        .last()
        .is_some_and(|frame| {
            frame.caster == caster
                && frame.spell_id == spell_id
                && frame
                    .damage
                    .first()
                    .is_some_and(|(unit, _)| *unit == target)
        })
}
///51723s2 is one combo point for the cast, not one for every knife hit.
pub(super) fn fan_of_knives(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if first_damage_target(world, caster, FAN_OF_KNIVES, target) {
        let amount = script_effect_value(world, caster, FAN_OF_KNIVES, 1);
        spell_cast::powers::change_power(world, caster, PowerType::ComboPoints, amount);
    }
}
///29722 explicitly names244670s1 fragments. Award the base once for the
///root damage, not each splash unit. Crit/talent additions remain uncertified.
pub(super) fn incinerate(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if first_damage_target(world, caster, INCINERATE, target) {
        spell_cast::cast_triggered(world, caster, INCINERATE_ENERGIZE, Some(caster), depth + 1);
    }
}
///30108 names233490o1/d; dispel punishment, multi-application and vulnerability
///are separate unresolved root contracts, not additional guessed effects.
pub(super) fn unstable_affliction(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, AFFLICTION_PERIODIC, Some(target), depth + 1);
}
///34861s2 caps injured allies in the actual requested ground area.
pub(super) fn sanctify_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let index = spell
        .effects
        .iter()
        .position(|e| e.index == 0)
        .expect("Sanctify heal effect");
    let effect = &spell.effects[index];
    let mut units = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
        .expect("Sanctify area selectors");
    units.retain(|&unit| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current < health.max)
    });
    units.truncate(script_effect_value(world, caster, SANCTIFY, 1).max(0.0) as usize);
    vec![PlannedEffect {
        index,
        units,
        dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}
///44425 consumes charges after launch.321526 refunds its authored hundredths
///of a percent of maximum mana per charge;231564 secondary hits remain open.
pub(super) fn arcane_barrage(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let (current, _) = spell_cast::powers::power_value(world, caster, PowerType::ArcaneCharges)
        .expect("Mage requires Arcane Charges");
    if crate::auras::has_aura(world, caster, BARRAGE_MANA_REFUND) {
        let (_, max_mana) = spell_cast::powers::power_value(world, caster, PowerType::Mana)
            .expect("Mage requires mana");
        let hundredths_percent = script_effect_value(world, caster, BARRAGE_MANA_REFUND, 0);
        let refund = current * max_mana * hundredths_percent / HUNDREDTHS_PERCENT_DENOMINATOR;
        spell_cast::powers::change_power(world, caster, PowerType::Mana, refund);
    }
    spell_cast::powers::change_power(world, caster, PowerType::ArcaneCharges, -current);
}
