//! TC a352b1fa Unit.cpp10174–10218 / SpellAuraEffects.cpp5369: raw resource,
//! inclusive apply check and strict-old/inclusive-new crossings; not percentages.
use bevy::prelude::*;
use shared::components::PowerType;

use super::{ActiveAuras, AuraEffectState, AuraInstance, handlers};

const GAIN: i32 = 0;
const LOSS: i32 = 1;

fn reaches_threshold(direction: i32, old: Option<f32>, new: f32, threshold: f32) -> bool {
    match direction {
        GAIN => old.is_none_or(|old| old < threshold) && new >= threshold,
        LOSS => old.is_none_or(|old| old > threshold) && new <= threshold,
        // TC's switch falls through for other directions.
        _ => true,
    }
}

fn matches_power(effect: &AuraEffectState, power: PowerType) -> bool {
    effect.aura_type == handlers::TRIGGER_SPELL_ON_POWER_AMOUNT
        && effect.misc_values[0] == power as i32
}

fn cast_children(world: &mut World, unit: Entity, children: Vec<u32>) {
    for child in children {
        let depth = crate::spell_cast::next_trigger_depth(world);
        crate::spell_cast::cast_triggered(world, unit, child, Some(unit), depth);
    }
}

pub(super) fn on_applied(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let children = aura
        .effects
        .iter()
        .filter_map(|effect| {
            if effect.aura_type != handlers::TRIGGER_SPELL_ON_POWER_AMOUNT {
                return None;
            }
            let power = PowerType::from_db(effect.misc_values[0])?;
            let (current, _) = crate::spell_cast::powers::power_value(world, unit, power)?;
            reaches_threshold(effect.misc_values[1], None, current, effect.amount)
                .then_some(effect.trigger_spell)
        })
        .collect();
    cast_children(world, unit, children);
}

pub(crate) fn on_changed(world: &mut World, unit: Entity, power: PowerType, old: f32, new: f32) {
    if old == new {
        return;
    }
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return;
    };
    let children = collect_children(auras, power, old, new);
    cast_children(world, unit, children);
}

fn collect_children(auras: &ActiveAuras, power: PowerType, old: f32, new: f32) -> Vec<u32> {
    auras
        .iter()
        .flat_map(|aura| &aura.effects)
        .filter(|effect| matches_power(effect, power))
        .filter(|effect| reaches_threshold(effect.misc_values[1], Some(old), new, effect.amount))
        .map(|effect| effect.trigger_spell)
        .collect()
}

pub(crate) fn queue_regenerated(
    commands: &mut Commands,
    unit: Entity,
    auras: Option<&ActiveAuras>,
    change: (PowerType, f32, f32),
) {
    let (power, old, new) = change;
    if old == new {
        return;
    }
    let Some(auras) = auras else {
        return;
    };
    let children = collect_children(auras, power, old, new);
    if !children.is_empty() {
        commands.queue(move |world: &mut World| cast_children(world, unit, children));
    }
}
