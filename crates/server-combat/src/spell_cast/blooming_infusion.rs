//! Mainline429433: independent five-cast counters, with authored429474/429438 modifiers.
use bevy::prelude::*;

use crate::auras::{ActiveAuras, AuraInstance, RemoveReason};
use crate::spell_info::{SpellInfo, SpellStore};

pub(crate) const BLOOMING_INFUSION: u32 = 429_433;
const OFFENSIVE_INFUSION: u32 = 429_474;
const HEALING_INFUSION: u32 = 429_438;
const REGROWTH: u32 = 8_936;
const STARSURGE: u32 = 78_674;
const STARFALL: u32 = 191_034;
const BALANCE_PASSIVE: u32 = 137_013;

#[derive(Component, Default)]
struct InfusionCasts {
    regrowth: u32,
    astral: u32,
}

pub(crate) fn start_cast_count(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit && world.get::<InfusionCasts>(unit).is_none() {
        world.entity_mut(unit).insert(InfusionCasts::default());
    }
}

pub(crate) fn stop_cast_count(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    if aura.caster == unit {
        world.entity_mut(unit).remove::<InfusionCasts>();
    }
}

fn generated_buff(world: &World, caster: Entity, spell_id: u32) -> Option<u32> {
    match spell_id {
        REGROWTH => Some(OFFENSIVE_INFUSION),
        STARSURGE => Some(HEALING_INFUSION),
        STARFALL if crate::auras::has_aura(world, caster, BALANCE_PASSIVE) => {
            Some(HEALING_INFUSION)
        }
        _ => None,
    }
}

fn native_threshold(world: &World, caster: Entity) -> Option<u32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == BLOOMING_INFUSION && aura.caster == caster)?;
    let amount = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
    let threshold = amount as u32;
    assert!(
        threshold > 0,
        "Blooming Infusion requires a positive authored cast count"
    );
    Some(threshold)
}

pub(super) fn check_children(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Result<(), super::CastFailure> {
    if native_threshold(world, caster).is_none() {
        return Ok(());
    }
    let Some(child) = generated_buff(world, caster, spell.id) else {
        return Ok(());
    };
    if world.resource::<SpellStore>().get(child).is_none() {
        return Err(super::CastFailure::Unsupported(format!(
            "Blooming Infusion missing native child {child}"
        )));
    }
    Ok(())
}

fn consume_healing_infusion(world: &mut World, caster: Entity, spell: &SpellInfo) {
    crate::auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == HEALING_INFUSION
            && aura
                .effects
                .iter()
                .any(|effect| crate::auras::spell_mods::class_mask_affects(aura, effect, spell))
    });
}

/// Called only for completed paid casts, not triggered copies or periodic ticks.
pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    consume_healing_infusion(world, caster, spell);
    let Some(threshold) = native_threshold(world, caster) else {
        return;
    };
    let Some(child) = generated_buff(world, caster, spell.id) else {
        return;
    };
    let mut casts = world
        .get_mut::<InfusionCasts>(caster)
        .expect("own Blooming Infusion cast state");
    let counter = if child == OFFENSIVE_INFUSION {
        &mut casts.regrowth
    } else {
        &mut casts.astral
    };
    *counter += 1;
    if *counter < threshold {
        return;
    }
    *counter = 0;
    super::cast_triggered(world, caster, child, Some(caster), 1);
}
