//! Resource-dependent values frozen at successful launch, before costs are paid.
//! Native SpellPower.OptionalCost is additional raw spend, capped by availability.
//! DurationPerResource is independent of the amount remaining at missile impact.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{CostPool, powers};
use crate::spell_info::SpellInfo;

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
            if duration.per_resource_ms != 0 {
                duration.duration_ms += (duration.per_resource_ms as f32 * combo) as i32;
                if duration.max_duration_ms > 0 {
                    duration.duration_ms = duration.duration_ms.min(duration.max_duration_ms);
                }
                duration.per_resource_ms = 0;
            }
        }
    }
    match spell.id {
        // Native descriptions explicitly multiply m1 by combo points spent.
        32645 | 319175 => scale_effect(&mut spell, 0, combo),
        365350 => {
            let (_, max) = powers::power_value(world, caster, PowerType::Mana)
                .expect("Arcane Surge requires mana");
            let mana = spent(costs, PowerType::Mana);
            let factor = super::scripts::script_effect_value(world, caster, spell.id, 1);
            let mut multiplier = factor * mana / max;
            if crate::auras::has_aura(world, caster, 457521) {
                multiplier *=
                    1.0 + super::scripts::script_effect_value(world, caster, 457521, 0) / 100.0;
            }
            scale_effect(&mut spell, 0, multiplier);
        }
        _ => {}
    }
    spell
}
