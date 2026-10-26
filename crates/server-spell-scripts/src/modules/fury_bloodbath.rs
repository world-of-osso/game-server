//! Landed Bloodbath applies or extends its own native bleed on that target.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPolicyDirectDamage(after_hit)];

const BLOODBATH: u32 = 335096;
const BLEED: u32 = 113344;

fn after_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != BLOODBATH {
        return;
    }
    // SimC76f8118 sc_warrior.cpp2777–2785/2915–2923: every landed
    // Bloodbath applies its native bleed; subsequent hits extend its duration.
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == BLEED && aura.caster == ctx.caster)
    });
    if owned {
        let duration = world
            .resource::<SpellStore>()
            .get(BLEED)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.duration.as_ref())
            .expect("native Bloodbath bleed duration")
            .duration_ms;
        let duration = u32::try_from(duration).expect("positive native Bloodbath duration");
        combat::auras::extend_owned_duration(world, ctx.caster, target, &[BLEED], duration);
    } else {
        combat::spell_cast::cast_triggered(world, ctx.caster, BLEED, Some(target), ctx.depth + 1);
    }
}
