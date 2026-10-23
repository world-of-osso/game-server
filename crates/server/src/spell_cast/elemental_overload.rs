//! TC a352b1fa spell_shaman.cpp2124–2230: native mastery, real SK modifier,
//!400ms callback and default TRIGGERED_NONE child cast; never a base-hit scalar.
use bevy::prelude::*;

use crate::auras::ActiveAuras;
use crate::spell_info::SpellInfo;

const MASTERY: u32 = 168534;
const STORMKEEPER: u32 = 191634;
const LIGHTNING_BOLT: u32 = 188196;
pub(super) const LIGHTNING_OVERLOAD: u32 = 45284;
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

/// The already matched launch snapshot proves SK actually modified this cast.
/// The live aura can disappear when its charge is consumed after this boundary.
pub(super) fn after_cast(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    if spell.id != LIGHTNING_BOLT {
        return;
    }
    let Some(chance) = mastery_amount(world, caster, 0) else {
        return;
    };
    let Some(target) = target else {
        return;
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
    super::queued_triggers::schedule_normal_cast(
        world,
        &ctx,
        target,
        LIGHTNING_OVERLOAD,
        OVERLOAD_DELAY_MS,
    );
}

/// TC reads the current native effect1 amount, including its authored modifiers.
/// Echo Chamber108/PointsIndex1 is already applied by ordinary aura calculation.
pub(super) fn scale_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    mastery_amount(world, caster, 1).map_or(amount, |percent| amount * percent / 100.0)
}
