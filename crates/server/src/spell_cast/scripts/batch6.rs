//! Bounded paths derived from local Mainline 12.1.0.69933 DB2 rows.
use bevy::prelude::*;
use shared::components::Health;

use super::{reduce_cooldown, script_effect_value};
use crate::auras::ActiveAuras;
use crate::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const SUNFIRE: u32 = 93402;
pub(super) const TEMPLARS_VERDICT: u32 = 85256;
pub(super) const ICE_LANCE: u32 = 30455;
pub(super) const STARFIRE: u32 = 194153;
pub(super) const KEG_SMASH: u32 = 121253;
pub(super) const BUTCHERY: u32 = 212436;
pub(super) const WILD_GROWTH: u32 = 48438;
pub(super) const SHADOW_WORD_DEATH: u32 = 32379;
pub(super) const STARFALL: u32 = 191034;
const SUNFIRE_DAMAGE: u32 = 164815;
const IMPROVED_SUNFIRE: u32 = 231050;
const VERDICT_DAMAGE: u32 = 224266;
const ICE_LANCE_DAMAGE: u32 = 228598;
const WILDFIRE_BOMB: u32 = 259495;
const FRENZY_STRIKES: u32 = 294029;
const AETHERIAL_KINDLING: u32 = 327541;
const MOONFIRE_DAMAGE: u32 = 164812;

fn launch_selected_child(
    world: &mut World,
    caster: Entity,
    target: Entity,
    child_id: u32,
    indices: &[u8],
    depth: u8,
) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!(child_id, "Class script trigger depth exhausted");
        return;
    }
    let child = world
        .resource::<SpellStore>()
        .get(child_id)
        .expect("class script requires its named catalog child")
        .clone();
    let mut plan: TargetPlan = child
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| indices.contains(&effect.index))
        .map(|(index, _)| PlannedEffect {
            index,
            units: vec![target],
            dest: None,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        })
        .collect();
    spell_cast::hit::remove_defended_targets(world, caster, &child, &mut plan);
    spell_cast::missiles::launch(world, caster, &child, plan, depth + 1);
}

/// Root names164815s1/o2/d;231050 explicitly spreads only the periodic effect.
pub(super) fn sunfire(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    launch_selected_child(world, caster, target, SUNFIRE_DAMAGE, &[0, 1], depth);
    if !crate::auras::has_aura(world, caster, IMPROVED_SUNFIRE) {
        return;
    }
    let child = world
        .resource::<SpellStore>()
        .get(SUNFIRE_DAMAGE)
        .unwrap()
        .clone();
    let periodic = child
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .unwrap();
    let units = targets::select_targets(world, caster, Some(target), &child, periodic, 0, false)
        .expect("Sunfire periodic catalog area selectors");
    for unit in units.into_iter().filter(|&unit| unit != target) {
        launch_selected_child(world, caster, unit, SUNFIRE_DAMAGE, &[1], depth);
    }
}

///85256 names224266s1, not that child's unknown Dummy markers.
pub(super) fn verdict(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    launch_selected_child(world, caster, target, VERDICT_DAMAGE, &[0], depth);
}

///30455 names228598s1. Freezing/shatter remains a separate contract.
pub(super) fn ice_lance(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, ICE_LANCE_DAMAGE, Some(target), depth + 1);
}

/// Keep native units, destinations and non-damage effects. Per-target multipliers
/// come from the root tooltip, not a guessed AoE replacement.
fn plan_primary_damage(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
    damage_index: u8,
    multipliers: (f32, f32),
) -> TargetPlan {
    spell
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| spell_cast::selects_targets(spell, effect))
        .map(|(index, effect)| {
            let units = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
                .expect("class spell catalog selectors");
            let chain_multipliers = if effect.index == damage_index {
                primary_damage_multipliers(&units, explicit, multipliers)
            } else {
                Vec::new()
            };
            PlannedEffect {
                index,
                units,
                dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
                chain_multipliers,
                blocked_units: Vec::new(),
                prevent_hit_script: false,
            }
        })
        .collect()
}

