//! Bounded scripts derived from local Retail 12.1.0.69933 tooltip/effect rows.
use bevy::prelude::*;

use super::{reduce_cooldown, script_effect_value};
use crate::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use crate::spell_info::{SpellInfo, SpellStore};
use targets::Relation;

pub(super) const PRAYER_OF_HEALING: u32 = 596;
pub(super) const GARROTE: u32 = 703;
pub(super) const SERENITY: u32 = 2050;
pub(super) const FLASH_HEAL: u32 = 2061;
pub(super) const SCORCH: u32 = 2948;
pub(super) const SHRED: u32 = 5221;
pub(super) const THUNDER_CLAP: u32 = 6343;
pub(super) const MOONFIRE: u32 = 8921;
pub(super) const DEVASTATE: u32 = 20243;
const MOONFIRE_DAMAGE: u32 = 164812;
const THUNDER_CLAP_SLOW: u32 = 435203;
const SHRED_RANK: u32 = 231063;
const SHIELD_SLAM: u32 = 23922;
const SHIELD_SLAM_RANK: u32 = 231834;
const STEALTH: u32 = 1784;
const GARROTE_RANK: u32 = 196861;
const GARROTE_SILENCE: u32 = 1330;
const GARROTE_DAMAGE_REDUCTION: u32 = 256148;

/// 8921 explicitly names 164812s1/o2/d. Resolve the two base effects, not
/// unauthored optional debuffs (notably the child's effect7 snare).
pub(super) fn moonfire(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!("Moonfire trigger depth exhausted");
        return;
    }
    let child = world
        .resource::<SpellStore>()
        .get(MOONFIRE_DAMAGE)
        .expect("Moonfire requires catalog 164812")
        .clone();
    let mut plan: TargetPlan = child
        .effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.index <= 1)
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

/// 6343's tooltip explicitly names 435203s1/d. Apply per damage-hit enemy.
pub(super) fn thunder_clap(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, THUNDER_CLAP_SLOW, Some(target), depth + 1);
}

/// 2050s2 is the number of seconds removed when Flash Heal is cast.
pub(super) fn flash_heal(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if world.resource::<SpellStore>().get(SERENITY).is_some() {
        let seconds = script_effect_value(world, caster, SERENITY, 1);
        let seconds = super::holy_priest::holy_word_reduction(world, caster, seconds);
        reduce_cooldown(world, caster, SERENITY, seconds);
    }
    super::holy_priest::spend_surge(world, caster);
}

/// 2948: strictly below s2% health, before the hit deals damage.
pub(super) fn scorch_crit(world: &World, caster: Entity, target: Entity) -> Option<f32> {
    let health = world.get::<shared::components::Health>(target)?;
    let threshold = script_effect_value(world, caster, SCORCH, 1);
    (health.current < health.max * threshold / 100.0).then_some(100.0)
}

/// 5221's tooltip gates the bleeding bonus on 231063 and reads its s2.
pub(super) fn shred_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    if crate::auras::has_aura(world, caster, SHRED_RANK) && is_bleeding(world, target) {
        amount * (1.0 + script_effect_value(world, caster, SHRED_RANK, 1) / 100.0)
    } else {
        amount
    }
}

pub(super) fn is_bleeding(world: &World, target: Entity) -> bool {
    const BLEED_MECHANIC: u32 = 15;
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(target) else {
        return false;
    };
    let store = world.resource::<SpellStore>();
    auras.iter().any(|aura| {
        let spell = store
            .get(aura.spell_id)
            .expect("active aura requires catalog spell");
        spell
            .categories
            .as_ref()
            .is_some_and(|c| c.mechanic == BLEED_MECHANIC)
            || spell.effects.iter().any(|effect| {
                effect.mechanic == BLEED_MECHANIC
                    && aura
                        .effects
                        .iter()
                        .any(|applied| applied.index == effect.index)
            })
    })
}

/// 231834s1 supplies the chance; the current 20243 Dummy mirror is not
/// separately interpreted. A successful Devastate hit can reset a running timer.
pub(super) fn devastate(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if !crate::auras::has_aura(world, caster, SHIELD_SLAM_RANK) {
        return;
    }
    let chance = script_effect_value(world, caster, SHIELD_SLAM_RANK, 0);
    if rand::random::<f32>() * 100.0 >= chance {
        return;
    }
    let now = spell_cast::history_now(world);
    let remaining = world
        .get::<spell_cast::SpellHistory>(caster)
        .and_then(|history| history.spell_ready_at.get(&SHIELD_SLAM))
        .map_or(0.0, |ready| (*ready - now).max(0.0));
    reduce_cooldown(world, caster, SHIELD_SLAM, remaining);
}

/// 703/196861/1330/256148: regular Stealth Garrote grants the catalog
/// five-second silence and damage reduction. Improved Garrote windows remain
/// unimplemented; they are not inferred from the root's zero-valued marker.
pub(super) fn garrote(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if !targets::relation_matches(world, caster, target, Relation::Enemy)
        || !crate::auras::has_aura(world, caster, GARROTE_RANK)
        || !crate::auras::has_aura(world, caster, STEALTH)
    {
        return;
    }
    spell_cast::cast_triggered(world, caster, GARROTE_SILENCE, Some(target), depth + 1);
    let silenced = world
        .get::<crate::auras::ActiveAuras>(target)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == GARROTE_SILENCE && aura.caster == caster)
        });
    // 196861: only enemies actually silenced by Garrote receive256148.
    if silenced {
        spell_cast::cast_triggered(
            world,
            caster,
            GARROTE_DAMAGE_REDUCTION,
            Some(target),
            depth + 1,
        );
    }
}

fn select_prayer_allies(
    world: &mut World,
    caster: Entity,
    primary: Entity,
    radius: f32,
    additional: usize,
) -> Vec<Entity> {
    let center = targets::position(world, primary).expect("validated ally position");
    let mut units = targets::area_units(world, caster, center, radius, Relation::Ally, 0);
    units.retain(|&unit| {
        unit != primary
            && world
                .get::<shared::components::Health>(unit)
                .is_some_and(|health| health.current < health.max)
    });
    units.truncate(additional);
    units.insert(0, primary);
    units
}

/// 596: target plus s3-1 injured allies inside effect1's second radius.
/// Data does not specify an injury ordering; retain normal nearest-first
/// area selection, exclude full-health units and cap the additional allies.
pub(super) fn prayer_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Prayer of Healing validated ally");
    let heal = spell
        .effects
        .iter()
        .position(|effect| effect.index == 1)
        .expect("Prayer heal effect");
    let radius = targets::radius(&spell.effects[heal], 1);
    let additional = script_effect_value(world, caster, PRAYER_OF_HEALING, 2) as usize - 1;
    let units = select_prayer_allies(world, caster, primary, radius, additional);
    vec![
        PlannedEffect {
            index: 0,
            units: vec![primary],
            dest: None,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
        PlannedEffect {
            index: heal,
            units,
            dest: None,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
    ]
}
