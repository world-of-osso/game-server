//! Bounded paths derived from local Mainline 12.1.0.69933 rows.
use bevy::prelude::*;
use shared::components::{Health, PowerType};

use super::{reduce_cooldown, script_effect_value};
use crate::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use crate::spell_info::SpellInfo;

pub(super) const LIGHT_OF_DAWN: u32 = 85222;
pub(super) const STEADY_SHOT: u32 = 56641;
pub(super) const KILL_SHOT: u32 = 53351;
pub(super) const MANGLE: u32 = 33917;
pub(super) const VIVIFY: u32 = 116670;
pub(super) const RENEWING_MIST_CAST: u32 = 115151;
pub(super) const CHAIN_LIGHTNING: u32 = 188443;
pub(super) const HOWLING_BLAST: u32 = 49184;
pub(super) const SANCTIFY: u32 = 34861;
pub(super) const ERUPTION: u32 = 395160;
const STEADY_FOCUS: u32 = 77443;
const MANGLE_RANK: u32 = 231064;
const VIVIFY_RANK: u32 = 274586;
const RENEWING_MIST: u32 = 119611;
const VIVIFY_SPLASH: u32 = 425804;
const ELEMENTAL_RESOURCE: u32 = 343725;
const FROST_FEVER: u32 = 55095;
const ERUPTION_EXTENSION: u32 = 395153;
const EBON_MIGHT: u32 = 395152;
const EBON_SELF: u32 = 395296;

/// 85222s2: injured allies only, capped after ordinary area/radius selection.
/// Data does not define injury priority; retain native nearest-first ordering.
pub(super) fn light_of_dawn_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .expect("Dawn heal");
    let units = select_dawn_allies(world, caster, spell, explicit, index);
    vec![PlannedEffect {
        index,
        units,
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn select_dawn_allies(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    index: usize,
) -> Vec<Entity> {
    let mut units = targets::select_targets(
        world,
        caster,
        explicit,
        spell,
        &spell.effects[index],
        0,
        false,
    )
    .expect("Light of Dawn catalog selectors");
    units.retain(|&unit| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current < health.max)
    });
    units.truncate(script_effect_value(world, caster, LIGHT_OF_DAWN, 1) as usize);
    units
}

/// 56641 explicitly references 77443s1 Focus; no guessed resource ID/amount.
pub(super) fn steady_shot(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, STEADY_FOCUS, Some(caster), depth + 1);
}

/// 53351: strictly less than s2% health, checked before costs/cooldowns.
pub(super) fn kill_shot_allowed(world: &World, caster: Entity, target: Entity) -> bool {
    let threshold = script_effect_value(world, caster, KILL_SHOT, 1);
    world
        .get::<Health>(target)
        .is_some_and(|health| health.current < health.max * threshold / 100.0)
}

/// 33917/231064: s3 bleeding bonus requires the named passive.
pub(super) fn mangle_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    if crate::auras::has_aura(world, caster, MANGLE_RANK)
        && super::batch3::is_bleeding(world, target)
    {
        amount * (1.0 + script_effect_value(world, caster, MANGLE, 2) / 100.0)
    } else {
        amount
    }
}

/// 115151 explicitly names119611o1/d. Apply only its base periodic heal;
/// jumping on overheal and rank231606 procs remain separate contracts.
pub(super) fn renewing_mist(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: RENEWING_MIST,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// 116670/274586 explicitly names 425804 and "your Renewing Mist".
/// Cast once per marked ally, not per tick and not for another Monk's aura.
pub(super) fn vivify(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !crate::auras::has_aura(world, caster, VIVIFY_RANK) {
        return;
    }
    let child = world
        .resource::<crate::spell_info::SpellStore>()
        .get(VIVIFY_SPLASH)
        .expect("Vivify requires425804")
        .clone();
    let index = child
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .expect("Vivify splash heal");
    let units = select_vivify_allies(world, caster, &child, index);
    let mut plan = vec![PlannedEffect {
        index,
        units,
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    spell_cast::hit::remove_defended_targets(world, caster, &child, &mut plan);
    spell_cast::missiles::launch(world, caster, &child, plan, 1);
}

fn select_vivify_allies(
    world: &mut World,
    caster: Entity,
    child: &SpellInfo,
    index: usize,
) -> Vec<Entity> {
    // Keep425804's catalog area/radius, narrowed to the root's owned marks.
    let mut units =
        targets::select_targets(world, caster, None, child, &child.effects[index], 0, false)
            .expect("Vivify catalog selectors");
    units.retain(|&unit| {
        world
            .get::<crate::auras::ActiveAuras>(unit)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|aura| aura.spell_id == RENEWING_MIST && aura.caster == caster)
            })
    });
    units
}

/// 188443: 343725s5 Maelstrom for each successful target hit, only with passive.
pub(super) fn chain_lightning(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if crate::auras::has_aura(world, caster, ELEMENTAL_RESOURCE) {
        let amount = script_effect_value(world, caster, ELEMENTAL_RESOURCE, 4);
        spell_cast::powers::change_power(world, caster, PowerType::Maelstrom, amount);
    }
}

/// 49184 names 55095 on all hit targets. Only its base periodic disease effect
/// is applied; optional proc/trait effects are not inferred from base tooltip.
pub(super) fn howling_blast(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: FROST_FEVER,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

/// 34861s3: Prayer of Healing reduces an existing recharge by six seconds.
pub(super) fn prayer(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if world
        .resource::<crate::spell_info::SpellStore>()
        .get(SANCTIFY)
        .is_some()
    {
        let seconds = script_effect_value(world, caster, SANCTIFY, 2);
        reduce_cooldown(world, caster, SANCTIFY, f64::from(seconds));
    }
}

/// 395160 gates extension on395153, whose s1 is milliseconds. Extend once
/// per cast, preserving ownership and tick clocks; never create missing buffs.
pub(super) fn eruption(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !crate::auras::has_aura(world, caster, ERUPTION_EXTENSION) {
        return;
    }
    let milliseconds = script_effect_value(world, caster, ERUPTION_EXTENSION, 0)
        .max(0.0)
        .round() as u32;
    let units: Vec<Entity> = world
        .query_filtered::<Entity, With<crate::auras::ActiveAuras>>()
        .iter(world)
        .collect();
    for unit in units {
        crate::auras::extend_owned_duration(
            world,
            caster,
            unit,
            &[EBON_MIGHT, EBON_SELF],
            milliseconds,
        );
    }
}
