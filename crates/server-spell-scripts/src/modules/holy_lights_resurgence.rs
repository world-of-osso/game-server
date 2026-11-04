//! VERIFIED_WCL BCX8v3VPd6pnyzM4 fight 1 pull 1: Prayer of Mending
//! 33110 and Renew 139 land together at 16.224s. DB2 70448 talent 193157
//! effect 0 gives the independent chance on each healed target.
use super::combat;
use bevy::prelude::*;
use combat::auras::has_aura;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const TALENT: u32 = 193157;
const MENDING_HEAL: u32 = 33110;
const RENEW: u32 = 139;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedHeal(apply_renew),
    SpellScriptHook::ConsumedEffectValues(&[(TALENT, 0)]),
];

fn apply_renew(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _raw: f32,
    _effective: f32,
    _critical: bool,
    _periodic: bool,
) {
    if ctx.spell.id != MENDING_HEAL || !has_aura(world, ctx.caster, TALENT) {
        return;
    }
    let chance = combat::spell_cast::scripts::script_effect_value(world, ctx.caster, TALENT, 0);
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, ctx.caster, RENEW, Some(target), ctx.depth + 1);
    }
}
