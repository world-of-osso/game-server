//! TC a352b1fa SpellEffects.cpp725–788: effect148 carries value to every child effect.
use super::effects::{EffectCtx, modded_base_points};
use super::{SpellInfo, effects, execute_costed_trigger, load_triggered_spell};
use crate::spell_info::SpellEffect;
use bevy::prelude::*;

fn prepare_child(world: &World, ctx: &EffectCtx<'_>, effect: &SpellEffect) -> Option<SpellInfo> {
    let mut child = load_triggered_spell(world, effect.trigger_spell, ctx.depth + 1)?;
    let value = modded_base_points(world, ctx.caster, ctx.spell, effect);
    for child_effect in &mut child.effects {
        child_effect.base_points = value;
    }
    child.cast_item_level = ctx.spell.cast_item_level;
    child.cast_original_caster = ctx.spell.cast_original_caster;
    Some(child)
}

pub(super) fn on_unit(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    effect: &SpellEffect,
    target: Entity,
) {
    let Some(child) = prepare_child(world, ctx, effect) else {
        return;
    };
    // HIT_TARGET carries its actual unit; retain native power/reagent costs.
    if let Err(error) =
        execute_costed_trigger(world, ctx.caster, &child, Some(target), ctx.depth + 1)
    {
        warn!(
            parent = ctx.spell.id,
            child = child.id,
            ?error,
            "Value missile child refused"
        );
    }
}

pub(in crate::spell_cast) fn at_destination(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    effect: &SpellEffect,
) {
    let Some(destination) = ctx.dest else {
        warn!(
            spell = ctx.spell.id,
            "Value missile child requires the parent's native destination"
        );
        return;
    };
    let Some(mut child) = prepare_child(world, ctx, effect) else {
        return;
    };
    child.cast_destination = Some(destination.position.to_array());
    if let Err(error) =
        effects::cast_destination_missile_child(world, ctx.caster, &child, ctx.depth + 1)
    {
        warn!(
            parent = ctx.spell.id,
            child = child.id,
            ?error,
            "Destination value missile child refused"
        );
    }
}
