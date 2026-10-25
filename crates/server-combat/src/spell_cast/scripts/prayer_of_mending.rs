//! Prayer of Mending; TC a352b1fa spell_priest.cpp3636–3806.
//! Stack values and OriginalCaster are real per-cast arguments, including flight.
use crate::random as rand;
use bevy::prelude::*;
use rand::seq::SliceRandom;
use shared::components::{Health, Player};

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_cast::effects::{self, EffectCtx, EffectHandler};
use crate::spell_cast::{TriggeredSpellOverrides, cast_triggered, cast_triggered_with_overrides};
use crate::spell_info::{SpellEffect, SpellInfo};

pub(crate) const PRAYER: u32 = 33_076;
pub(crate) const WARD: u32 = 41_635;
const HEAL: u32 = 33_110;
const JUMP: u32 = 155_793;
const PROC_DEPTH: u8 = 1;

pub(super) fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    match (spell, index) {
        (PRAYER, 0) => Some(cast_initial_ward),
        (JUMP, 0) => Some(land_jump),
        _ => None,
    }
}

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        WARD,
        AuraScript {
            check_proc: Some(took_damage),
            on_damage_proc: Some(heal_and_jump),
            ..Default::default()
        },
    );
}

fn took_damage(_world: &World, _aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.damage > 0.0
}

fn cast_initial_ward(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let jumps = effects::modded_base_points(world, ctx.caster, ctx.spell, effect) as u8;
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == WARD && aura.caster == ctx.caster)
    });
    let stacks = if owned {
        jumps
    } else {
        jumps.saturating_add(1)
    };
    cast_ward(world, ctx.caster, target, stacks, ctx.depth + 1);
}

fn cast_ward(world: &mut World, priest: Entity, target: Entity, stacks: u8, depth: u8) {
    assert!(
        stacks > 0,
        "Prayer of Mending requires a positive native stack amount"
    );
    cast_triggered_with_overrides(
        world,
        priest,
        WARD,
        Some(target),
        depth,
        TriggeredSpellOverrides {
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    );
}

fn heal_and_jump(world: &mut World, holder: Entity, aura: &AuraInstance, _event: &AuraEvent) {
    if world.get_entity(aura.caster).is_err() {
        return;
    }
    cast_triggered(world, aura.caster, HEAL, Some(holder), PROC_DEPTH);
    if aura.stacks > 1 {
        cast_triggered_with_overrides(
            world,
            holder,
            JUMP,
            Some(holder),
            PROC_DEPTH,
            TriggeredSpellOverrides {
                effect0_amount: Some(f32::from(aura.stacks - 1)),
                original_caster: Some(aura.caster),
                ..Default::default()
            },
        );
    }
    // TC launches the jump before removing its current holder's aura.
    crate::auras::remove_instance(world, holder, aura.instance_id, RemoveReason::Consumed);
}

fn land_jump(world: &mut World, ctx: &EffectCtx, effect: &SpellEffect, target: Entity) {
    let Some(priest) = ctx.spell.cast_original_caster else {
        warn!("Prayer of Mending jump missing its original caster");
        return;
    };
    // GetOriginalCaster can disappear during the native40yd/s flight.
    if world.get_entity(priest).is_err() {
        return;
    }
    let stacks = effects::modded_base_points(world, ctx.caster, ctx.spell, effect) as u8;
    cast_ward(world, priest, target, stacks, ctx.depth + 1);
}

pub(super) fn area_filter(spell: u32, index: u8) -> Option<super::AreaTargetFilter> {
    ((spell, index) == (JUMP, 0)).then_some(select_random_jump_target)
}

/// Spell.cpp9516–9581: injury precedes player priority; full-health units stay
/// eligible. The native EXCLUDE_CASTER selector already removes the holder.
fn select_random_jump_target(
    world: &World,
    _spell: &SpellInfo,
    _explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    if units.len() <= 1 {
        return;
    }
    let best = units
        .iter()
        .map(|unit| read_target_priority(world, *unit))
        .min()
        .expect("more than one candidate");
    units.retain(|unit| read_target_priority(world, *unit) == best);
    units.shuffle(&mut rand::thread_rng());
    units.truncate(1);
}

fn read_target_priority(world: &World, unit: Entity) -> (bool, bool) {
    let full = world
        .get::<Health>(unit)
        .is_none_or(|health| health.current >= health.max);
    // NPC static-flag4 treat-as-raid priority has no producer in this runtime;
    // that special NPC classification remains an explicitly unqualified case.
    (full, world.get::<Player>(unit).is_none())
}
