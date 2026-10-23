//! TC a352b1fa spell_shaman.cpp2124–2230: native mastery, real SK modifier,
//!400ms callback and default TRIGGERED_NONE child cast; never a base-hit scalar.
use bevy::prelude::*;

use crate::auras::ActiveAuras;
use crate::spell_info::SpellInfo;

const MASTERY: u32 = 168534;
const STORMKEEPER: u32 = 191634;
const LIGHTNING_BOLT: u32 = 188196;
const CHAIN_LIGHTNING: u32 = 188443;
pub(super) const LIGHTNING_OVERLOAD: u32 = 45284;
pub(super) const CHAIN_OVERLOAD: u32 = 45297;
const OVERLOAD_DELAY_MS: u32 = 400;

fn mastery_amount(world: &World, caster: Entity, index: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == MASTERY && aura.caster == caster)?
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .map(|effect| effect.amount)
}

fn stormkeeper_modified_cast(spell: &SpellInfo) -> bool {
    spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == STORMKEEPER)
}

/// TDB168534 phase2 is HIT: roll once per landed target, then wait400ms.
/// The retained launch snapshot proves SK matched even after charge consumption.
pub(super) fn after_hit(world: &mut World, caster: Entity, spell: &SpellInfo, target: Entity) {
    let overload = match spell.id {
        LIGHTNING_BOLT => LIGHTNING_OVERLOAD,
        CHAIN_LIGHTNING => CHAIN_OVERLOAD,
        _ => return,
    };
    let Some(chance) = mastery_amount(world, caster, 0) else {
        return;
    };
    let chance = if spell.id == CHAIN_LIGHTNING {
        chance / 3.0
    } else {
        chance
    };
    let guaranteed = stormkeeper_modified_cast(spell);
    if !guaranteed && crate::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let ctx = super::effects::EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::queued_triggers::schedule_normal_cast(world, &ctx, target, overload, OVERLOAD_DELAY_MS);
}

/// TC reads the current native effect1 amount, including its authored modifiers.
/// Echo Chamber108/PointsIndex1 is already applied by ordinary aura calculation.
pub(super) fn scale_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    mastery_amount(world, caster, 1).map_or(amount, |percent| amount * percent / 100.0)
}
