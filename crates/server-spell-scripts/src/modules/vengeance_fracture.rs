//! Fracture creates fragments on its two native weapon impacts.
use super::combat;
use bevy::prelude::*;
use combat::soul_fragments::{self, FragmentKind};
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(on_impact)];

fn on_impact(world: &mut World, ctx: &EffectCtx, _: Entity, _: bool, damage: f32) {
    if damage <= 0.0
        || !matches!(ctx.spell.id, 225919 | 225921)
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 581)
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp8014–8046 assigns half of the
    // native Fracture effect1 count to each hand; the main hand gets the remainder.
    let count = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, 263642, 0);
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 {
        error!(count, "invalid native Fracture fragment count");
        return;
    }
    let count = count as u32;
    let count = count / 2 + u32::from(ctx.spell.id == 225919) * (count % 2);
    for _ in 0..count {
        if let Err(reason) = soul_fragments::spawn(world, ctx.caster, FragmentKind::Lesser, false) {
            error!(%reason, "Fracture fragment spawn failed");
        }
    }
}
