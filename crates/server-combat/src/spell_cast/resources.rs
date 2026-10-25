//! Resource-dependent values frozen at successful launch, before costs are paid.
//! Native SpellPower.OptionalCost is additional raw spend, capped by availability.
//! DurationPerResource is independent of the amount remaining at missile impact.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{CostPool, powers};
use crate::spell_info::{SpellDuration, SpellEffect, SpellInfo};

// Native Massacre281001 overrides163201 to281000; both trigger260798.
pub(in crate::spell_cast) const EXECUTE_PARENTS: &[u32] = &[163201, 281000];
const ROGUE_SPELL_FAMILY: u32 = 8;
const FEROCIOUS_BITE: u32 = 22568;
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
    scale_effect_values(effect, multiplier);
}

fn scale_effect_values(effect: &mut SpellEffect, multiplier: f32) {
    effect.base_points *= multiplier;
    effect.bonus_coefficient *= multiplier;
    effect.bonus_coefficient_from_ap *= multiplier;
}

fn is_rogue_finisher(spell: &SpellInfo) -> bool {
    spell
        .class_options
        .as_ref()
        .is_some_and(|options| options.spell_class_set == ROGUE_SPELL_FAMILY)
        && spell
            .power_costs
            .iter()
            .any(|cost| cost.power_type == PowerType::ComboPoints as i32)
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

/// SimC76f8118 sc_warrior.cpp4212–4238: full/free damage is2× the native
/// coefficient; paid casts scale by paid Rage / native base+optional maximum.
fn execute_damage_multiplier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    costs: &[(CostPool, f32)],
) -> f32 {
    let paid = spent(costs, PowerType::Rage);
    if paid == 0.0 {
        return 2.0;
    }
    let rage_costs = spell
        .power_costs
        .iter()
        .filter(|cost| cost.power_type == PowerType::Rage as i32);
    let maximum = rage_costs
        .filter(|cost| {
            cost.required_aura_spell_id == 0
                || crate::auras::has_aura(world, caster, cost.required_aura_spell_id)
        })
        .map(|cost| (cost.mana_cost + cost.optional_cost) as f32)
        .reduce(f32::max)
        .expect("Execute paid Rage without an applicable native cost row");
    assert!(
        maximum > 0.0,
        "Execute native Rage maximum must be positive"
    );
    2.0 * paid.min(maximum) / maximum
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
    let combo = super::scripts::effective_resource_value(
        world,
        caster,
        source,
        PowerType::ComboPoints,
        spent(costs, PowerType::ComboPoints),
    );
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
    // SimC76f8118 sc_rogue.cpp2320–2334 scales every direct Rogue
    // finisher by its frozen effective points, not a list of action IDs.
    if is_rogue_finisher(&spell) {
        for effect in &mut spell.effects {
            if matches!(effect.effect, 2 | 9 | 31 | 58 | 121) {
                scale_effect_values(effect, combo);
            }
        }
        return spell;
    }
    let multiplier = match spell.id {
        id if EXECUTE_PARENTS.contains(&id) => {
            let rage_multiplier = execute_damage_multiplier(world, caster, &spell, costs);
            spell.cast_damage_multiplier =
                Some(spell.cast_damage_multiplier.unwrap_or(1.0) * rage_multiplier);
            return spell;
        }
        // SimC76f8118 sc_druid.cpp4358–4371,4400–4401: Bite's coefficient
        // already represents maximum CP; free casts retain that full coefficient.
        FEROCIOUS_BITE if combo > 0.0 => {
            let (_, maximum) = powers::power_value(world, caster, PowerType::ComboPoints)
                .expect("native Bite requires a combo-point pool");
            combo / maximum
        }
        ARCANE_BARRAGE => barrage_multiplier(world, caster),
        ARCANE_BLAST => super::scripts::blast_multiplier(world, caster, 0),
        _ => return spell,
    };
    scale_effect(&mut spell, 0, multiplier);
    spell
}
