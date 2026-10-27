//! Ruthlessness for native self-aura finishers that have no damage impact.
use super::{combat, rogue_ruthlessness};
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterResolvedHits(after_self_finisher)];

fn after_self_finisher(world: &mut World, ctx: &EffectCtx) {
    if ctx.depth != 0 || ctx.spell.effects.is_empty() {
        return;
    }
    // SimC76f8118 sc_rogue.cpp2683–2688/4784–4810: a successful
    // self-only finisher executes once, not once for each channel hand tick.
    let self_auras_only = ctx
        .spell
        .effects
        .iter()
        .all(|effect| effect.effect == 6 && effect.implicit_targets == [1, 0]);
    if self_auras_only {
        rogue_ruthlessness::award_finisher(world, ctx);
    }
}