fn primary_damage_multipliers(
    units: &[Entity],
    explicit: Option<Entity>,
    (primary, secondary): (f32, f32),
) -> Vec<(Entity, f32)> {
    units
        .iter()
        .map(|&unit| {
            let multiplier = if Some(unit) == explicit {
                primary
            } else {
                secondary
            };
            (unit, multiplier)
        })
        .collect()
}

///194153s3 supplies the70% secondary coefficient. Reduced damage beyond8 and
///429523's optional formula are not certified by the untalented base path.
pub(super) fn starfire_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let secondary = script_effect_value(world, caster, STARFIRE, 2) / 100.0;
    plan_primary_damage(world, caster, spell, explicit, 0, (1.0, secondary))
}

///121253s8 is the50% primary bonus; effect1 is its area damage.
pub(super) fn keg_smash_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = 1.0 + script_effect_value(world, caster, KEG_SMASH, 7) / 100.0;
    plan_primary_damage(world, caster, spell, explicit, 1, (primary, 1.0))
}

///294029 explicitly names212436s2 milliseconds per hit, capped at s3 targets.
/// The damage-effect ledger excludes defended/immune targets but includes
/// fully absorbed hits, which still count as successful targets.
pub(super) fn butchery(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if !crate::auras::has_aura(world, caster, FRENZY_STRIKES) {
        return;
    }
    let cap = script_effect_value(world, caster, BUTCHERY, 2) as usize;
    let position = world
        .resource::<spell_cast::ResolvingHits>()
        .0
        .last()
        .filter(|frame| frame.caster == caster && frame.spell_id == BUTCHERY)
        .and_then(|frame| frame.damage.iter().position(|(unit, _)| *unit == target));
    if position.is_some_and(|position| position < cap) {
        let seconds = f64::from(script_effect_value(world, caster, BUTCHERY, 1)) / 1000.0;
        reduce_cooldown(world, caster, WILDFIRE_BOMB, seconds);
    }
}

///48438s2: at most five injured allies in the native target-centered area.
/// Declining tick coefficients and Tree of Life's additional targets stay open.
pub(super) fn wild_growth_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == 0)
        .unwrap();
    let effect = &spell.effects[index];
    let mut units = targets::select_targets(world, caster, explicit, spell, effect, 0, false)
        .expect("Wild Growth catalog selectors");
    units.retain(|&unit| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current < health.max)
    });
    units.truncate(script_effect_value(world, caster, WILD_GROWTH, 1) as usize);
    vec![PlannedEffect {
        index,
        units,
        dest: targets::effect_dest(world, caster, explicit, effect, spell.cast_destination),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

///32379s6: surviving target causes5% caster-max-health backlash, after damage.
/// Use ordinary damage/death/log/absorb handling, not direct Health mutation.
pub(super) fn shadow_word_death(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if !targets::is_alive(world, target) {
        return;
    }
    let health = world
        .get::<Health>(caster)
        .expect("Shadow Word: Death caster health");
    let amount = health.max * script_effect_value(world, caster, SHADOW_WORD_DEATH, 5) / 100.0;
    let spell = world
        .resource::<SpellStore>()
        .get(SHADOW_WORD_DEATH)
        .unwrap()
        .clone();
    let ctx = spell_cast::effects::EffectCtx {
        caster,
        spell: &spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::effects::deal_damage(world, &ctx, caster, amount, false, false);
}

///191034/327541: extend owned DoTs by s1 seconds, at most s2 seconds remaining.
pub(super) fn starfall(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if !crate::auras::has_aura(world, caster, AETHERIAL_KINDLING) {
        return;
    }
    let milliseconds = (script_effect_value(world, caster, AETHERIAL_KINDLING, 0) * 1000.0) as u32;
    let maximum_ms = (script_effect_value(world, caster, AETHERIAL_KINDLING, 1) * 1000.0) as u32;
    let units: Vec<Entity> = world
        .query_filtered::<Entity, With<ActiveAuras>>()
        .iter(world)
        .collect();
    for unit in units {
        crate::auras::extend_owned_duration_capped(
            world,
            caster,
            unit,
            &[MOONFIRE_DAMAGE, SUNFIRE_DAMAGE],
            milliseconds,
            maximum_ms,
        );
    }
}
