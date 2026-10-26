//! Rampaging Ruin is a selected-talent/Whirlwind trigger, not a fifth Rampage hit.
use bevy::prelude::*;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::effects::{EffectCtx, EffectHandler};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellEffect;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EffectHandler(effect_handler)];

const RAMPAGE: u32 = 184367;
const RAMPAGING_RUIN: u32 = 1265357;
const WHIRLWIND: u32 = 85739;
const RUIN_EFFECT: u8 = 5;

fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (spell == RAMPAGE && index == RUIN_EFFECT).then_some(schedule_qualified_ruin)
}

fn schedule_qualified_ruin(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    effect: &SpellEffect,
    target: Entity,
) {
    // SimC76f8118 sc_warrior.cpp5727–5748/5800–5801: only selected
    // Rampaging Ruin with live Whirlwind may schedule the modern AoE child.
    // Native70448 effect5 supplies the payload and delay; do not change either.
    let selected = world.get::<TraitLoadout>(ctx.caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == RAMPAGING_RUIN)
    });
    let whirlwind = combat::auras::has_aura(world, ctx.caster, WHIRLWIND);
    if selected && whirlwind {
        combat::spell_cast::queued_triggers::schedule(world, ctx, effect, target);
    }
}
