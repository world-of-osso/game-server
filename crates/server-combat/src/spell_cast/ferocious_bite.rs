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
const SABER_JAWS: u32 = 421_432;
const BURN_EFFECT: u8 = 1;

fn read_aura_multiplier(world: &World, caster: Entity, spell: u32, index: u8) -> f32 {
    let amount = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == index))
        .map_or(0.0, |effect| effect.amount);
    1.0 + amount / 100.0
}

fn extra_energy_cap(world: &World, caster: Entity, source: &SpellInfo) -> f32 {
    let burn = source
        .effects
        .iter()
        .find(|effect| effect.index == BURN_EFFECT)
        .expect("native Ferocious Bite requires its Energy-burn effect");
    let maximum = effects::modded_base_points(world, caster, source, burn)
        * read_aura_multiplier(world, caster, INCARNATION, BURN_EFFECT);
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
    let mut spell = source.clone();
    // SimC76f8118 sc_druid.cpp4358–4401: Saber Jaws augments only the excess-Energy term.
    let excess = read_extra_fraction(world, caster, source);
    // sc_druid.cpp4391–4394 excludes Saber Jaws from free secondary Bites.
    let saber = if is_free_spender(source) {
        1.0
    } else {
        read_aura_multiplier(world, caster, SABER_JAWS, 0)
    };
    spell.cast_damage_multiplier = Some(1.0 + excess * saber);
    Cow::Owned(spell)
}

fn is_free_spender(spell: &SpellInfo) -> bool {
    spell
        .cast_policy_context
        .as_ref()
        .is_some_and(|context| context.free_spender)
}

fn read_extra_fraction(world: &World, caster: Entity, source: &SpellInfo) -> f32 {
    // SimC76f8118 sc_druid.cpp4374–4401: free secondary Bites retain
    // maximum effectiveness without reading or spending an effective pool.
    if is_free_spender(source) {
        return 1.0;
    }
    let maximum = extra_energy_cap(world, caster, source);
    let (energy, _) = powers::power_value(world, caster, PowerType::Energy)
        .expect("native Bite requires its Energy pool");
    energy.min(maximum) / maximum
}

/// Decode the extra-Energy factor retained by this actual native Bite launch.
/// SimC76f8118 sc_druid.cpp4374–4398 passes this fraction (not1+Saber
/// damage effectiveness) to Unseen attacks. Reading cannot spend or grant Energy.
pub fn read_extra_energy_modifier(world: &World, caster: Entity, spell: &SpellInfo) -> Option<f32> {
    if spell.id != FEROCIOUS_BITE {
        return None;
    }
    let effectiveness = spell.cast_damage_multiplier?;
    if is_free_spender(spell) {
        return Some(1.0);
    }
    let saber = read_aura_multiplier(world, caster, SABER_JAWS, 0);
    let extra = (effectiveness - 1.0) / saber;
    assert!(
        extra.is_finite() && (0.0..=1.000001).contains(&extra),
        "native Bite snapshot must retain a valid extra-Energy fraction"
    );
    Some(extra.min(1.0))
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
        if is_free_spender(spell) {
            return 0.0;
        }
        amount * read_aura_multiplier(world, caster, INCARNATION, BURN_EFFECT)
    } else {
        amount
    }
}
