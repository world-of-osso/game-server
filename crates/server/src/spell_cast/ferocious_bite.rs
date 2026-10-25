//! TC spell_dru_ferocious_bite: post-payment Energy scales this cast's damage.
use std::borrow::Cow;

use bevy::prelude::*;
use shared::components::PowerType;

use super::{effects, powers};
use crate::{
    auras::ActiveAuras,
    spell_info::{SpellInfo, SpellStore},
};

const FEROCIOUS_BITE: u32 = 22_568;
const INCARNATION: u32 = 102_543;
const BURN_EFFECT: u8 = 1;

fn incarnation_multiplier(world: &World, caster: Entity) -> f32 {
    let amount = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == INCARNATION))
        .and_then(|aura| {
            aura.effects
                .iter()
                .find(|effect| effect.index == BURN_EFFECT)
        })
        .map_or(0.0, |effect| effect.amount);
    // Native70448 effect1 is a trigger, so no such aura effect exists there.
    1.0 + amount / 100.0
}

fn extra_energy_cap(world: &World, caster: Entity, source: &SpellInfo) -> f32 {
    let burn = source
        .effects
        .iter()
        .find(|effect| effect.index == BURN_EFFECT)
        .expect("native Ferocious Bite requires its Energy-burn effect");
    let maximum = effects::modded_base_points(world, caster, source, burn)
        * incarnation_multiplier(world, caster);
    assert!(
        maximum > 0.0,
        "native Ferocious Bite requires a positive extra-Energy cap"
    );
    maximum
}

/// Actual modified Energy cost plus native extra cap, independent of affordability.
/// Reads the same numeric cost producer as spell_costs; never spends or fills a pool.
pub fn read_maximum_energy(world: &World, caster: Entity) -> Result<f32, super::CastFailure> {
    let spell = world
        .resource::<SpellStore>()
        .get(FEROCIOUS_BITE)
        .ok_or(super::CastFailure::UnknownSpell)?;
    let energy = super::applicable_costs(world, caster, spell)?
        .into_iter()
        .filter(|(pool, _, _)| *pool == super::CostPool::Power(PowerType::Energy))
        .map(|(_, amount, _)| amount)
        .sum::<f32>();
    Ok(energy + extra_energy_cap(world, caster, spell))
}

/// Native22568 is immediate: capture after ordinary payment, before any hit effect.
/// Keep the real coefficients and use the existing cast-only effectiveness field.
pub(super) fn snapshot<'s>(
    world: &World,
    caster: Entity,
    source: &'s SpellInfo,
) -> Cow<'s, SpellInfo> {
    if source.id != FEROCIOUS_BITE {
        return Cow::Borrowed(source);
    }
    let maximum = extra_energy_cap(world, caster, source);
    let energy =
        powers::power_value(world, caster, PowerType::Energy).map_or(0.0, |(current, _)| current);
    let mut spell = source.clone();
    spell.cast_damage_multiplier = Some(1.0 + energy.min(maximum) / maximum);
    Cow::Owned(spell)
}

/// EffectPowerBurn clamps this cap to actual remaining power; no power is granted.
pub(super) fn burn_limit(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect_index: u8,
    amount: f32,
) -> f32 {
    if spell.id == FEROCIOUS_BITE && effect_index == BURN_EFFECT {
        amount * incarnation_multiplier(world, caster)
    } else {
        amount
    }
}
