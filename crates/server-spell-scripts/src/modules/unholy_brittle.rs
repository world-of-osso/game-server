//! Blood and Unholy disease ticks can apply the native Brittle debuff.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPeriodicDamagePolicies(tick)];

fn tick(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !matches!(ctx.spell.id, 191587 | 1240996 | 55095 | 55078)
        || !matches!(
            world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0),
            Some(250 | 252)
        )
        || !combat::auras::has_aura(world, ctx.caster, 374504)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7798–7822: each actual DK disease
    // tick rolls selected Brittle's native chance on its own target.
    let chance = world
        .resource::<SpellStore>()
        .get(374504)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Brittle proc chance")
        .proc_chance;
    if combat::random::random::<f32>() * 100.0 < chance as f32
        && !spell_cast::try_cast_triggered(world, ctx.caster, 374557, Some(target), ctx.depth + 1)
    {
        warn!(caster = ?ctx.caster, ?target, "native Brittle application refused");
    }
}
