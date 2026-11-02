//! Native Temple strike debuffs: DB2 spell descriptions and WCL
//! M4cdBNJkTjavYgQV/11 pulls 4 and 12 show these auras on the hit target.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterPolicyDirectDamage(
    apply_strike_debuff,
)];

fn apply_strike_debuff(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _critical: bool) {
    let debuff = match ctx.spell.id {
        1290797 => 1308838, // Merektha: Lightning Bite
        1300803 => 1303446, // Guardian: Tainted Strike
        _ => return,
    };
    if !combat::spell_cast::try_cast_triggered(
        world,
        ctx.caster,
        debuff,
        Some(target),
        ctx.depth + 1,
    ) {
        warn!(caster = ?ctx.caster, ?target, debuff, "Temple strike debuff refused");
    }
}
