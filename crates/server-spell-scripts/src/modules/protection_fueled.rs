//! Protection's selected Fueled by Violence heals from actual bleed ticks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::PeriodicPrimaryDamage(heal_bleed)];

fn heal_bleed(world: &mut World, ctx: &EffectCtx, _: Entity, damage: f32, _: bool) {
    // SimC 76f8118 sc_warrior.cpp 2275–2283 and 3811–3822:
    // each actual Rend or Deep Wounds tick heals the selected warrior.
    if !matches!(ctx.spell.id, 388539 | 262115)
        || damage <= 0.0
        || world
            .get::<ActiveSpec>(ctx.caster)
            .is_none_or(|spec| spec.0 != 73)
        || !combat::auras::has_aura(world, ctx.caster, 383103)
    {
        return;
    }
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, 383103, 0);
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        383104,
        Some(ctx.caster),
        ctx.depth + 1,
        damage * percent / 100.0,
    );
}
