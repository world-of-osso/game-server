//! Protection MID2 4-piece: real generator impacts produce Unrelenting Edict.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraSource};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(after_hit)];

fn after_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, critical: bool, amount: f32) {
    if !matches!(ctx.spell.id, 275779 | 1241413 | 204301) || amount <= 0.0 {
        return;
    }
    let equipped = world.get::<ActiveAuras>(ctx.caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == 1296659
                && aura.caster == ctx.caster
                && matches!(aura.source, AuraSource::ItemSet(_))
        })
    });
    if !equipped {
        return;
    }
    // SimC76f8118 sc_paladin.cpp1366–1376: 20% of the actual impact,
    // doubled for a critical source; both values come from the equipped set.
    let percent = spell_cast::scripts::script_effect_value(world, ctx.caster, 1296659, 0);
    let critical_bonus = spell_cast::scripts::script_effect_value(world, ctx.caster, 1296659, 1);
    let multiplier = if critical {
        1.0 + critical_bonus / 100.0
    } else {
        1.0
    };
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        1300662,
        Some(target),
        ctx.depth + 1,
        amount * percent / 100.0 * multiplier,
    );
}
