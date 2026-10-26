//! Native Ice Lance payload dispatch; retained until the shared handler cut lands.
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::SpellEffect;
use bevy::prelude::*;

pub(super) fn launch_lance_payload(
    world: &mut World,
    ctx: &EffectCtx,
    _effect: &SpellEffect,
    target: Entity,
) {
    let Some(mut payload) = spell_cast::load_triggered_spell(world, 228598, ctx.depth + 1) else {
        return;
    };
    payload.cast_launch_auras = ctx.spell.cast_launch_auras.clone();
    spell_cast::launch_triggered_spell(world, ctx.caster, &payload, Some(target), ctx.depth + 1);
}
