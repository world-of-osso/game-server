//! SimC76f8118 sc_druid.cpp12579–12610: Nature's Guardian follows eligible heals.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::stats::CharacterStats;

const MASTERY: u32 = 155783;
const MASTERY_HEAL: u32 = 227034;
const LEECH: u32 = 143924;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedHeal(after_heal)];

fn after_heal(
    world: &mut World,
    ctx: &EffectCtx,
    _target: Entity,
    raw: f32,
    _effective: f32,
    _crit: bool,
    periodic: bool,
) {
    if raw <= 0.0
        || matches!(ctx.spell.id, MASTERY_HEAL | LEECH)
        || ctx.spell.effects.iter().any(|effect| {
            effect.effect == effects::EFFECT_HEAL_PCT
                || (periodic && effect.aura == combat::auras::handlers::OBS_MOD_HEALTH)
        })
        || !combat::auras::has_aura(world, ctx.caster, MASTERY)
    {
        return;
    }
    let mastery = world.get::<CharacterStats>(ctx.caster).expect("Guardian mastery stats").mastery_pct / 100.0;
    let spell = combat::spell_cast::load_triggered_spell(world, MASTERY_HEAL, 1)
        .expect("native Nature's Guardian heal");
    let proc_ctx = EffectCtx {
        caster: ctx.caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_calculated_heal(world, &proc_ctx, ctx.caster, raw * mastery);
}
