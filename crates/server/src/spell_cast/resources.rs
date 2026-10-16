//! Resource-dependent values frozen at successful launch, before costs are paid.
//! Native SpellPower.OptionalCost is additional raw spend, capped by availability.
//! DurationPerResource is independent of the amount remaining at missile impact.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{CostPool, powers};
use crate::spell_info::{SpellDuration, SpellInfo};

const ENVENOM: u32 = 32645;
const BLACK_POWDER: u32 = 319175;
const DISPATCH: u32 = 2098;
const EVISCERATE: u32 = 196819;
const ARCANE_BARRAGE: u32 = 44425;
const ARCANE_BLAST: u32 = 30451;
const ARCANE_CHARGE: u32 = 36032;

/// CalcPowerCost's optional branch: signed, power/family-masked caster total.
pub(super) fn additional_optional_cost(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    power_type: i32,
) -> f32 {
    use crate::auras::{ActiveAuras, handlers, spell_mods};
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return 0.0;
    };
    auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(move |effect| {
                    effect.aura_type == handlers::MOD_ADDITIONAL_POWER_COST
                        && effect.misc_values[0] == power_type
                        && spell_mods::class_mask_affects(aura, effect, spell)
                })
                .map(move |effect| effect.amount.trunc() * aura.stack_multiplier())
        })
        .sum()
}

pub(super) fn spent(costs: &[(CostPool, f32)], power: PowerType) -> f32 {
    costs
        .iter()
        .filter_map(|(pool, amount)| match pool {
            CostPool::Power(kind) if *kind == power => Some(*amount),
            _ => None,
        })
        .sum()
}

fn scale_effect(spell: &mut SpellInfo, index: u8, multiplier: f32) {
    let effect = spell
        .effects
        .iter_mut()
        .find(|effect| effect.index == index)
        .expect("resource formula requires catalog effect");
    effect.base_points *= multiplier;
    effect.bonus_coefficient *= multiplier;
    effect.bonus_coefficient_from_ap *= multiplier;
}

fn snapshot_duration(duration: &mut SpellDuration, combo: f32) {
    if duration.per_resource_ms == 0 {
        return;
    }
    duration.duration_ms += (duration.per_resource_ms as f32 * combo) as i32;
    if duration.max_duration_ms > 0 {
        duration.duration_ms = duration.duration_ms.min(duration.max_duration_ms);
    }
    duration.per_resource_ms = 0;
}

fn barrage_multiplier(world: &World, caster: Entity) -> f32 {
    let (charges, _) = powers::power_value(world, caster, PowerType::ArcaneCharges)
        .expect("Arcane Barrage requires charges");
    let bonus = super::scripts::script_effect_value(world, caster, ARCANE_CHARGE, 1);
    1.0 + charges * bonus / 100.0
}

pub(super) fn snapshot(
    world: &World,
    caster: Entity,
    source: &SpellInfo,
    costs: &[(CostPool, f32)],
) -> SpellInfo {
    let mut spell = source.clone();
    let combo = spent(costs, PowerType::ComboPoints);
    if combo > 0.0
        && let Some(misc) = &mut spell.misc
    {
        for duration in [&mut misc.duration, &mut misc.pvp_duration]
            .into_iter()
            .flatten()
        {
            snapshot_duration(duration, combo);
        }
    }
    let multiplier = match spell.id {
        // Native descriptions explicitly multiply m1 by combo points spent.
        ENVENOM | BLACK_POWDER | DISPATCH | EVISCERATE => combo,
        ARCANE_BARRAGE => barrage_multiplier(world, caster),
        ARCANE_BLAST => super::scripts::blast_multiplier(world, caster, 0),
        _ => return spell,
    };
    scale_effect(&mut spell, 0, multiplier);
    spell
}
